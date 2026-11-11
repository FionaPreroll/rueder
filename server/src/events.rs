//! Live updates: what changes for an account reaches all its open tabs and
//! devices at once, over one connection per tab (`GET /api/events`, server-sent
//! events).
//!
//! - **Where events come from.** Fetching says when a feed got new articles,
//!   new versions or starts and stops failing ([`Events::publish_to_readers`]).
//!   Whatever a reader changes through the API is announced without the
//!   endpoint doing anything for it: [`announce`] wraps the whole API and
//!   publishes after every request that changed something and went well.
//! - **Who gets them.** Every event names the accounts it is for, and a
//!   connection passes on only what names its own.
//! - **Nothing is lost on reconnecting.** Events are numbered. The server keeps
//!   the last [`KEPT`], and a browser that comes back says the last number it
//!   saw (`Last-Event-ID`) and gets what it missed. If that is no longer there,
//!   it is told to load everything anew (`resync`).
//!
//! Events are notices, not the data itself: they say what changed and carry
//! the few numbers that save a request. One server holds them in memory; with
//! more than one, PostgreSQL's `LISTEN/NOTIFY` would have to carry them across.

use std::{
    collections::{HashMap, VecDeque},
    convert::Infallible,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    extract::{Request, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::{
        IntoResponse, Response,
        sse::{Event as SseEvent, KeepAlive, Sse},
    },
};
use futures_util::{Stream, stream};
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::{sync::broadcast, time::Instant};
use ts_rs::TS;

use crate::{
    AppError, AppState,
    reading::{self, FeedUnread},
    sessions::{self, Session},
};

/// How many events are kept for those who reconnect.
pub const KEPT: usize = 2000;
/// How many connections one account may hold open: a few devices with a few
/// tabs each.
pub const STREAMS_PER_ACCOUNT: usize = 20;
/// How many events may wait for a connection that reads slowly. One that falls
/// further behind is told to load everything anew.
const WAITING: usize = 256;
/// After this long a connection is closed. The browser opens the next one by
/// itself and gets what it missed; the server sees again whether the session
/// still holds.
const STREAM_LASTS: Duration = Duration::from_secs(30 * 60);
/// A line that says nothing, so that proxies on the way keep the connection.
const KEEP_ALIVE: Duration = Duration::from_secs(20);
/// The header a tab names itself in, so it can tell its own changes from those
/// of other tabs when they come back as events.
const TAB: &str = "x-rueder-tab";

/// What changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// A feed got new articles.
    Articles {
        feed_id: i32,
        /// The number of the feed's newest article now.
        last_seq: i32,
        new: usize,
    },
    /// Articles of a feed were changed by their publisher and have a new version.
    Updated { feed_id: i32, count: usize },
    /// A feed cannot be fetched any more (`error` says why) or can again.
    Feed { feed_id: i32, error: Option<String> },
    /// What the account has read or starred changed.
    Reading {
        /// The feed's numbers after the change, if it was about one feed.
        feed: Option<FeedUnread>,
        /// The article, if it was about one.
        seq: Option<i32>,
        by: Option<String>,
    },
    /// Something else of the account's changed.
    Area { area: Area, by: Option<String> },
}

/// The parts of what an account has, as far as events tell them apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Area {
    /// Folders and subscriptions.
    Tree,
    Labels,
    Settings,
    /// Filter rules. They hide articles and mark them as read, so the unread
    /// counts may have changed with them.
    Rules,
    /// A session of the account ended.
    Session,
}

impl Change {
    /// The name of the event, which says what the browser has to do.
    pub fn kind(&self) -> String {
        Sent::from(self).parts().0
    }

    /// What the event carries.
    pub fn data(&self) -> Value {
        Sent::from(self).parts().1
    }
}

