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
//! - **After a break.** Every connection begins with `hello`. A browser that
//!   comes back after a break loads what it shows anew; the server keeps
//!   nothing for it.
//!
//! Events are notices, not the data itself: they say what changed and carry
//! the few numbers that save a request. One server holds them in memory; with
//! more than one, PostgreSQL's `LISTEN/NOTIFY` would have to carry them across.

use std::{
    collections::HashMap,
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
use futures_util::{Stream, StreamExt, stream};
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::{sync::broadcast, time::Instant};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::{
    AppError, AppState,
    reading::{self, FeedUnread},
    sessions::{self, Session},
};

/// How many connections one account may hold open: a few devices with a few
/// tabs each.
pub const STREAMS_PER_ACCOUNT: usize = 20;
/// How many events may wait for a connection that reads slowly. One that falls
/// further behind is told to load everything anew.
const WAITING: usize = 256;
/// How often a connection sees whether its session still holds. A session
/// that ends through a request closes its connections at once (see
/// [`ServerEvent::Session`]); one that runs out or is ended on the command
/// line, also by disabling or removing its account there, closes them within
/// this time.
pub const SESSION_CHECK: Duration = Duration::from_secs(60);
/// A line that says nothing, so that proxies on the way keep the connection.
const KEEP_ALIVE: Duration = Duration::from_secs(20);
/// The header a tab names itself in, so it can tell its own changes from those
/// of other tabs when they come back as events.
const TAB: &str = "x-rueder-tab";

/// An event of `GET /api/events` as it goes over the connection: `event` is
/// its name there and `data` what it carries, as JSON.
///
/// Events are notices: one says what changed, and the browser loads what it
/// shows of that anew.
///
/// - `hello`: the connection is up, and what is loaded from now on misses
///   nothing.
/// - `resync`: the connection fell behind and missed events; load everything
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
/// - `apps`: app passwords were made or revoked, or an app logged in with one
///   for the first time.
///
/// `by` is the name the tab that made the change gave itself in the header
/// `X-Rueder-Tab`, if it did, so that a tab knows its own change when it
/// comes back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS, ToSchema)]
#[serde(tag = "event", content = "data", rename_all = "lowercase")]
pub enum ServerEvent {
    Hello {},
    Resync {},
    Articles {
        feed_id: i32,
        /// The number of the feed's newest article now.
        last_seq: i32,
        /// How many came.
        new: usize,
    },
    Updated {
        feed_id: i32,
        count: usize,
    },
    Feed {
        feed_id: i32,
        error: Option<String>,
    },
    Reading {
        /// The feed's numbers after the change, if it was about one feed.
        feed: Option<FeedUnread>,
        /// The article's number in the feed, if it was about one article.
        seq: Option<i32>,
        by: Option<String>,
    },
    Tree {
        by: Option<String>,
    },
    Labels {
        by: Option<String>,
    },
    Settings {
        by: Option<String>,
    },
    Rules {
        by: Option<String>,
    },
    Session {
        by: Option<String>,
    },
    Apps {
        by: Option<String>,
    },
}

impl ServerEvent {
    /// The event as it is written to the connection: its name and what it
    /// carries.
    fn sse(&self) -> SseEvent {
        let Ok(Value::Object(mut whole)) = serde_json::to_value(self) else {
            return SseEvent::default();
        };
        let kind = match whole.remove("event") {
            Some(Value::String(kind)) => kind,
            _ => String::new(),
        };
        let data = whole.remove("data").unwrap_or_else(|| json!({}));
        SseEvent::default().event(kind).data(data.to_string())
    }
}

/// The types of the events, for the frontend.
pub(crate) fn types(types: &mut crate::typescript::Types) {
    types.add::<ServerEvent>();
}

/// An event on its way, with the accounts it is for.
#[derive(Debug)]
struct Notice {
    accounts: Vec<i32>,
    event: ServerEvent,
}

struct Inner {
    sender: broadcast::Sender<Arc<Notice>>,
    /// How many connections each account holds.
    streams: Mutex<HashMap<i32, usize>>,
    /// See [`SESSION_CHECK`].
    session_check: Duration,
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

/// Gives the account its place back when the connection goes.
struct Closing {
    events: Events,
    account_id: i32,
}

impl Drop for Closing {
    fn drop(&mut self) {
        if let Ok(mut streams) = self.events.inner.streams.lock()
            && let Some(count) = streams.get_mut(&self.account_id)
        {
            *count = count.saturating_sub(1);
            if *count == 0 {
                streams.remove(&self.account_id);
            }
        }
    }
}

impl Events {
    pub fn new() -> Self {
        Self::checking_sessions_every(SESSION_CHECK)
    }

    /// Events whose connections see this often whether their session still
    /// holds, in place of [`SESSION_CHECK`].
    pub fn checking_sessions_every(session_check: Duration) -> Self {
        let (sender, _) = broadcast::channel(WAITING);
        Self {
            inner: Arc::new(Inner {
                sender,
                streams: Mutex::new(HashMap::new()),
                session_check,
            }),
        }
    }

    /// Tells the accounts' open connections about a change.
    pub fn publish(&self, accounts: Vec<i32>, event: ServerEvent) {
        if accounts.is_empty() {
            return;
        }
        // Nobody listening is no error.
        let _ = self.inner.sender.send(Arc::new(Notice { accounts, event }));
    }

    /// Tells everybody who subscribes to the feed about a change of it.
    pub async fn publish_to_readers(
        &self,
        db: &PgPool,
        feed_id: i32,
        event: ServerEvent,
    ) -> anyhow::Result<()> {
        let readers = sqlx::query_scalar!(
            "SELECT account_id FROM subscriptions WHERE feed_id = $1",
            feed_id
        )
        .fetch_all(db)
        .await?;
        self.publish(readers, event);
        Ok(())
    }

    /// Takes a place for a connection of the account, or `None` if it holds
    /// as many as it may.
    fn open(&self, account_id: i32) -> Option<Closing> {
        let mut streams = self.inner.streams.lock().ok()?;
        let count = streams.entry(account_id).or_default();
        if *count >= STREAMS_PER_ACCOUNT {
            return None;
        }
        *count += 1;
        Some(Closing {
            events: self.clone(),
            account_id,
        })
    }

    /// How many connections are open, of all accounts.
    pub fn all_streams(&self) -> usize {
        let streams = self.inner.streams.lock();
        streams.map_or(0, |streams| streams.values().sum())
    }

    /// How many connections the account holds open.
    pub fn streams(&self, account_id: i32) -> usize {
        let streams = self.inner.streams.lock();
        streams.map_or(0, |streams| streams.get(&account_id).copied().unwrap_or(0))
    }
}

/// A connection that is being served.
struct Serving {
    events: Events,
    db: PgPool,
    account_id: i32,
    /// The session's token, to see whether the session still holds.
    token: String,
    receiver: broadcast::Receiver<Arc<Notice>>,
    /// When it sees next whether its session still holds.
    check: Instant,
    _closing: Closing,
}

impl Serving {
    /// The next event for the browser, or `None` when the connection is over.
    async fn next(&mut self) -> Option<SseEvent> {
        loop {
            let received = tokio::time::timeout_at(self.check, self.receiver.recv()).await;
            let notice = match received {
                // Time to see whether the session still holds.
                Err(_) => {
                    if !self.session_holds().await {
                        return None;
                    }
                    continue;
                }
                Ok(Err(broadcast::error::RecvError::Closed)) => return None,
                // More came than this connection took: it cannot know what it
                // missed. It goes on with what comes from now on.
                Ok(Err(broadcast::error::RecvError::Lagged(_))) => {
                    self.receiver = self.receiver.resubscribe();
                    return Some(ServerEvent::Resync {}.sse());
                }
                Ok(Ok(notice)) => notice,
            };
            if !notice.accounts.contains(&self.account_id) {
                continue;
            }
            // A session of the account ended. If it was this connection's, the
            // connection ends with it.
            let session_ended = matches!(notice.event, ServerEvent::Session { .. });
            if session_ended && !self.session_holds().await {
                return None;
            }
            return Some(notice.event.sse());
        }
    }

    /// Whether the connection's session is still there, and when to look
    /// again. One that cannot be found, or not now, ends the connection; the
    /// browser opens the next one by itself if it may.
    async fn session_holds(&mut self) -> bool {
        self.check = Instant::now() + self.events.inner.session_check;
        matches!(sessions::find(&self.db, &self.token).await, Ok(Some(_)))
    }
}

/// `GET /api/events`: the account's events as they happen.
///
/// A stream of server-sent events, one for each change of the account,
/// whichever tab, app or fetch made it. It begins with `hello`. An event names
/// what changed; the page then asks the API for it again. Events missed while
/// the connection was away are not sent again: a browser that comes back
/// loads what it shows anew.
#[utoipa::path(
    get,
    path = "/events",
    tag = "Live updates",
    security(("session" = [])),
    responses(
        (status = OK, description = "The events, as `text/event-stream`, until the session ends.", content_type = "text/event-stream"),
        crate::openapi::NeedsSession,
        (status = TOO_MANY_REQUESTS, description = "Too many open streams for this account."),
    ),
)]
pub async fn stream(
    State(state): State<AppState>,
    session: Session,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let Some(closing) = state.events.open(session.account_id) else {
        return Err(AppError::Status(
            StatusCode::TOO_MANY_REQUESTS,
            "too many open connections for this account".into(),
        ));
    };
    // Listening starts before the greeting: what the browser loads after it
    // misses nothing.
    let serving = Serving {
        events: state.events.clone(),
        db: state.db.clone(),
        account_id: session.account_id,
        token: sessions::token(&headers).unwrap_or_default().to_owned(),
        receiver: state.events.inner.sender.subscribe(),
        check: Instant::now() + state.events.inner.session_check,
        _closing: closing,
    };
    let hello = stream::once(async { Ok::<_, Infallible>(ServerEvent::Hello {}.sse()) });
    let events = stream::unfold(serving, |mut serving| async move {
        let event = serving.next().await?;
        Some((Ok::<_, Infallible>(event), serving))
    });
    Ok(sse(hello.chain(events)))
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
pub async fn announce(State(state): State<AppState>, request: Request, next: Next) -> Response {
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
    // Found before the request, which may end the session, as logging out does.
    let session = match sessions::token(request.headers()) {
        Some(token) => sessions::find(&state.db, token).await.ok().flatten(),
        None => None,
    };
    let response = next.run(request).await;
    if response.status().is_success()
        && let Some(session) = session
        && let Some(what) = changed(&path)
    {
        publish(&state, session.account_id, what, by).await;
    }
    response
}

/// Tells the account's open tabs what a request changed. `by` is the tab
/// that asked, if it named itself.
pub async fn publish(state: &AppState, account_id: i32, what: Changed, by: Option<String>) {
    let tell = |event| state.events.publish(vec![account_id], event);
    let (feed_id, seq) = match what {
        Changed::Tree => return tell(ServerEvent::Tree { by }),
        Changed::Labels => return tell(ServerEvent::Labels { by }),
        Changed::Settings => return tell(ServerEvent::Settings { by }),
        Changed::Rules => return tell(ServerEvent::Rules { by }),
        Changed::Session => return tell(ServerEvent::Session { by }),
        Changed::Apps => return tell(ServerEvent::Apps { by }),
        Changed::Everything => {
            tell(ServerEvent::Labels { by: by.clone() });
            tell(ServerEvent::Settings { by: by.clone() });
            tell(ServerEvent::Rules { by: by.clone() });
            return tell(ServerEvent::Tree { by });
        }
        Changed::Names => {
            tell(ServerEvent::Tree { by: by.clone() });
            return tell(ServerEvent::Labels { by });
        }
        Changed::Articles => {
            tell(ServerEvent::Labels { by: by.clone() });
            (None, None)
        }
        Changed::Reading { feed_id, seq } => (feed_id, seq),
    };
    let feed = match feed_id {
        Some(feed_id) => reading::unread_of(&state.db, account_id, feed_id)
            .await
            .ok()
            .flatten(),
        None => None,
    };
    tell(ServerEvent::Reading { feed, seq, by });
}

/// What a request to an address of the API changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Changed {
    /// Folders and subscriptions.
    Tree,
    Labels,
    Settings,
    /// Filter rules, and with them maybe the unread counts.
    Rules,
    /// A session of the account ended, or what its sessions may changed.
    Session,
    /// App passwords.
    Apps,
    /// Everything an account has: a move from another rueder brings folders,
    /// feeds, what was read, labels, rules and settings at once.
    Everything,
    Reading {
        feed_id: Option<i32>,
        seq: Option<i32>,
    },
    /// What was read, starred and labelled, in several feeds at once: what an
    /// app sends.
    Articles,
    /// The names of folders and labels, which an app tells apart only by
    /// what they hold.
    Names,
}

