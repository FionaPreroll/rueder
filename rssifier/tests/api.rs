//! RSSifier as a client sees it, against a made-up site on this machine: the
//! token, the feed, the cache, the limits and the problems.

use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
    response::Html,
    routing::get,
};
use hardened_http::{Config, HttpClient};
use http_body_util::BodyExt;
use rssifier::{
    AppState, Sites, app,
    limits::Pace,
    recipes::Recipes,
    tokens::{self, Keeper, Limits, Token},
};
use serde_json::Value;
use tower::ServiceExt;

const LISTED: &str = r#"<html><head><title>Club news</title>
<script type="application/ld+json">{"@type":"ItemList","itemListElement":[
 {"@type":"ListItem","item":{"@type":"NewsArticle","headline":"Summer fair","url":"/news/fair","datePublished":"2026-07-01","description":"<b>Come</b> along"}},
 {"@type":"ListItem","item":{"@type":"NewsArticle","headline":"New pitch","url":"/news/pitch"}},
 {"@type":"ListItem","item":{"@type":"NewsArticle","headline":"Elections","url":"/news/elections"}}]}</script>
</head><body><h1>News</h1></body></html>"#;

/// A page that says nothing about itself, just looks like a list.
const CLUB: &str = r#"<title>Club</title><nav><a href="/">Home</a> <a href="/club">Club</a></nav>
<main><div class=item><h3><a href="/club/fair">The summer fair is coming</a></h3><p>Bring a cake.</p></div>
<div class=item><h3><a href="/club/pitch">Our new pitch is open now</a></h3><p>Since 3 May 2026.</p></div>
<div class=item><h3><a href="/club/vote">Elections for the board</a></h3><p>Vote! 4 comments</p></div></main>"#;

#[allow(
    clippy::disallowed_methods,
    reason = "a made-up site of the test, not a route of RSSifier"
)]
fn site(hits: Arc<AtomicUsize>) -> Router {
    let counted = move |page: &'static str| {
        let hits = hits.clone();
        move || async move {
            hits.fetch_add(1, Ordering::SeqCst);
            Html(page)
        }
    };
    Router::new()
        .route("/news", get(counted(LISTED)))
        .route(
            "/plain",
            get(counted("<title>About</title><p>Just one page.</p>")),
        )
        .route(
            "/app",
            get(counted(
                r#"<div id=app></div><script src="/app.js"></script>"#,
            )),
        )
        .route("/club", get(counted(CLUB)))
        .route("/private", get(counted(LISTED)))
        .route(
            "/robots.txt",
            get(|| async { "User-agent: rueder-rssifier\nDisallow: /private\n" }),
        )
        .route("/gone", get(|| async { StatusCode::NOT_FOUND }))
}

struct World {
    app: Router,
    site: SocketAddr,
    hits: Arc<AtomicUsize>,
    tokens: PathBuf,
}

impl World {
    async fn new(limits: Limits) -> (Self, String) {
        Self::with(limits, Sites::new(Recipes::default(), "")).await
    }

    async fn with(limits: Limits, sites: Sites) -> (Self, String) {
        let hits = Arc::new(AtomicUsize::new(0));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let site = listener.local_addr().unwrap();
        let router = self::site(hits.clone());
        tokio::spawn(async move { axum::serve(listener, router).await });

        let file =
            std::env::temp_dir().join(format!("rssifier-{}.json", tokens::new_secret().unwrap()));
        let secret = tokens::new_secret().unwrap();
        let token = Token {
            name: "rueder".into(),
            hash: tokens::hash(&secret),
            limits,
        };
        tokens::save(&file, &[token]).unwrap();
        let client = HttpClient::new(Config {
            agent: rssifier::AGENT.into(),
            allow: vec!["127.0.0.1".parse().unwrap()],
            ..Config::default()
        })
        .unwrap();
        let pace = Pace {
            host_spacing: Duration::ZERO,
            ..Pace::default()
        };
        let state = AppState::new(client, Keeper::new(file.clone()).unwrap(), pace, sites);
        let world = Self {
            app: app(state),
            site,
            hits,
            tokens: file,
        };
        (world, secret)
    }

