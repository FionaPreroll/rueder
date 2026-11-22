//! Web pages without a feed, through RSSifier: the search offers such a page,
//! subscribing makes a feed of it, the fetching asks RSSifier for it, its
//! problems stand at the feed, the status says how RSSifier fares, and a page
//! moves as a page in OPML and in the file of an account. The site and
//! RSSifier are made up, on this machine.

mod common;

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use axum::{
    Router,
    extract::{Query, State},
    http::{HeaderMap, Method, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use common::call;
use rueder::{
    AppState, accounts,
    cleaning::Rules,
    fetch::{self, Outcome},
    http_client::{Config, HttpClient},
    rssifier::Rssifier,
};
use serde_json::{Value, json};
use sqlx::PgPool;

const TOKEN: &str = "rsf_made_up_for_the_test";

const PAGE: &str = r#"<!doctype html><html><head><title>The club</title></head>
    <body><ul><li><a href="/news/1">Summer party</a></li></ul></body></html>"#;

const BLOG: &str = r#"<!doctype html><html><head>
    <link rel="alternate" type="application/rss+xml" href="/feed.xml"></head></html>"#;

/// What the made-up RSSifier saw.
#[derive(Default)]
struct Seen {
    feeds: AtomicUsize,
    conditional: AtomicUsize,
}

/// The made-up RSSifier: the club's page is a feed of two entries, written
/// without its tracking parameter; the empty page is a problem.
async fn rssifier_feed(
    State((site, seen)): State<(String, Arc<Seen>)>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let problem = |status: StatusCode, kind: &str, detail: &str| {
        let body = json!({ "type": kind, "status": status.as_u16(), "detail": detail });
        let kind = [(header::CONTENT_TYPE, "application/problem+json")];
        (status, kind, body.to_string()).into_response()
    };
    let said = |name| headers.get(name).and_then(|v| v.to_str().ok());
    if said(header::AUTHORIZATION) != Some(&format!("Bearer {TOKEN}")) {
        return problem(StatusCode::UNAUTHORIZED, "token", "this token is not known");
    }
    seen.feeds.fetch_add(1, Ordering::SeqCst);
    let page = query.get("url").cloned().unwrap_or_default();
    if page.starts_with(&format!("{site}/empty")) {
        return problem(
            StatusCode::UNPROCESSABLE_ENTITY,
            "no-items",
            "the page lists nothing RSSifier can tell apart",
        );
    }
    if !page.starts_with(&format!("{site}/club")) {
        return problem(StatusCode::BAD_GATEWAY, "upstream", "no such page");
    }
    let etag = "\"club-1\"";
    if said(header::IF_NONE_MATCH) == Some(etag) {
        seen.conditional.fetch_add(1, Ordering::SeqCst);
        return (StatusCode::NOT_MODIFIED, [(header::ETAG, etag)]).into_response();
    }
    let item = |n: u32, title: &str| {
        json!({
            "id": format!("{site}/news/{n}"),
            "url": format!("{site}/news/{n}"),
            "title": title,
            "content_html": format!("<p>What {title} is about.</p>"),
            "date_published": format!("2026-11-0{n}T10:00:00Z"),
        })
    };
    let feed = json!({
        "version": "https://jsonfeed.org/version/1.1",
        "title": "The club",
        "home_page_url": format!("{site}/club"),
        "items": [item(2, "General meeting"), item(1, "Summer party")],
        "_rssifier": { "source": "guess" },
    });
    let headers = [
        (header::CONTENT_TYPE, "application/feed+json"),
        (header::ETAG, etag),
    ];
    (headers, feed.to_string()).into_response()
}

async fn rssifier_usage(headers: HeaderMap) -> Response {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    if token != Some(&format!("Bearer {TOKEN}")) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let usage = json!({
        "name": "rueder",
        "limits": { "per_minute": 120, "fetches_per_hour": null, "at_once": 8 },
        "requests": 3, "fetches": 2, "under_way": 0,
    });
    (
        [(header::CONTENT_TYPE, "application/json")],
        usage.to_string(),
    )
        .into_response()
}

/// Starts the site and RSSifier, and returns the site's address, RSSifier's
/// and what RSSifier saw.
#[allow(
    clippy::disallowed_methods,
    reason = "a made-up site and RSSifier of the test, not routes of rueder"
)]
async fn start() -> (String, String, Arc<Seen>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let site = format!("http://{addr}");
    let pages = Router::new()
        .route(
            "/club",
            get(async || ([(header::CONTENT_TYPE, "text/html")], PAGE)),
        )
        .route(
            "/empty",
            get(async || ([(header::CONTENT_TYPE, "text/html")], PAGE)),
        )
        .route(
            "/blog",
            get(async || ([(header::CONTENT_TYPE, "text/html")], BLOG)),
        );
    tokio::spawn(async move { axum::serve(listener, pages).await.unwrap() });

    let seen = Arc::new(Seen::default());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let rssifier = Router::new()
        .route("/v1/feed", get(rssifier_feed))
        .route("/v1/usage", get(rssifier_usage))
        .with_state((site.clone(), seen.clone()));
    tokio::spawn(async move { axum::serve(listener, rssifier).await.unwrap() });
    (site, format!("http://{addr}"), seen)
}