/// An event of `GET /api/events` as it goes over the connection: `event` is
/// its name there and `data` what it carries, as JSON.
///
/// Events are notices: one says what changed, and the browser loads what it
/// shows of that anew.
///
/// - `hello`: the connection is up, and what is loaded from now on misses
///   nothing.
/// - `resync`: the server cannot say what the browser missed; load everything
///   anew.
/// - `articles`: a feed got new articles.
/// - `updated`: articles of a feed were changed by their publisher and have a
///   new version.
/// - `feed`: a feed cannot be fetched any more (`error` says why) or can
///   again (`error` is null).
/// - `reading`: what the account has read or starred changed.
/// - `tree`: folders or subscriptions changed.
/// - `labels`, `settings`: they changed.
/// - `rules`: filter rules changed. They hide articles and mark them as read,
///   so the unread counts may have changed with them.
/// - `session`: a session of the account ended.
///
/// `by` is the name the tab that made the change gave itself in the header
/// `X-Rueder-Tab`, if it did, so that a tab knows its own change when it
/// comes back.
#[derive(Debug, Serialize, TS)]
#[serde(tag = "event", content = "data", rename_all = "lowercase")]
#[ts(rename = "ServerEvent")]
enum Sent {
    /// The connection is up, and what is loaded from now on misses nothing.
    Hello {},
    /// The server cannot say what the browser missed: load everything anew.
    Resync {},
    /// A feed got new articles.
    Articles {
        feed_id: i32,
        /// The number of the feed's newest article now.
        last_seq: i32,
        /// How many came.
        new: usize,
    },
    /// Articles of a feed were changed by their publisher and have a new version.
    Updated {
        feed_id: i32,
        count: usize,
    },
    /// A feed cannot be fetched any more (`error` says why) or can again.
    Feed {
        feed_id: i32,
        error: Option<String>,
    },
    /// What the account has read or starred changed.
    Reading {
        /// The feed's numbers after the change, if it was about one feed.
        feed: Option<FeedUnread>,
        /// The article's number in the feed, if it was about one article.
        seq: Option<i32>,
        by: Option<String>,
    },
    /// Folders or subscriptions changed.
    Tree {
        by: Option<String>,
    },
    Labels {
        by: Option<String>,
    },
    Settings {
        by: Option<String>,
    },
    /// Filter rules changed. They hide articles and mark them as read, so the
    /// unread counts may have changed with them.
    Rules {
        by: Option<String>,
    },
    /// A session of the account ended.
    Session {
        by: Option<String>,
    },
}

impl From<&Change> for Sent {
    fn from(change: &Change) -> Self {
        match change.clone() {
            Change::Articles {
                feed_id,
                last_seq,
                new,
            } => Self::Articles {
                feed_id,
                last_seq,
                new,
            },
            Change::Updated { feed_id, count } => Self::Updated { feed_id, count },
            Change::Feed { feed_id, error } => Self::Feed { feed_id, error },
            Change::Reading { feed, seq, by } => Self::Reading { feed, seq, by },
            Change::Area { area, by } => match area {
                Area::Tree => Self::Tree { by },
                Area::Labels => Self::Labels { by },
                Area::Settings => Self::Settings { by },
                Area::Rules => Self::Rules { by },
                Area::Session => Self::Session { by },
            },
        }
    }
}

impl Sent {
    /// The name and what it carries, as they are written to the connection.
    fn parts(&self) -> (String, Value) {
        let Ok(Value::Object(mut whole)) = serde_json::to_value(self) else {
            return (String::new(), json!({}));
        };
        let kind = match whole.remove("event") {
            Some(Value::String(kind)) => kind,
            _ => String::new(),
        };
        (kind, whole.remove("data").unwrap_or_else(|| json!({})))
    }
}

/// The types of the events, for the frontend.
pub(crate) fn types(types: &mut crate::typescript::Types) {
    types.add::<Sent>();
}

/// An event as it is kept and passed on.
#[derive(Debug)]
struct Event {
    number: u64,
    /// The accounts it is for.
    accounts: Vec<i32>,
    change: Change,
}

struct Kept {
    /// The number of the newest event, 0 before the first.
    newest: u64,
    events: VecDeque<Arc<Event>>,
    /// How many connections each account holds.
    streams: HashMap<i32, usize>,
}

struct Inner {
    /// Tells this run of the server from the one before: numbers start anew
    /// with every start, and an old one must not be taken for a new one.
    run: String,
    kept: Mutex<Kept>,
    sender: broadcast::Sender<Arc<Event>>,
}