    async fn get(
        &self,
        path: &str,
        secret: Option<&str>,
        etag: Option<&str>,
    ) -> (StatusCode, header::HeaderMap, Value) {
        let mut request = Request::get(path);
        if let Some(secret) = secret {
            request = request.header(header::AUTHORIZATION, format!("Bearer {secret}"));
        }
        if let Some(etag) = etag {
            request = request.header(header::IF_NONE_MATCH, etag);
        }
        let response = self
            .app
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let (parts, body) = response.into_parts();
        let bytes = body.collect().await.unwrap().to_bytes();
        let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (parts.status, parts.headers, json)
    }

    fn feed_of(&self, path: &str) -> String {
        let page = format!("http://{}{path}", self.site);
        format!(
            "/v1/feed?url={}",
            url::form_urlencoded::byte_serialize(page.as_bytes()).collect::<String>()
        )
    }
}

impl Drop for World {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.tokens);
    }
}

#[tokio::test]
async fn a_page_with_a_list_becomes_a_feed_that_is_fetched_once_for_a_while() {
    let (world, secret) = World::new(Limits::default()).await;
    let (status, headers, feed) = world
        .get(&world.feed_of("/news#top"), Some(&secret), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{feed}");
    assert_eq!(headers[header::CONTENT_TYPE], "application/feed+json");
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    assert_eq!(feed["version"], "https://jsonfeed.org/version/1.1");
    assert_eq!(feed["title"], "Club news");
    assert_eq!(feed["home_page_url"], format!("http://{}/news", world.site));
    assert_eq!(feed["_rssifier"]["source"], "json-ld");
    let first = &feed["items"][0];
    assert_eq!(first["id"], format!("http://{}/news/fair", world.site));
    assert_eq!(first["title"], "Summer fair");
    // Text from the page stays text.
    assert_eq!(
        first["content_html"],
        "<p>&lt;b&gt;Come&lt;/b&gt; along</p>"
    );
    assert_eq!(first["date_published"], "2026-07-01T00:00:00Z");
    assert_eq!(feed["items"].as_array().unwrap().len(), 3);

    // The same page again, also written another way, comes from the cache.
    let etag = headers[header::ETAG].to_str().unwrap().to_owned();
    let (status, _, _) = world
        .get(&world.feed_of("/news"), Some(&secret), Some(&etag))
        .await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert_eq!(world.hits.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn without_a_known_token_nothing_is_fetched() {
    let (world, _) = World::new(Limits::default()).await;
    for secret in [None, Some("rsf_aaaaaaaaaaaaaaaaaaaaaaaa")] {
        let (status, headers, problem) = world.get(&world.feed_of("/news"), secret, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(headers[header::CONTENT_TYPE], "application/problem+json");
        assert_eq!(problem["type"], "token");
    }
    let (status, _, _) = world.get("/v1/usage", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(world.hits.load(Ordering::SeqCst), 0);
    // What says that the server runs needs none.
    let (status, _, _) = world.get("/health", None, None).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn a_revoked_token_ends_with_the_next_request() {
    let (world, secret) = World::new(Limits::default()).await;
    let (status, _, _) = world.get("/v1/usage", Some(&secret), None).await;
    assert_eq!(status, StatusCode::OK);
    // A file system may keep the time of a change to the second.
    tokio::time::sleep(Duration::from_millis(1100)).await;
    tokens::save(&world.tokens, &[]).unwrap();
    let (status, _, _) = world.get("/v1/usage", Some(&secret), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn pages_that_list_nothing_say_why() {
    let (world, secret) = World::new(Limits::default()).await;
    let cases = [
        ("/plain", StatusCode::UNPROCESSABLE_ENTITY, "no-items"),
        ("/app", StatusCode::UNPROCESSABLE_ENTITY, "needs-javascript"),
        ("/gone", StatusCode::BAD_GATEWAY, "upstream"),
    ];
    for (path, status, kind) in cases {
        let (got, _, problem) = world.get(&world.feed_of(path), Some(&secret), None).await;
        assert_eq!(
            (got, problem["type"].as_str()),
            (status, Some(kind)),
            "{path}"
        );
    }
    let (_, _, problem) = world
        .get(&world.feed_of("/gone"), Some(&secret), None)
        .await;
    assert_eq!(problem["detail"], "the site answers with 404 Not Found");
}

#[tokio::test]
async fn a_page_that_only_looks_like_a_list_is_guessed() {
    let (world, secret) = World::new(Limits::default()).await;
    let (status, _, feed) = world
        .get(&world.feed_of("/club"), Some(&secret), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{feed}");
    assert_eq!(feed["_rssifier"]["source"], "guess");
    let titles: Vec<&str> = feed["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["title"].as_str().unwrap())
        .collect();
    assert_eq!(
        titles,
        [
            "The summer fair is coming",
            "Our new pitch is open now",
            "Elections for the board"
        ]
    );
    assert_eq!(feed["items"][1]["date_published"], "2026-05-03T00:00:00Z");
    assert_eq!(feed["items"][2]["content_html"], "<p>Vote!</p>");
}

#[tokio::test]
async fn what_robots_txt_or_the_operator_forbids_is_not_fetched() {
    let (world, secret) = World::new(Limits::default()).await;
    let (status, _, problem) = world
        .get(&world.feed_of("/private"), Some(&secret), None)
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(problem["type"], "robots");
    assert_eq!(world.hits.load(Ordering::SeqCst), 0);

    let blocked = Sites::new(Recipes::default(), "127.0.0.1");
    let (world, secret) = World::with(Limits::default(), blocked).await;
    let (status, _, problem) = world
        .get(&world.feed_of("/news"), Some(&secret), None)
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(problem["type"], "blocked-host");
    assert_eq!(world.hits.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn addresses_inside_the_network_and_other_schemes_are_refused() {
    let (world, secret) = World::new(Limits::default()).await;
    for address in [
        "http://127.0.0.2/news",
        "http://10.0.0.1/",
        "file:///etc/passwd",
        "not an address",
    ] {
        let path = format!(
            "/v1/feed?url={}",
            url::form_urlencoded::byte_serialize(address.as_bytes()).collect::<String>()
        );
        let (status, _, problem) = world.get(&path, Some(&secret), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{address}");
        assert_eq!(problem["type"], "address");
    }
}

#[tokio::test]
async fn a_token_over_its_limits_is_told_when_to_come_back() {
    let limits = Limits {
        per_minute: Some(3),
        fetches_per_hour: Some(1),
        at_once: None,
    };
    let (world, secret) = World::new(limits).await;
    let (status, _, _) = world
        .get(&world.feed_of("/news"), Some(&secret), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    // A second page would be a second fetch this hour.
    let (status, headers, problem) = world
        .get(&world.feed_of("/plain"), Some(&secret), None)
        .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(problem["type"], "quota");
    let wait: u64 = headers[header::RETRY_AFTER]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!((3500..=3600).contains(&wait), "{wait}");
    // The cache still answers, until the requests per minute are used up.
    let (status, _, _) = world
        .get(&world.feed_of("/news"), Some(&secret), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = world
        .get(&world.feed_of("/news"), Some(&secret), None)
        .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);

    let (_, _, usage) = world.get("/v1/usage", Some(&secret), None).await;
    assert_eq!(usage["name"], "rueder");
    assert_eq!(usage["limits"]["per_minute"], 3);
    assert_eq!(usage["requests"], 3);
    assert_eq!(usage["fetches"], 1);
    assert_eq!(world.hits.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn the_description_lists_every_route_and_matches_the_file() {
    let (world, _) = World::new(Limits::default()).await;
    let (status, _, served) = world.get("/openapi.json", None, None).await;
    assert_eq!(status, StatusCode::OK);
    let paths: Vec<&String> = served["paths"].as_object().unwrap().keys().collect();
    assert_eq!(paths, ["/health", "/openapi.json", "/v1/feed", "/v1/usage"]);
    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/openapi.json");
    let written = serde_json::to_string_pretty(&served).unwrap() + "\n";
    if std::env::var_os("UPDATE_OPENAPI").is_some() {
        std::fs::write(file, &written).unwrap();
    }
    let kept = std::fs::read_to_string(file).unwrap_or_default();
    assert!(
        kept == written,
        "openapi.json is out of date: run UPDATE_OPENAPI=1 cargo test"
    );
}