/// What a request that is not a mere question changes, by its address, or
/// `None` if it changes nothing others should hear of. The address is the one
/// under `/api`, with or without that prefix.
///
/// Whatever is not known here counts as a change of the tree, which makes
/// browsers load folders, subscriptions and counts anew: too much is safe, too
/// little would leave a tab showing what is no longer so.
pub fn changed(path: &str) -> Option<Changed> {
    if let Some(path) = path.strip_prefix("/greader") {
        return changed_by_app(path);
    }
    let path = path.strip_prefix("/api").unwrap_or(path);
    let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    let number = |part: &str| part.parse::<i32>().ok();
    Some(match parts.as_slice() {
        // Logging in changes nothing for those who already are.
        ["login", ..] | ["oidc", ..] => return None,
        // Nor does making an account, and invitations are shown where they
        // are made.
        ["register"] | ["invites", ..] => return None,
        // A link that is opened or sent again has no session to announce it
        // by; opening it tells the account's tabs itself (see `email`).
        ["email", "confirm" | "resend"] => return None,
        // Asking changes nothing yet; setting the password publishes for itself,
        // as nobody is logged in to it.
        ["password", "forgot" | "reset"] => return None,
        // A new address waits for its link, or one goes: `me` says it.
        ["email"] => Changed::Session,
        // Questions that are sent like changes: what a rule would do, and
        // which feeds a page has.
        ["rules", "preview"] | ["feeds", "find"] => return None,
        // An admin's list of accounts is nothing a reader's tabs show.
        ["accounts", ..] => return None,
        // A session ended: by logging out, by ending it from another device,
        // with a new password, which ends them all, or with the account,
        // which ends them all for good.
        ["logout"] | ["sessions", ..] | ["password"] | ["account"] => Changed::Session,
        // A secret that waits for its code changes nothing yet; switching the
        // second factor on or off changes what the sessions may.
        ["second-factor", "setup"] => return None,
        // Starting to add a passkey or to show it is the owner changes
        // nothing yet; a passkey that comes or goes changes what the
        // sessions may, as the second factor does.
        ["passkeys", "register" | "proof"] => return None,
        ["passkeys", ..] => Changed::Session,
        ["second-factor", ..] => Changed::Session,
        // Agreeing to the terms ends the question in the other tabs.
        ["terms"] => Changed::Session,
        ["app-passwords", ..] => Changed::Apps,
        // A report changes nothing a reader's tabs show.
        ["feeds", _, "report"] | ["feeds", _, "articles", _, "report"] => return None,
        ["settings", ..] => Changed::Settings,
        // Whether the account sends reports of errors is one of its settings;
        // the reports themselves are nothing its tabs show.
        ["reports", "consent"] => Changed::Settings,
        ["reports", ..] => return None,
        ["rules", ..] => Changed::Rules,
        ["import"] => Changed::Everything,
        ["labels", ..] | ["feeds", _, "articles", _, "labels", ..] => Changed::Labels,
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
        _ => Changed::Tree,
    })
}

/// What a request of an app changes, by its address under `/greader` (see
/// `greader`). Unlike the page's API, whatever is not known here changes
/// nothing: the API answers every address it does not have with `[]`, and a
/// new endpoint that changes something gets its line here.
fn changed_by_app(path: &str) -> Option<Changed> {
    let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    // Logging in and images change nothing.
    let ["reader", "api", "0", rest @ ..] = parts.as_slice() else {
        return None;
    };
    Some(match rest {
        ["edit-tag"] => Changed::Articles,
        ["mark-all-as-read"] => Changed::Reading {
            feed_id: None,
            seq: None,
        },
        ["rename-tag"] | ["disable-tag"] => Changed::Names,
        ["subscription", "edit" | "quickadd"] => Changed::Tree,
        // Everything else only reads, however it is sent, or is answered
        // `[]` by the API without changing anything: an app asks for its
        // lists every few minutes, and the tabs need not load anything anew
        // for it.
        _ => return None,
    })
}