/// Takes events and hands them to the connections. Cheap to clone; all clones
/// are the same.
#[derive(Clone)]
pub struct Events {
    inner: Arc<Inner>,
}

impl Default for Events {
    fn default() -> Self {
        Self::new()
    }
}

/// What a new connection starts with.
struct Opened {
    /// What it gets at once: what it missed, or the word that it has to load
    /// everything anew, or just a greeting with the number events stand at.
    first: VecDeque<SseEvent>,
    /// The number of the last event it has seen.
    seen: u64,
    receiver: broadcast::Receiver<Arc<Event>>,
    closing: Closing,
}

/// Gives the account its place back when the connection goes.
struct Closing {
    events: Events,
    account_id: i32,
}

impl Drop for Closing {
    fn drop(&mut self) {
        if let Ok(mut kept) = self.events.inner.kept.lock()
            && let Some(streams) = kept.streams.get_mut(&self.account_id)
        {
            *streams = streams.saturating_sub(1);
            if *streams == 0 {
                kept.streams.remove(&self.account_id);
            }
        }
    }
}

impl Events {
    pub fn new() -> Self {
        let mut run = [0_u8; 6];
        // Without random numbers the time will do: it only has to differ from
        // the run before.
        if getrandom::fill(&mut run).is_err() {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.as_nanos());
            run.copy_from_slice(&now.to_le_bytes()[..6]);
        }
        let (sender, _) = broadcast::channel(WAITING);
        Self {
            inner: Arc::new(Inner {
                run: run.iter().map(|byte| format!("{byte:02x}")).collect(),
                kept: Mutex::new(Kept {
                    newest: 0,
                    events: VecDeque::new(),
                    streams: HashMap::new(),
                }),
                sender,
            }),
        }
    }

    /// Tells the accounts' open connections about a change.
    pub fn publish(&self, accounts: Vec<i32>, change: Change) {
        if accounts.is_empty() {
            return;
        }
        let Ok(mut kept) = self.inner.kept.lock() else {
            return;
        };
        kept.newest += 1;
        let event = Arc::new(Event {
            number: kept.newest,
            accounts,
            change,
        });
        kept.events.push_back(event.clone());
        if kept.events.len() > KEPT {
            kept.events.pop_front();
        }
        // Sent while the lock is held, so numbers arrive in order. Nobody
        // listening is no error.
        let _ = self.inner.sender.send(event);
    }

    /// Tells everybody who subscribes to the feed about a change of it.
    pub async fn publish_to_readers(
        &self,
        db: &PgPool,
        feed_id: i32,
        change: Change,
    ) -> anyhow::Result<()> {
        let readers = sqlx::query_scalar!(
            "SELECT account_id FROM subscriptions WHERE feed_id = $1",
            feed_id
        )
        .fetch_all(db)
        .await?;
        self.publish(readers, change);
        Ok(())
    }

    /// The id an event goes out with: this run of the server and the number.
    fn id(&self, number: u64) -> String {
        format!("{}-{number}", self.inner.run)
    }

    fn sse(&self, number: u64, sent: &Sent) -> SseEvent {
        let (kind, data) = sent.parts();
        SseEvent::default()
            .id(self.id(number))
            .event(kind)
            .data(data.to_string())
    }

    /// Opens a connection for an account. `last_seen` is the id of the last
    /// event the browser got on its connection before, if it had one. `None`
    /// if the account holds as many connections as it may.
    fn open(&self, account_id: i32, last_seen: Option<&str>) -> Option<Opened> {
        let mut kept = self.inner.kept.lock().ok()?;
        let streams = kept.streams.entry(account_id).or_default();
        if *streams >= STREAMS_PER_ACCOUNT {
            return None;
        }
        *streams += 1;
        let closing = Closing {
            events: self.clone(),
            account_id,
        };
        // Listening starts under the lock: nothing is published between
        // looking at what is kept and hearing what comes.
        let receiver = self.inner.sender.subscribe();
        let newest = kept.newest;
        let seen = last_seen.and_then(|id| {
            let (run, number) = id.rsplit_once('-')?;
            let number: u64 = number.parse().ok()?;
            // Everything after it must still be there.
            let oldest = kept.events.front().map_or(newest + 1, |event| event.number);
            (run == self.inner.run && number <= newest && number + 1 >= oldest).then_some(number)
        });
        let first = match (last_seen, seen) {
            (None, _) => VecDeque::from([self.sse(newest, &Sent::Hello {})]),
            (Some(_), None) => VecDeque::from([self.sse(newest, &Sent::Resync {})]),
            (Some(_), Some(seen)) => kept
                .events
                .iter()
                .filter(|event| event.number > seen && event.accounts.contains(&account_id))
                .map(|event| self.sse(event.number, &Sent::from(&event.change)))
                .collect(),
        };
        Some(Opened {
            first,
            seen: newest,
            receiver,
            closing,
        })
    }

    /// The number of the newest event.
    fn newest(&self) -> u64 {
        self.inner.kept.lock().map_or(0, |kept| kept.newest)
    }

    /// How many connections the account holds open.
    pub fn streams(&self, account_id: i32) -> usize {
        let kept = self.inner.kept.lock();
        kept.map_or(0, |kept| {
            kept.streams.get(&account_id).copied().unwrap_or(0)
        })
    }
}