/// A server that reaches the site on this machine, and RSSifier with `token`.
fn state(db: &PgPool, rssifier: Option<(&str, &str)>) -> AppState {
    let config = Config {
        allow: vec!["127.0.0.1".parse().unwrap()],
        ..Config::default()
    };
    AppState {
        client: Some(HttpClient::new(config).unwrap()),
        rssifier: rssifier.map(|(url, token)| Arc::new(Rssifier::new(url, token).unwrap())),
        ..AppState::new(db.clone())
    }
}

async fn post(state: &AppState, path: &str, cookie: &str, body: Value) -> (StatusCode, Value) {
    call(state, Method::POST, path, Some(cookie), Some(body)).await
}

#[sqlx::test]
async fn the_search_offers_a_page_without_a_feed_and_subscribing_makes_one(db: PgPool) {
    let (site, rssifier, seen) = start().await;
    let state = state(&db, Some((&rssifier, TOKEN)));
    let ada = common::account(&db, "ada").await;
    let cookie = common::session(&db, ada).await;
    let find =
        async |url: String| post(&state, "/api/feeds/find", &cookie, json!({ "url": url })).await;

    // A page that names a feed is not asked of RSSifier.
    let (status, found) = find(format!("{site}/blog")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(found["feeds"][0]["page"], json!(false));
    assert_eq!(seen.feeds.load(Ordering::SeqCst), 0);

    // One that names none is, and comes as RSSifier writes it.
    let page = format!("{site}/club");
    let (status, found) = find(format!("{site}/club?utm_source=x")).await;
    assert_eq!(status, StatusCode::OK);
    let offered = json!({ "url": page, "title": "The club", "feed_id": null, "page": true });
    assert_eq!(found, json!({ "feeds": [offered], "moved_to": null }));
    // One where RSSifier finds nothing has no feed either.
    let (status, found) = find(format!("{site}/empty")).await;
    assert_eq!((status, &found["feeds"]), (StatusCode::OK, &json!([])));

    let new = json!({ "url": page, "page": true });
    let (status, made) = post(&state, "/api/feeds", &cookie, new.clone()).await;
    assert_eq!(status, StatusCode::CREATED);
    let id = made["id"].as_i64().unwrap() as i32;
    let (source, url, site_url): (String, String, Option<String>) =
        sqlx::query_as("SELECT source, feed_url, site_url FROM feeds WHERE id = $1")
            .bind(id)
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!((source.as_str(), url.as_str()), ("page", page.as_str()));
    assert_eq!(site_url.as_deref(), Some(page.as_str()));
    let titles: Vec<String> =
        sqlx::query_scalar("SELECT title FROM articles WHERE feed_id = $1 ORDER BY seq")
            .bind(id)
            .fetch_all(&db)
            .await
            .unwrap();
    assert_eq!(titles, ["Summer party", "General meeting"]);
    // The search says it is read now.
    let (_, found) = find(page.clone()).await;
    assert_eq!(found["feeds"][0]["feed_id"], json!(id));

    // Fetching it again asks RSSifier whether it changed.
    let rules = Arc::new(Rules::built_in().unwrap());
    let client = state.client.clone().unwrap();
    let outcome = fetch::fetch_feed(&db, &client, state.rssifier.as_deref(), &rules, id)
        .await
        .unwrap();
    assert_eq!(outcome, Outcome::NotModified);
    assert_eq!(seen.conditional.load(Ordering::SeqCst), 1);

    // Without RSSifier the page is no feed that can be fetched or made.
    let outcome = fetch::fetch_feed(&db, &client, None, &rules, id)
        .await
        .unwrap();
    assert_eq!(
        outcome,
        Outcome::Failed("this server reads no web pages without a feed".into())
    );
    let without = self::state(&db, None);
    let grace = common::account(&db, "grace").await;
    let grace_cookie = common::session(&db, grace).await;
    let other = json!({ "url": format!("{site}/club/other"), "page": true });
    let (status, _) = post(&without, "/api/feeds", &grace_cookie, other).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    let (_, found) = post(
        &without,
        "/api/feeds/find",
        &grace_cookie,
        json!({ "url": page }),
    )
    .await;
    assert_eq!(found["feeds"], json!([]));
}

#[sqlx::test]
async fn another_account_sees_nothing_of_a_page_and_a_feed_is_not_a_page(db: PgPool) {
    let (site, rssifier, _) = start().await;
    let state = state(&db, Some((&rssifier, TOKEN)));
    let ada = common::account(&db, "ada").await;
    let grace = common::account(&db, "grace").await;
    let (cookie, grace_cookie) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
    );
    let page = format!("{site}/club");
    let (status, made) = post(
        &state,
        "/api/feeds",
        &cookie,
        json!({ "url": page, "page": true }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (_, found) = post(
        &state,
        "/api/feeds/find",
        &grace_cookie,
        json!({ "url": page }),
    )
    .await;
    assert_eq!(found["feeds"][0]["feed_id"], Value::Null);
    let (_, tree) = call(&state, Method::GET, "/api/tree", Some(&grace_cookie), None).await;
    assert_eq!(tree["feeds"], json!([]));

    // Grace subscribing to the page gets the same feed; the address as a feed
    // is none, and makes no second one.
    let (status, shared) = post(
        &state,
        "/api/feeds",
        &grace_cookie,
        json!({ "url": page, "page": true }),
    )
    .await;
    assert_eq!((status, &shared["id"]), (StatusCode::CREATED, &made["id"]));
    let (status, _) = post(&state, "/api/feeds", &grace_cookie, json!({ "url": page })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let feeds: i64 = sqlx::query_scalar("SELECT count(*) FROM feeds")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(feeds, 1);
}

#[sqlx::test]
async fn what_rssifier_says_is_wrong_stands_at_the_feed(db: PgPool) {
    let (site, rssifier, _) = start().await;
    let (_, feed) = common::reader(&db, "ada", 0).await;
    sqlx::query("UPDATE feeds SET feed_url = $2, source = 'page' WHERE id = $1")
        .bind(feed)
        .bind(format!("{site}/empty"))
        .execute(&db)
        .await
        .unwrap();
    let rules = Arc::new(Rules::built_in().unwrap());
    let client = HttpClient::new(Config::default()).unwrap();
    let error = async |rssifier: &Rssifier| {
        fetch::fetch_feed(&db, &client, Some(rssifier), &rules, feed)
            .await
            .unwrap();
        sqlx::query_scalar::<_, Option<String>>("SELECT error_message FROM feeds WHERE id = $1")
            .bind(feed)
            .fetch_one(&db)
            .await
            .unwrap()
    };
    let right = Rssifier::new(&rssifier, TOKEN).unwrap();
    assert_eq!(
        error(&right).await.as_deref(),
        Some("the page lists nothing RSSifier can tell apart")
    );
    let wrong = Rssifier::new(&rssifier, "rsf_wrong").unwrap();
    assert_eq!(
        error(&wrong).await.as_deref(),
        Some("this token is not known")
    );
}

#[sqlx::test]
async fn the_status_says_how_rssifier_fares(db: PgPool) {
    let (site, rssifier, _) = start().await;
    let state = state(&db, Some((&rssifier, TOKEN)));
    let ada = common::account(&db, "ada").await;
    accounts::set_admin(&db, "ada", true).await.unwrap();
    let cookie = common::session(&db, ada).await;
    let new = json!({ "url": format!("{site}/club"), "page": true });
    let (status, _) = post(&state, "/api/feeds", &cookie, new).await;
    assert_eq!(status, StatusCode::CREATED);

    let status = async |state: &AppState| {
        let (status, body) = call(state, Method::GET, "/api/status", Some(&cookie), None).await;
        assert_eq!(status, StatusCode::OK);
        body["rssifier"].clone()
    };
    assert_eq!(
        status(&state).await,
        json!({
            "on": true, "pages": 1, "failing": 0, "error": null,
            "usage": {
                "name": "rueder",
                "limits": { "per_minute": 120, "fetches_per_hour": null, "at_once": 8 },
                "requests": 3, "fetches": 2, "under_way": 0,
            },
        })
    );
    let wrong = self::state(&db, Some((&rssifier, "rsf_wrong")));
    let said = status(&wrong).await;
    assert_eq!(
        (&said["usage"], said["error"].is_string()),
        (&Value::Null, true)
    );
    let off = status(&self::state(&db, None)).await;
    assert_eq!(
        off,
        json!({ "on": false, "pages": 1, "failing": 0, "usage": null, "error": null })
    );
}

#[sqlx::test]
async fn a_page_moves_as_a_page_in_opml_and_in_the_file_of_an_account(db: PgPool) {
    // Without RSSifier too: what comes with a file is fetched later.
    let state = state(&db, None);
    let ada = common::account(&db, "ada").await;
    let blog = common::feed(&db, "blog").await;
    let page: i32 = sqlx::query_scalar(
        "INSERT INTO feeds (feed_url, title, source)
         VALUES ('https://club.example/news', 'The club', 'page') RETURNING id",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    common::subscribe(&db, ada, blog).await;
    common::subscribe(&db, ada, page).await;
    let cookie = common::session(&db, ada).await;
    let subscribed = async |account: i32| {
        sqlx::query_scalar::<_, i32>(
            "SELECT feed_id FROM subscriptions WHERE account_id = $1 ORDER BY feed_id",
        )
        .bind(account)
        .fetch_all(&db)
        .await
        .unwrap()
    };

    let (status, _, opml) =
        common::raw(&state, Method::GET, "/api/opml", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    let opml = String::from_utf8(opml).unwrap();
    assert!(
        opml.contains(
            r#"<outline type="link" text="The club" title="The club" url="https://club.example/news" rueder:source="page"/>"#
        ),
        "{opml}"
    );
    assert!(
        opml.contains(r#"xmlUrl="https://blog.example/feed.xml""#),
        "{opml}"
    );
    let grace = common::account(&db, "grace").await;
    let grace_cookie = common::session(&db, grace).await;
    let body = Some(("text/x-opml", opml.into_bytes()));
    let (status, _, _) =
        common::raw(&state, Method::POST, "/api/opml", Some(&grace_cookie), body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(subscribed(grace).await, [blog, page]);

    let (status, file) = call(&state, Method::GET, "/api/export", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    let pages: Vec<_> = file["tree"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| {
            (
                node["url"].as_str().unwrap(),
                node["page"].as_bool().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        pages,
        [
            ("https://blog.example/feed.xml", false),
            ("https://club.example/news", true)
        ]
    );
    let linus = common::account(&db, "linus").await;
    let linus_cookie = common::session(&db, linus).await;
    let (status, _) = post(&state, "/api/import", &linus_cookie, file).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(subscribed(linus).await, [blog, page]);
    let feeds: i64 = sqlx::query_scalar("SELECT count(*) FROM feeds")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(feeds, 2);
}
