//! Live updates: who hears of what, what a browser gets that comes back after
//! a break, and what fetching and the API tell.

mod common;

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode, header},
    routing::get,
};
use http_body_util::BodyExt;
use rueder::{
    AppState,
    cleaning::Rules,
    events::{Area, Change, Changed, Events, KEPT, STREAMS_PER_ACCOUNT, changed},
    feeds::Stored,
    fetch::{self, Outcome},
    http_client::{Config, HttpClient},
    reading::FeedUnread,
    router,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

/// An event as a browser gets it.
#[derive(Debug, Clone, PartialEq)]
struct Sent {
    id: String,
    kind: String,
    data: Value,
}

impl Sent {
    /// The number in the id, which counts up.
    fn number(&self) -> u64 {
        self.id.rsplit_once('-').unwrap().1.parse().unwrap()
    }
}

/// An open connection to `/api/events`.
struct Stream {
    body: Body,
    unread: String,
}

impl Stream {
    /// The next event, or `None` if none comes for a while or the connection
    /// has ended.
    async fn next(&mut self) -> Option<Sent> {
        self.next_within(Duration::from_secs(5)).await
    }

    async fn next_within(&mut self, patience: Duration) -> Option<Sent> {
        loop {
            if let Some((event, rest)) = self.unread.split_once("\n\n") {
                let (event, rest) = (event.to_owned(), rest.to_owned());
                self.unread = rest;
                let field = |name: &str| {
                    event
                        .lines()
                        .find_map(|line| line.strip_prefix(name)?.strip_prefix(':'))
                        .map(|value| value.trim().to_owned())
                };
                // Lines that only keep the connection open say nothing.
                let Some(kind) = field("event") else { continue };
                return Some(Sent {
                    id: field("id").unwrap(),
                    kind,
                    data: serde_json::from_str(&field("data").unwrap()).unwrap(),
                });
            }
            let frame = tokio::time::timeout(patience, self.body.frame()).await;
            let bytes = frame.ok()??.ok()?.into_data().ok()?;
            self.unread.push_str(std::str::from_utf8(&bytes).unwrap());
        }
    }

    /// Whether nothing arrives.
    async fn is_quiet(&mut self) -> bool {
        self.next_within(Duration::from_millis(300)).await.is_none()
    }

    /// Whether the server has closed the connection.
    async fn has_ended(&mut self) -> bool {
        let frame = tokio::time::timeout(Duration::from_secs(5), self.body.frame()).await;
        matches!(frame, Ok(None))
    }
}

/// Opens a connection as the browser with this cookie, which saw `last` on
/// its connection before.
async fn open(state: &AppState, cookie: Option<&str>, last: Option<&str>) -> (StatusCode, Stream) {
    let mut request = Request::get("/api/events");
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    if let Some(last) = last {
        request = request.header("last-event-id", last);
    }
    let app = router(state.clone(), "missing".as_ref());
    let response = app
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    if response.status() == StatusCode::OK {
        let said = |name: &str| response.headers().get(name).unwrap().to_str().unwrap();
        assert_eq!(said("content-type"), "text/event-stream");
        assert_eq!(said("cache-control"), "no-cache");
        assert_eq!(said("x-accel-buffering"), "no");
    }
    let status = response.status();
    let stream = Stream {
        body: response.into_body(),
        unread: String::new(),
    };
    (status, stream)
}

/// A connection that has been greeted and is waiting for events.
async fn listening(state: &AppState, cookie: &str) -> Stream {
    let (_, mut stream) = open(state, Some(cookie), None).await;
    let hello = stream.next().await.expect("a greeting");
    assert_eq!((hello.kind.as_str(), &hello.data), ("hello", &json!({})));
    stream
}

/// Sends a request that changes something, as a tab that names itself.
async fn send(
    state: &AppState,
    method: Method,
    path: &str,
    cookie: &str,
    tab: Option<&str>,
    body: Value,
) -> StatusCode {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(tab) = tab {
        request = request.header("x-rueder-tab", tab);
    }
    let app = router(state.clone(), "missing".as_ref());
    let response = app
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    response.status()
}

/// Whether nobody is told to do anything: what `changed` says for a request
/// that only asks.
#[test]
fn what_a_request_changes_is_told_by_its_address() {
    let reading = |feed_id, seq| Some(Changed::Reading { feed_id, seq });
    let area = |area| Some(Changed::Area(area));
    for (path, what) in [
        ("/api/feeds/7/articles/3/state", reading(Some(7), Some(3))),
        ("/feeds/7/articles/3/state", reading(Some(7), Some(3))),
        ("/api/feeds/7/read", reading(Some(7), None)),
        ("/api/folders/2/read", reading(None, None)),
        ("/api/read", reading(None, None)),
        ("/api/feeds/7/articles/3/labels/1", area(Area::Labels)),
        ("/api/labels", area(Area::Labels)),
        ("/api/labels/order", area(Area::Labels)),
        ("/api/labels/4", area(Area::Labels)),
        ("/api/settings", area(Area::Settings)),
        ("/api/reports/consent", area(Area::Settings)),
        ("/api/rules", area(Area::Rules)),
        ("/api/rules/3", area(Area::Rules)),
        ("/api/rules/order", area(Area::Rules)),
        ("/api/logout", area(Area::Session)),
        ("/api/sessions", area(Area::Session)),
        ("/api/sessions/3", area(Area::Session)),
        ("/api/password", area(Area::Session)),
        ("/api/account", area(Area::Session)),
        ("/api/second-factor", area(Area::Session)),
        ("/api/second-factor/confirm", area(Area::Session)),
        ("/api/terms", area(Area::Session)),
        ("/api/tree/move", area(Area::Tree)),
        ("/api/folders", area(Area::Tree)),
        ("/api/folders/2", area(Area::Tree)),
        ("/api/feeds/7", area(Area::Tree)),
        ("/api/feeds/7/agent", area(Area::Tree)),
        ("/api/feeds", area(Area::Tree)),
        ("/api/opml", area(Area::Tree)),
        ("/api/import", Some(Changed::Everything)),
        // What nobody has thought of yet makes browsers load the tree anew.
        ("/api/something/new", area(Area::Tree)),
        // Questions that are sent like changes, and what only admins see.
        ("/api/rules/preview", None),
        ("/api/second-factor/setup", None),
        ("/api/feeds/7/report", None),
        ("/api/feeds/7/articles/3/report", None),
        ("/api/feeds/find", None),
        ("/api/accounts", None),
        ("/api/register", None),
        ("/api/invites", None),
        ("/api/invites/4", None),
        ("/api/reports", None),
        ("/api/feeds/not-a-number/read", reading(None, None)),
        ("/api/login", None),
        ("/api/oidc/callback", None),
    ] {
        assert_eq!(changed(path), what, "{path}");
    }
}

#[sqlx::test]
async fn the_tabs_of_an_account_hear_of_a_change_and_nobody_else(db: PgPool) {
    let (ada, feed) = common::reader(&db, "ada", 3).await;
    let (grace, _) = common::reader(&db, "grace", 3).await;
    common::subscribe(&db, grace, feed).await;
    let state = AppState::new(db.clone());
    let ada_cookie = common::session(&db, ada).await;
    let phone_cookie = common::session(&db, ada).await;
    let grace_cookie = common::session(&db, grace).await;
    let mut tab = listening(&state, &ada_cookie).await;
    let mut phone = listening(&state, &phone_cookie).await;
    let mut stranger = listening(&state, &grace_cookie).await;

    // Ada reads an article in one tab.
    let path = format!("/api/feeds/{feed}/articles/3/state");
    assert_eq!(
        send(
            &state,
            Method::PUT,
            &path,
            &ada_cookie,
            Some("tab-1"),
            json!({ "read": true })
        )
        .await,
        StatusCode::NO_CONTENT
    );
    // Both of her connections hear of it, with the feed's numbers as they are
    // now and the name of the tab that did it.
    let heard = tab.next().await.expect("an event");
    assert_eq!(heard.kind, "reading");
    assert_eq!(
        heard.data,
        json!({
            "feed": { "feed_id": feed, "last_seq": 3, "read_upto": 0, "unread": 2 },
            "seq": 3,
            "by": "tab-1"
        })
    );
    assert_eq!(phone.next().await, Some(heard.clone()));
    // Grace reads the same feed and has not read that article.
    assert!(stranger.is_quiet().await);

    // Grace changes something of her own: only she hears of it.
    assert_eq!(
        send(
            &state,
            Method::PUT,
            "/api/settings",
            &grace_cookie,
            None,
            json!({ "theme": "dark" })
        )
        .await,
        StatusCode::NO_CONTENT
    );
    let hers = stranger.next().await.expect("an event");
    assert_eq!(
        (hers.kind.as_str(), &hers.data),
        ("settings", &json!({ "by": null }))
    );
    assert!(hers.number() > heard.number());
    assert!(tab.is_quiet().await);
    assert!(phone.is_quiet().await);

    // What does not go through changes nothing and is not told: a feed that
    // is not hers, an article that is not there, a request nobody is logged
    // in for, and one that only asks.
    let (_, other_feed) = common::reader(&db, "linus", 1).await;
    let foreign = format!("/api/feeds/{other_feed}/articles/1/state");
    let missing = format!("/api/feeds/{feed}/articles/99/state");
    for (path, cookie, status) in [
        (foreign.as_str(), ada_cookie.as_str(), StatusCode::NOT_FOUND),
        (missing.as_str(), ada_cookie.as_str(), StatusCode::NOT_FOUND),
        (path.as_str(), "nobody=home", StatusCode::UNAUTHORIZED),
    ] {
        let answer = send(
            &state,
            Method::PUT,
            path,
            cookie,
            None,
            json!({ "read": true }),
        )
        .await;
        assert_eq!(answer, status, "{path}");
    }
    let (status, _) = common::call(&state, Method::GET, "/api/tree", Some(&ada_cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    for stream in [&mut tab, &mut phone, &mut stranger] {
        assert!(stream.is_quiet().await);
    }
    assert_eq!(
        (state.events.streams(ada), state.events.streams(grace)),
        (2, 1)
    );
}

#[sqlx::test]
async fn every_change_through_the_api_is_told(db: PgPool) {
    let (ada, feed) = common::reader(&db, "ada", 2).await;
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;
    let mut tab = listening(&state, &cookie).await;
    let label = sqlx::query_scalar::<_, i32>(
        "INSERT INTO labels (account_id, title, color, position) VALUES ($1, 'Later', '#112233', 1)
         RETURNING id",
    )
    .bind(ada)
    .fetch_one(&db)
    .await
    .unwrap();
    let folder = common::folder(&db, ada, None, "News").await;

    let none = Value::Null;
    let numbers = |unread: i64, read_upto: i32| json!({ "feed_id": feed, "last_seq": 2, "read_upto": read_upto, "unread": unread });
    for (method, path, body, kind, feed_numbers) in [
        (
            Method::POST,
            "/api/folders".to_owned(),
            json!({ "title": "Tech", "parent_id": null }),
            "tree",
            none.clone(),
        ),
        (
            Method::PATCH,
            format!("/api/folders/{folder}"),
            json!({ "title": "World" }),
            "tree",
            none.clone(),
        ),
        (
            Method::PATCH,
            format!("/api/feeds/{feed}"),
            json!({ "title": "Mine" }),
            "tree",
            none.clone(),
        ),
        (
            Method::DELETE,
            format!("/api/folders/{folder}"),
            none.clone(),
            "tree",
            none.clone(),
        ),
        (
            Method::PUT,
            "/api/settings".to_owned(),
            json!({ "theme": "dark" }),
            "settings",
            none.clone(),
        ),
        (
            Method::POST,
            "/api/labels".to_owned(),
            json!({ "title": "Soon", "color": "#445566" }),
            "labels",
            none.clone(),
        ),
        (
            Method::PUT,
            format!("/api/feeds/{feed}/articles/1/labels/{label}"),
            none.clone(),
            "labels",
            none.clone(),
        ),
        (
            Method::DELETE,
            format!("/api/labels/{label}"),
            none.clone(),
            "labels",
            none.clone(),
        ),
        (
            Method::PUT,
            format!("/api/feeds/{feed}/articles/2/state"),
            json!({ "starred": true }),
            "reading",
            numbers(2, 0),
        ),
        (
            Method::POST,
            format!("/api/feeds/{feed}/read"),
            json!({ "upto": 1 }),
            "reading",
            numbers(1, 1),
        ),
        (
            Method::POST,
            "/api/read".to_owned(),
            json!({ "upto_id": i64::MAX }),
            "reading",
            none.clone(),
        ),
        (
            Method::DELETE,
            format!("/api/feeds/{feed}"),
            none.clone(),
            "tree",
            none.clone(),
        ),
    ] {
        let status = send(&state, method.clone(), &path, &cookie, Some("t"), body).await;
        assert!(status.is_success(), "{method} {path}: {status}");
        let heard = tab
            .next()
            .await
            .unwrap_or_else(|| panic!("no event for {method} {path}"));
        assert_eq!(heard.kind, kind, "{method} {path}");
        assert_eq!(heard.data["by"], "t");
        if kind == "reading" {
            assert_eq!(heard.data["feed"], feed_numbers, "{method} {path}");
        }
    }
    assert!(tab.is_quiet().await);
}

#[sqlx::test]
async fn a_browser_that_comes_back_gets_what_it_missed(db: PgPool) {
    let ada = common::account(&db, "ada").await;
    let grace = common::account(&db, "grace").await;
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;
    let events = &state.events;
    let tree = |by: &str| Change::Area {
        area: Area::Tree,
        by: Some(by.to_owned()),
    };

    // Something happened before anybody listened.
    events.publish(vec![ada], tree("before"));
    let mut tab = listening(&state, &cookie).await;
    // While the tab listens: one for her, one for somebody else, one for both.
    events.publish(vec![ada], tree("one"));
    events.publish(vec![grace], tree("not hers"));
    events.publish(vec![grace, ada], tree("two"));
    let one = tab.next().await.unwrap();
    let two = tab.next().await.unwrap();
    assert_eq!(
        (one.data["by"].as_str(), two.data["by"].as_str()),
        (Some("one"), Some("two"))
    );
    assert_eq!((one.number(), two.number()), (2, 4));
    drop(tab);

    // The connection breaks, and more happens.
    events.publish(vec![ada], tree("three"));
    events.publish(vec![grace], tree("not hers either"));
    events.publish(
        vec![ada],
        Change::Articles {
            feed_id: 9,
            last_seq: 12,
            new: 2,
        },
    );

    // Back with the last id it saw: exactly what it missed, in order, and then
    // what comes.
    let (_, mut back) = open(&state, Some(&cookie), Some(&two.id)).await;
    let three = back.next().await.unwrap();
    assert_eq!(
        (three.kind.as_str(), three.data["by"].as_str()),
        ("tree", Some("three"))
    );
    let articles = back.next().await.unwrap();
    assert_eq!(articles.kind, "articles");
    assert_eq!(
        articles.data,
        json!({ "feed_id": 9, "last_seq": 12, "new": 2 })
    );
    assert_eq!((three.number(), articles.number()), (5, 7));
    assert!(back.is_quiet().await);
    events.publish(vec![ada], tree("four"));
    assert_eq!(back.next().await.unwrap().data["by"], "four");

    // Back having missed nothing: nothing is said twice.
    let (_, mut current) = open(&state, Some(&cookie), Some(&articles.id)).await;
    assert_eq!(current.next().await.unwrap().data["by"], "four");
    let last = format!("{}-8", articles.id.rsplit_once('-').unwrap().0);
    let (_, mut current) = open(&state, Some(&cookie), Some(&last)).await;
    assert!(current.is_quiet().await);

    // An id the server cannot go on from means: load everything anew. That is
    // one from before the server started again, one from the future, and
    // nonsense.
    for unknown in [
        "000000000000-3",
        &format!("{}-999", last.rsplit_once('-').unwrap().0),
        "7",
        "",
        "x-y",
    ] {
        let (_, mut lost) = open(&state, Some(&cookie), Some(unknown)).await;
        let resync = lost.next().await.unwrap();
        assert_eq!(
            (resync.kind.as_str(), resync.number()),
            ("resync", 8),
            "{unknown}"
        );
        assert!(lost.is_quiet().await);
    }

    // And one so old that what followed it is no longer kept.
    for n in 0..KEPT {
        events.publish(vec![grace], tree(&n.to_string()));
    }
    let (_, mut old) = open(&state, Some(&cookie), Some(&articles.id)).await;
    assert_eq!(old.next().await.unwrap().kind, "resync");
    // The newest of the kept ones still does.
    let newest = format!("{}-{}", last.rsplit_once('-').unwrap().0, 8 + KEPT - 1);
    events.publish(vec![ada], tree("latest"));
    let (_, mut recent) = open(&state, Some(&cookie), Some(&newest)).await;
    assert_eq!(recent.next().await.unwrap().data["by"], "latest");
}

#[sqlx::test]
async fn a_connection_that_falls_behind_is_told_to_start_over(db: PgPool) {
    let ada = common::account(&db, "ada").await;
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;
    let mut tab = listening(&state, &cookie).await;
    // Far more than waits for a connection that does not read.
    for n in 0..1000 {
        let by = Some(n.to_string());
        state.events.publish(
            vec![ada],
            Change::Area {
                area: Area::Labels,
                by,
            },
        );
    }
    let told = tab.next().await.unwrap();
    assert_eq!((told.kind.as_str(), told.number()), ("resync", 1000));
    // After that it is up to date and hears what comes.
    assert!(tab.is_quiet().await);
    state.events.publish(
        vec![ada],
        Change::Area {
            area: Area::Tree,
            by: None,
        },
    );
    let next = tab.next().await.unwrap();
    assert_eq!((next.kind.as_str(), next.number()), ("tree", 1001));
}

#[sqlx::test]
async fn only_who_is_logged_in_listens_and_not_without_end(db: PgPool) {
    let ada = common::account(&db, "ada").await;
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;

    // Nobody without a session.
    for cookie in [None, Some("__Host-rueder=nonsense")] {
        let app = router(state.clone(), "missing".as_ref());
        let mut request = Request::get("/api/events");
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        let response = app
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    assert_eq!(state.events.streams(ada), 0);

    // An account holds only so many connections.
    let mut tabs = Vec::new();
    for _ in 0..STREAMS_PER_ACCOUNT {
        tabs.push(listening(&state, &cookie).await);
    }
    assert_eq!(state.events.streams(ada), STREAMS_PER_ACCOUNT);
    let one_more = || async {
        let app = router(state.clone(), "missing".as_ref());
        let request = Request::get("/api/events").header(header::COOKIE, &cookie);
        let response = app
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        response.status()
    };
    assert_eq!(one_more().await, StatusCode::TOO_MANY_REQUESTS);
    // Somebody else is not kept out by that.
    let grace = common::account(&db, "grace").await;
    let grace_cookie = common::session(&db, grace).await;
    let _hers = listening(&state, &grace_cookie).await;
    // A tab that closes makes room.
    tabs.pop();
    assert_eq!(state.events.streams(ada), STREAMS_PER_ACCOUNT - 1);
    assert_eq!(one_more().await, StatusCode::OK);
    drop(tabs);
    assert_eq!(state.events.streams(ada), 0);
}

#[sqlx::test]
async fn logging_out_ends_the_connections_of_that_session(db: PgPool) {
    let ada = common::account(&db, "ada").await;
    let state = AppState::new(db.clone());
    let laptop = common::session(&db, ada).await;
    let phone = common::session(&db, ada).await;
    let mut laptop_tab = listening(&state, &laptop).await;
    let mut other_laptop_tab = listening(&state, &laptop).await;
    let mut phone_tab = listening(&state, &phone).await;

    assert_eq!(
        send(
            &state,
            Method::POST,
            "/api/logout",
            &laptop,
            None,
            Value::Null
        )
        .await,
        StatusCode::NO_CONTENT
    );
    // The tabs of the session that ended are closed without another word.
    assert!(laptop_tab.has_ended().await);
    assert!(other_laptop_tab.has_ended().await);
    // The phone is still logged in. It hears that a session ended and stays.
    let heard = phone_tab.next().await.unwrap();
    assert_eq!(heard.kind, "session");
    state.events.publish(
        vec![ada],
        Change::Area {
            area: Area::Tree,
            by: None,
        },
    );
    assert_eq!(phone_tab.next().await.unwrap().kind, "tree");
    assert_eq!(state.events.streams(ada), 1);
}

#[sqlx::test]
async fn a_session_that_ends_outside_a_request_closes_its_connections(db: PgPool) {
    let ada = common::account(&db, "ada").await;
    let state = AppState {
        events: Events::checking_sessions_every(Duration::from_millis(100)),
        ..AppState::new(db.clone())
    };
    let laptop = common::session(&db, ada).await;
    let phone = common::session(&db, ada).await;
    let mut laptop_tab = listening(&state, &laptop).await;
    let mut phone_tab = listening(&state, &phone).await;

    // As the command line ends a session, or as one runs out: no request, so
    // no event.
    let token = laptop.split_once('=').unwrap().1;
    rueder::sessions::end(&db, token).await.unwrap();
    assert!(laptop_tab.has_ended().await);
    // The other session's connection stays and hears what comes.
    tokio::time::sleep(Duration::from_millis(300)).await;
    state.events.publish(
        vec![ada],
        Change::Area {
            area: Area::Tree,
            by: None,
        },
    );
    assert_eq!(phone_tab.next().await.unwrap().kind, "tree");
    assert_eq!(state.events.streams(ada), 1);
}

#[sqlx::test]
async fn deleting_an_account_ends_all_its_connections_at_once(db: PgPool) {
    let ada = rueder::accounts::create(&db, "ada", Some("a long password"), false)
        .await
        .unwrap()
        .unwrap();
    let grace = common::account(&db, "grace").await;
    let state = AppState::new(db.clone());
    let laptop = common::session(&db, ada).await;
    let phone = common::session(&db, ada).await;
    let theirs = common::session(&db, grace).await;
    let mut laptop_tab = listening(&state, &laptop).await;
    let mut phone_tab = listening(&state, &phone).await;
    let mut grace_tab = listening(&state, &theirs).await;

    assert_eq!(
        send(
            &state,
            Method::DELETE,
            "/api/account",
            &laptop,
            None,
            json!({ "password": "a long password" }),
        )
        .await,
        StatusCode::NO_CONTENT
    );
    // Within the time a connection takes to see a session that ended through
    // a request, well before it would look by itself.
    assert!(laptop_tab.has_ended().await);
    assert!(phone_tab.has_ended().await);
    assert_eq!(state.events.streams(ada), 0);
    assert_eq!(
        open(&state, Some(&phone), None).await.0,
        StatusCode::UNAUTHORIZED
    );
    // Grace hears nothing of it and stays.
    assert!(grace_tab.is_quiet().await);
    assert_eq!(state.events.streams(grace), 1);
}

#[sqlx::test]
async fn a_disabled_account_gets_no_events(db: PgPool) {
    let (ada, feed) = common::reader(&db, "ada", 3).await;
    let grace = common::account(&db, "grace").await;
    common::subscribe(&db, grace, feed).await;
    let state = AppState {
        events: Events::checking_sessions_every(Duration::from_millis(100)),
        ..AppState::new(db.clone())
    };
    let laptop = common::session(&db, ada).await;
    let theirs = common::session(&db, grace).await;
    let mut ada_tab = listening(&state, &laptop).await;
    let mut grace_tab = listening(&state, &theirs).await;

    // On the command line, so no request tells the connections.
    assert!(
        rueder::accounts::set_disabled(&db, "ada", true)
            .await
            .unwrap()
    );
    assert!(ada_tab.has_ended().await);
    // No new connection, not even with a session made after disabling.
    assert_eq!(
        open(&state, Some(&laptop), None).await.0,
        StatusCode::UNAUTHORIZED
    );
    let after = common::session(&db, ada).await;
    assert_eq!(
        open(&state, Some(&after), None).await.0,
        StatusCode::UNAUTHORIZED
    );
    // New articles of the feed reach Grace only.
    let change = Change::Articles {
        feed_id: feed,
        last_seq: 4,
        new: 1,
    };
    state
        .events
        .publish_to_readers(&db, feed, change)
        .await
        .unwrap();
    assert_eq!(grace_tab.next().await.unwrap().kind, "articles");
    assert_eq!(state.events.streams(ada), 0);
}

#[test]
fn a_fetch_tells_what_it_found() {
    let stored = |new, updated, first_new| {
        Outcome::Fetched(Stored {
            new,
            updated,
            first_new,
        })
    };
    let works = Change::Feed {
        feed_id: 4,
        error: None,
    };
    let articles = |last_seq, new| Change::Articles {
        feed_id: 4,
        last_seq,
        new,
    };
    for (was_failing, outcome, changes) in [
        (false, Outcome::NotModified, vec![]),
        (false, stored(0, 0, None), vec![]),
        (false, stored(3, 0, Some(8)), vec![articles(10, 3)]),
        (
            false,
            stored(0, 2, None),
            vec![Change::Updated {
                feed_id: 4,
                count: 2,
            }],
        ),
        (
            false,
            stored(1, 1, Some(1)),
            vec![
                articles(1, 1),
                Change::Updated {
                    feed_id: 4,
                    count: 1,
                },
            ],
        ),
        // A feed that fails says so once, and once when it works again.
        (
            false,
            Outcome::Failed("gone".into()),
            vec![Change::Feed {
                feed_id: 4,
                error: Some("gone".into()),
            }],
        ),
        (true, Outcome::Failed("still gone".into()), vec![]),
        (true, Outcome::NotModified, vec![works.clone()]),
        (
            true,
            stored(2, 0, Some(5)),
            vec![works.clone(), articles(6, 2)],
        ),
    ] {
        assert_eq!(
            fetch::changes(4, was_failing, outcome.clone()),
            changes,
            "{outcome:?}"
        );
    }
}

/// A site with a feed that can be changed and broken.
#[derive(Default)]
struct Site {
    entries: Vec<(&'static str, &'static str)>,
    broken: bool,
}

async fn feed_xml(State(site): State<Arc<Mutex<Site>>>) -> (StatusCode, String) {
    let site = site.lock().unwrap();
    if site.broken {
        return (StatusCode::INTERNAL_SERVER_ERROR, String::new());
    }
    let items: String = site
        .entries
        .iter()
        .map(|(id, text)| {
            format!(
                "<item><guid>{id}</guid><title>{id}</title><description>{text}</description></item>"
            )
        })
        .collect();
    let feed = format!(
        "<?xml version=\"1.0\"?><rss version=\"2.0\"><channel><title>Example</title>{items}</channel></rss>"
    );
    (StatusCode::OK, feed)
}

#[sqlx::test]
async fn the_readers_of_a_feed_hear_what_a_round_of_fetching_finds(db: PgPool) {
    let site: Arc<Mutex<Site>> = Arc::default();
    site.lock().unwrap().entries = vec![("b", "two"), ("a", "one")];
    let app = Router::new()
        .route("/feed.xml", get(feed_xml))
        .with_state(site.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let (ada, feed) = common::reader(&db, "ada", 0).await;
    let (grace, other_feed) = common::reader(&db, "grace", 0).await;
    let linus = common::account(&db, "linus").await;
    common::subscribe(&db, linus, feed).await;
    sqlx::query("UPDATE feeds SET feed_url = $2 WHERE id = $1")
        .bind(feed)
        .bind(format!("http://{addr}/feed.xml"))
        .execute(&db)
        .await
        .unwrap();
    // The other feed is not due, so the round leaves it alone.
    sqlx::query("UPDATE feeds SET next_fetch_at = now() + interval '1 day' WHERE id = $1")
        .bind(other_feed)
        .execute(&db)
        .await
        .unwrap();

    let state = AppState::new(db.clone());
    let mut reader = listening(&state, &common::session(&db, ada).await).await;
    let mut second_reader = listening(&state, &common::session(&db, linus).await).await;
    let mut stranger = listening(&state, &common::session(&db, grace).await).await;
    let client = HttpClient::new(Config {
        allow: vec!["127.0.0.1".parse().unwrap()],
        ..Config::default()
    })
    .unwrap();
    let rules = Arc::new(Rules::built_in().unwrap());
    let round = || async {
        sqlx::query("UPDATE feeds SET next_fetch_at = now() WHERE id = $1")
            .bind(feed)
            .execute(&db)
            .await
            .unwrap();
        fetch::fetch_due(
            &db,
            &client,
            &rules,
            &state.events,
            &state.monitor,
            &fetch::Pace::default(),
        )
        .await
        .unwrap()
    };

    // New articles: everybody who reads the feed hears of them.
    assert_eq!(round().await, 1);
    let heard = reader.next().await.unwrap();
    assert_eq!(heard.kind, "articles");
    assert_eq!(
        heard.data,
        json!({ "feed_id": feed, "last_seq": 2, "new": 2 })
    );
    assert_eq!(second_reader.next().await, Some(heard));
    // Nothing new: nothing is said.
    assert_eq!(round().await, 1);
    assert!(reader.is_quiet().await);

    // An article changed and another arrived.
    site.lock().unwrap().entries = vec![("c", "three"), ("b", "two, corrected"), ("a", "one")];
    round().await;
    let heard = reader.next().await.unwrap();
    assert_eq!(
        (heard.kind.as_str(), &heard.data),
        (
            "articles",
            &json!({ "feed_id": feed, "last_seq": 3, "new": 1 })
        )
    );
    let heard = reader.next().await.unwrap();
    assert_eq!(
        (heard.kind.as_str(), &heard.data),
        ("updated", &json!({ "feed_id": feed, "count": 1 }))
    );

    // The site breaks: said once, with the reason. It mends: said once.
    site.lock().unwrap().broken = true;
    round().await;
    let heard = reader.next().await.unwrap();
    assert_eq!(heard.kind, "feed");
    assert_eq!(heard.data["feed_id"], feed);
    assert!(
        heard.data["error"].as_str().unwrap().contains("500"),
        "{}",
        heard.data
    );
    round().await;
    assert!(reader.is_quiet().await);
    site.lock().unwrap().broken = false;
    round().await;
    let heard = reader.next().await.unwrap();
    assert_eq!(
        (heard.kind.as_str(), &heard.data),
        ("feed", &json!({ "feed_id": feed, "error": null }))
    );

    // Who does not read the feed heard nothing of all this.
    assert!(stranger.is_quiet().await);
    for _ in 0..4 {
        assert!(second_reader.next().await.is_some());
    }
    assert!(second_reader.is_quiet().await);
}

#[test]
fn events_say_what_they_are_about() {
    let feed = FeedUnread {
        feed_id: 2,
        last_seq: 9,
        read_upto: 4,
        unread: 3,
    };
    for (change, kind, data) in [
        (
            Change::Articles {
                feed_id: 2,
                last_seq: 9,
                new: 1,
            },
            "articles",
            json!({ "feed_id": 2, "last_seq": 9, "new": 1 }),
        ),
        (
            Change::Updated {
                feed_id: 2,
                count: 3,
            },
            "updated",
            json!({ "feed_id": 2, "count": 3 }),
        ),
        (
            Change::Feed {
                feed_id: 2,
                error: Some("no answer".into()),
            },
            "feed",
            json!({ "feed_id": 2, "error": "no answer" }),
        ),
        (
            Change::Reading {
                feed: Some(feed),
                seq: Some(5),
                by: None,
            },
            "reading",
            json!({ "feed": { "feed_id": 2, "last_seq": 9, "read_upto": 4, "unread": 3 }, "seq": 5, "by": null }),
        ),
        (
            Change::Reading {
                feed: None,
                seq: None,
                by: Some("t".into()),
            },
            "reading",
            json!({ "feed": null, "seq": null, "by": "t" }),
        ),
        (
            Change::Area {
                area: Area::Tree,
                by: None,
            },
            "tree",
            json!({ "by": null }),
        ),
        (
            Change::Area {
                area: Area::Labels,
                by: None,
            },
            "labels",
            json!({ "by": null }),
        ),
        (
            Change::Area {
                area: Area::Settings,
                by: None,
            },
            "settings",
            json!({ "by": null }),
        ),
        (
            Change::Area {
                area: Area::Rules,
                by: None,
            },
            "rules",
            json!({ "by": null }),
        ),
        (
            Change::Area {
                area: Area::Session,
                by: None,
            },
            "session",
            json!({ "by": null }),
        ),
    ] {
        assert_eq!((change.kind().as_str(), change.data()), (kind, data));
    }
    // Nobody to tell is nothing to keep.
    let events = Events::new();
    events.publish(
        vec![],
        Change::Area {
            area: Area::Tree,
            by: None,
        },
    );
    assert_eq!(events.streams(1), 0);
}

#[sqlx::test]
async fn a_tab_names_itself_only_in_plain_words(db: PgPool) {
    let ada = common::account(&db, "ada").await;
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;
    let mut tab = listening(&state, &cookie).await;
    let long = "x".repeat(65);
    for (name, by) in [
        (Some("Tab_1-a"), json!("Tab_1-a")),
        (Some("<script>"), Value::Null),
        (Some("two words"), Value::Null),
        (Some(long.as_str()), Value::Null),
        (Some(""), Value::Null),
        (None, Value::Null),
    ] {
        let status = send(
            &state,
            Method::PUT,
            "/api/settings",
            &cookie,
            name,
            json!({}),
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(tab.next().await.unwrap().data["by"], by, "{name:?}");
    }
}

#[sqlx::test]
async fn a_question_that_is_sent_like_a_change_is_not_told(db: PgPool) {
    let (ada, _) = common::reader(&db, "ada", 2).await;
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;
    let mut tab = listening(&state, &cookie).await;
    let rule = json!({
        "scope": { "kind": "all" },
        "field": "title",
        "match": "contains",
        "pattern": "article 2",
        "action": "hide",
    });

    // Asking what a rule would do changes nothing.
    let status = send(
        &state,
        Method::POST,
        "/api/rules/preview",
        &cookie,
        None,
        rule.clone(),
    )
    .await;
    assert!(status.is_success(), "{status}");
    assert!(tab.is_quiet().await);
    // Making the rule does: it hides an article, so the counts are others now.
    let status = send(&state, Method::POST, "/api/rules", &cookie, Some("t"), rule).await;
    assert_eq!(status, StatusCode::CREATED);
    let heard = tab.next().await.unwrap();
    assert_eq!(
        (heard.kind.as_str(), &heard.data),
        ("rules", &json!({ "by": "t" }))
    );
    assert!(tab.is_quiet().await);
}