/// A connection that is being served.
struct Serving {
    events: Events,
    db: PgPool,
    account_id: i32,
    /// The session's token, to see whether the session still holds.
    token: String,
    first: VecDeque<SseEvent>,
    seen: u64,
    receiver: broadcast::Receiver<Arc<Event>>,
    ends: Instant,
    _closing: Closing,
}

impl Serving {
    /// The next event for the browser, or `None` when the connection is over.
    async fn next(&mut self) -> Option<SseEvent> {
        if let Some(event) = self.first.pop_front() {
            return Some(event);
        }
        loop {
            let received = tokio::time::timeout_at(self.ends, self.receiver.recv()).await;
            let event = match received {
                // Time is up.
                Err(_) => return None,
                Ok(Err(broadcast::error::RecvError::Closed)) => return None,
                // More came than this connection took: it cannot know what it
                // missed.
                Ok(Err(broadcast::error::RecvError::Lagged(_))) => {
                    self.seen = self.events.newest();
                    return Some(self.events.sse(self.seen, &Sent::Resync {}));
                }
                Ok(Ok(event)) => event,
            };
            if event.number <= self.seen || !event.accounts.contains(&self.account_id) {
                continue;
            }
            self.seen = event.number;
            // A session of the account ended. If it was this connection's, the
            // connection ends with it.
            let session_ended = matches!(
                event.change,
                Change::Area {
                    area: Area::Session,
                    ..
                }
            );
            if session_ended && !matches!(sessions::find(&self.db, &self.token).await, Ok(Some(_)))
            {
                return None;
            }
            return Some(self.events.sse(event.number, &Sent::from(&event.change)));
        }
    }
}

/// `GET /api/events`: the account's events as they happen.
pub async fn stream(
    State(state): State<AppState>,
    session: Session,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let last_seen = headers.get("last-event-id").and_then(|id| id.to_str().ok());
    let Some(opened) = state.events.open(session.account_id, last_seen) else {
        return Err(AppError::Status(
            StatusCode::TOO_MANY_REQUESTS,
            "too many open connections for this account".into(),
        ));
    };
    let serving = Serving {
        events: state.events.clone(),
        db: state.db.clone(),
        account_id: session.account_id,
        token: sessions::token(&headers).unwrap_or_default().to_owned(),
        first: opened.first,
        seen: opened.seen,
        receiver: opened.receiver,
        ends: Instant::now() + STREAM_LASTS,
        _closing: opened.closing,
    };
    let events = stream::unfold(serving, |mut serving| async move {
        let event = serving.next().await?;
        Some((Ok::<_, Infallible>(event), serving))
    });
    Ok(sse(events))
}

fn sse(events: impl Stream<Item = Result<SseEvent, Infallible>> + Send + 'static) -> Response {
    let mut response = Sse::new(events)
        .keep_alive(KeepAlive::new().interval(KEEP_ALIVE))
        .into_response();
    // A reverse proxy must pass events on as they come, not in batches.
    response.headers_mut().insert(
        HeaderName::from_static("x-accel-buffering"),
        HeaderValue::from_static("no"),
    );
    response
}

/// Wraps the API: when a request that changes something went well, the
/// account's other tabs and devices hear of it.
///
/// What they hear is told from the address alone (see [`changed`]), so an
/// endpoint needs no code for it, and one that is added later is announced
/// too. A tab can name itself in the header `X-Rueder-Tab`; the name comes back
/// in the event as `by`, so the tab knows its own change when it sees it.
pub async fn announce(State(state): State<AppState>, mut request: Request, next: Next) -> Response {
    let reads_only = matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    );
    if reads_only {
        return next.run(request).await;
    }
    let path = request.uri().path().to_owned();
    let by = request
        .headers()
        .get(TAB)
        .and_then(|tab| tab.to_str().ok())
        .filter(|tab| {
            let plain = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
            !tab.is_empty() && tab.len() <= 64 && tab.chars().all(plain)
        })
        .map(str::to_owned);
    // Who asks is found out here, once, and handed on to the endpoint.
    let session = match sessions::token(request.headers()) {
        Some(token) => sessions::find(&state.db, token).await.ok().flatten(),
        None => None,
    };
    if let Some(session) = &session {
        request.extensions_mut().insert(session.clone());
    }
    let response = next.run(request).await;
    if response.status().is_success()
        && let Some(session) = session
        && let Some(what) = changed(&path)
    {
        let change = match what {
            Changed::Area(area) => Change::Area { area, by },
            Changed::Reading { feed_id, seq } => {
                let feed = match feed_id {
                    Some(feed_id) => reading::unread_of(&state.db, session.account_id, feed_id)
                        .await
                        .ok()
                        .flatten(),
                    None => None,
                };
                Change::Reading { feed, seq, by }
            }
        };
        state.events.publish(vec![session.account_id], change);
    }
    response
}

/// What a request to an address of the API changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Changed {
    Area(Area),
    Reading {
        feed_id: Option<i32>,
        seq: Option<i32>,
    },
}

/// What a request that is not a mere question changes, by its address, or
/// `None` if it changes nothing others should hear of. The address is the one
/// under `/api`, with or without that prefix.
///
/// Whatever is not known here counts as a change of the tree, which makes
/// browsers load folders, subscriptions and counts anew: too much is safe, too
/// little would leave a tab showing what is no longer so.
pub fn changed(path: &str) -> Option<Changed> {
    let path = path.strip_prefix("/api").unwrap_or(path);
    let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    let number = |part: &str| part.parse::<i32>().ok();
    Some(match parts.as_slice() {
        // Logging in changes nothing for those who already are.
        ["login"] | ["oidc", ..] => return None,
        // Questions that are sent like changes: what a rule would do, and
        // which feeds a page has.
        ["rules", "preview"] | ["feeds", "find"] => return None,
        // An admin's list of accounts is nothing a reader's tabs show.
        ["accounts", ..] => return None,
        // A session ended: by logging out, by ending it from another device,
        // or with a new password, which ends them all.
        ["logout"] | ["sessions", ..] | ["password"] => Changed::Area(Area::Session),
        ["settings", ..] => Changed::Area(Area::Settings),
        ["rules", ..] => Changed::Area(Area::Rules),
        ["labels", ..] | ["feeds", _, "articles", _, "labels", ..] => Changed::Area(Area::Labels),
        ["read"] | ["folders", _, "read"] => Changed::Reading {
            feed_id: None,
            seq: None,
        },
        ["feeds", feed, "read"] => Changed::Reading {
            feed_id: number(feed),
            seq: None,
        },
        ["feeds", feed, "articles", seq, "state"] => Changed::Reading {
            feed_id: number(feed),
            seq: number(seq),
        },
        _ => Changed::Area(Area::Tree),
    })
}
