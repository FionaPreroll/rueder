//! Fetching feeds from a site on this machine: what is asked for, what is
//! written down at the feed, what happens when the site fails, and which feeds
//! a round takes.

mod common;

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use rueder::{
    cleaning::Rules,
    events::Events,
    feeds::Stored,
    fetch::{self, MAX_INTERVAL, MIN_INTERVAL, Outcome},
    http_client::{Config, HttpClient},
};
use sqlx::{PgPool, Row};
use time::Duration;

fn rules() -> Arc<Rules> {
    Arc::new(Rules::built_in().unwrap())
}

/// A site with one feed, and what it saw.
struct Site {
    body: String,
    status: StatusCode,
    requests: usize,
    /// Requests that said which copy the caller has.
    conditional: usize,
    agents: Vec<String>,
}

type Shared = Arc<Mutex<Site>>;

async fn feed_xml(State(site): State<Shared>, headers: HeaderMap) -> Response {
    let mut site = site.lock().unwrap();
    site.requests += 1;
    let said = |name| {
        headers
            .get(name)
            .and_then(|value: &header::HeaderValue| value.to_str().ok())
            .unwrap_or_default()
            .to_owned()
    };
    site.agents.push(said(header::USER_AGENT));
    let etag = format!("\"{}\"", site.body.len());
    if !said(header::IF_NONE_MATCH).is_empty() {
        site.conditional += 1;
        if said(header::IF_NONE_MATCH) == etag && site.status == StatusCode::OK {
            return StatusCode::NOT_MODIFIED.into_response();
        }
    }
    (site.status, [(header::ETAG, etag)], site.body.clone()).into_response()
}

fn rss(entries: &[(&str, &str, &str)]) -> String {
    let items: String = entries
        .iter()
        .map(|(id, title, text)| format!("<item><guid>{id}</guid><title>{title}</title><description>{text}</description></item>"))
        .collect();
    format!(
        "<?xml version=\"1.0\"?><rss version=\"2.0\"><channel><title>The Example</title>\
         <link>https://example.org/</link>{items}</channel></rss>"
    )
}

/// Starts the site and returns the address of its feed.
async fn site(entries: &[(&str, &str, &str)]) -> (String, Shared) {
    let site = Arc::new(Mutex::new(Site {
        body: rss(entries),
        status: StatusCode::OK,
        requests: 0,
        conditional: 0,
        agents: Vec::new(),
    }));
    let router = Router::new()
        .route("/feed.xml", get(feed_xml))
        .with_state(site.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (format!("http://{addr}/feed.xml"), site)
}

/// A client that may reach the site on this machine and nothing else inside.
fn client() -> HttpClient {
    let config = Config {
        allow: vec!["127.0.0.1".parse().unwrap()],
        ..Config::default()
    };
    HttpClient::new(config).unwrap()
}

/// A feed with a reader, at the address.
async fn feed_at(db: &PgPool, reader: &str, url: &str) -> i32 {
    let (_, feed) = common::reader(db, reader, 0).await;
    sqlx::query("UPDATE feeds SET feed_url = $2 WHERE id = $1")
        .bind(feed)
        .bind(url)
        .execute(db)
        .await
        .unwrap();
    feed
}

async fn row(db: &PgPool, feed: i32) -> sqlx::postgres::PgRow {
    sqlx::query(
        "SELECT title, site_url, etag, error_message, fetch_interval_s, last_seq,
                last_success_at IS NOT NULL AS worked, last_error_at IS NOT NULL AS failed,
                extract(epoch FROM next_fetch_at - now())::float8 AS wait
         FROM feeds WHERE id = $1",
    )
    .bind(feed)
    .fetch_one(db)
    .await
    .unwrap()
}

#[sqlx::test]
async fn a_fetch_stores_the_articles_and_the_next_one_asks_whether_anything_changed(db: PgPool) {
    let (url, site) = site(&[("b", "Second", "two"), ("a", "First", "one")]).await;
    let feed = feed_at(&db, "ada", &url).await;
    let client = client();

    let outcome = fetch::fetch_feed(&db, &client, &rules(), feed)
        .await
        .unwrap();
    assert_eq!(
        outcome,
        Outcome::Fetched(Stored {
            new: 2,
            updated: 0,
            first_new: Some(1)
        })
    );
    let state = row(&db, feed).await;
    assert_eq!(state.get::<String, _>("title"), "The Example");
    assert_eq!(
        state.get::<Option<String>, _>("site_url").as_deref(),
        Some("https://example.org/")
    );
    assert!(state.get::<Option<String>, _>("etag").is_some());
    assert_eq!(state.get::<Option<String>, _>("error_message"), None);
    assert!(state.get::<bool, _>("worked"));
    assert!(!state.get::<bool, _>("failed"));
    assert_eq!(state.get::<i32, _>("last_seq"), 2);
    // The next fetch is due in about the feed's interval.
    let wait = state.get::<f64, _>("wait");
    assert!((3500.0..3800.0).contains(&wait), "{wait}");
    assert!(site.lock().unwrap().agents[0].starts_with("rueder/"));

    // Nothing changed: the site says so and sends no feed.
    assert_eq!(
        fetch::fetch_feed(&db, &client, &rules(), feed)
            .await
            .unwrap(),
        Outcome::NotModified
    );
    assert_eq!(site.lock().unwrap().conditional, 1);
    assert_eq!(row(&db, feed).await.get::<i32, _>("last_seq"), 2);

    // A new article and a corrected one.
    site.lock().unwrap().body = rss(&[
        ("c", "Third", "three"),
        ("b", "Second", "two, corrected"),
        ("a", "First", "one"),
    ]);
    let outcome = fetch::fetch_feed(&db, &client, &rules(), feed)
        .await
        .unwrap();
    assert_eq!(
        outcome,
        Outcome::Fetched(Stored {
            new: 1,
            updated: 1,
            first_new: Some(3)
        })
    );
}

#[sqlx::test]
async fn the_readers_rules_see_what_a_fetch_brings(db: PgPool) {
    let (url, _site) = site(&[("b", "Second", "two"), ("a", "First", "one")]).await;
    let feed = feed_at(&db, "ada", &url).await;
    let grace = common::account(&db, "grace").await;
    common::subscribe(&db, grace, feed).await;
    sqlx::query(
        "INSERT INTO filter_rules (account_id, position, field, match, pattern, action)
         SELECT id, 1, 'title', 'word', 'second', 'hide' FROM accounts WHERE username = 'ada'",
    )
    .execute(&db)
    .await
    .unwrap();

    fetch::fetch_feed(&db, &client(), &rules(), feed)
        .await
        .unwrap();
    // The article is hidden for Ada from the moment it is there, and for
    // nobody else.
    let hidden: Vec<(String, String)> = sqlx::query_as(
        "SELECT c.username, a.title FROM article_states st
         JOIN accounts c ON c.id = st.account_id
         JOIN articles a ON a.feed_id = st.feed_id AND a.seq = st.seq
         WHERE st.hidden",
    )
    .fetch_all(&db)
    .await
    .unwrap();
    assert_eq!(hidden, [("ada".to_owned(), "Second".to_owned())]);
}

#[sqlx::test]
async fn a_failure_stands_at_the_feed_and_makes_the_next_try_come_later(db: PgPool) {
    let (url, site) = site(&[("a", "First", "one")]).await;
    let feed = feed_at(&db, "ada", &url).await;
    let client = client();

    site.lock().unwrap().status = StatusCode::NOT_FOUND;
    let outcome = fetch::fetch_feed(&db, &client, &rules(), feed)
        .await
        .unwrap();
    assert_eq!(
        outcome,
        Outcome::Failed("the site answers with 404 Not Found".to_owned())
    );
    let state = row(&db, feed).await;
    assert_eq!(
        state.get::<Option<String>, _>("error_message").as_deref(),
        Some("the site answers with 404 Not Found")
    );
    assert!(state.get::<bool, _>("failed"));
    assert_eq!(state.get::<i32, _>("fetch_interval_s"), 7200);
    assert!(state.get::<f64, _>("wait") > 7000.0);

    // Failing again doubles the wait, up to the longest interval.
    for _ in 0..5 {
        fetch::fetch_feed(&db, &client, &rules(), feed)
            .await
            .unwrap();
    }
    assert_eq!(
        row(&db, feed).await.get::<i32, _>("fetch_interval_s"),
        MAX_INTERVAL
    );

    // What comes back is not a feed.
    {
        let mut site = site.lock().unwrap();
        site.status = StatusCode::OK;
        site.body = "<html><body>Under construction</body></html>".to_owned();
    }
    let outcome = fetch::fetch_feed(&db, &client, &rules(), feed)
        .await
        .unwrap();
    assert!(
        matches!(&outcome, Outcome::Failed(why) if why.starts_with("not a feed")),
        "{outcome:?}"
    );
    assert_eq!(row(&db, feed).await.get::<i32, _>("last_seq"), 0);

    // The site is back: the error is gone and the interval is short again.
    site.lock().unwrap().body = rss(&[("a", "First", "one")]);
    let outcome = fetch::fetch_feed(&db, &client, &rules(), feed)
        .await
        .unwrap();
    assert!(
        matches!(outcome, Outcome::Fetched(Stored { new: 1, .. })),
        "{outcome:?}"
    );
    let state = row(&db, feed).await;
    assert_eq!(state.get::<Option<String>, _>("error_message"), None);
    assert_eq!(state.get::<i32, _>("fetch_interval_s"), 3600);
}

#[sqlx::test]
async fn a_feed_inside_the_own_network_is_not_fetched(db: PgPool) {
    let client = client();
    for url in [
        "http://127.0.0.2:9/feed.xml",
        "http://169.254.169.254/latest/meta-data/",
        "file:///etc/passwd",
        "not an address",
    ] {
        let feed = feed_at(&db, &format!("reader of {url}"), url).await;
        let outcome = fetch::fetch_feed(&db, &client, &rules(), feed)
            .await
            .unwrap();
        assert!(matches!(outcome, Outcome::Failed(_)), "{url}: {outcome:?}");
        let message = row(&db, feed)
            .await
            .get::<Option<String>, _>("error_message")
            .unwrap();
        assert!(
            message.contains("public internet") || message.contains("not an address"),
            "{url}: {message}"
        );
    }
}

#[sqlx::test]
async fn a_feed_can_be_fetched_as_a_browser(db: PgPool) {
    let (url, site) = site(&[("a", "First", "one")]).await;
    let feed = feed_at(&db, "ada", &url).await;
    sqlx::query("UPDATE feeds SET fetch_agent = 'browser' WHERE id = $1")
        .bind(feed)
        .execute(&db)
        .await
        .unwrap();
    fetch::fetch_feed(&db, &client(), &rules(), feed)
        .await
        .unwrap();
    assert!(site.lock().unwrap().agents[0].starts_with("Mozilla/5.0"));
}

#[sqlx::test]
async fn a_round_takes_the_feeds_that_are_due_and_have_a_reader(db: PgPool) {
    let (url, site) = site(&[("a", "First", "one")]).await;
    let due = feed_at(&db, "ada", &url).await;
    let also_due = feed_at(&db, "grace", &url).await;
    let later = feed_at(&db, "linus", &url).await;
    sqlx::query("UPDATE feeds SET next_fetch_at = now() + interval '1 hour' WHERE id = $1")
        .bind(later)
        .execute(&db)
        .await
        .unwrap();
    // Nobody reads this one.
    let unread = common::feed(&db, "nobody's").await;
    sqlx::query("UPDATE feeds SET feed_url = $2 WHERE id = $1")
        .bind(unread)
        .bind(&url)
        .execute(&db)
        .await
        .unwrap();

    let client = client();
    assert_eq!(
        fetch::fetch_due(&db, &client, &rules(), &Events::new())
            .await
            .unwrap(),
        2
    );
    assert_eq!(site.lock().unwrap().requests, 2);
    for (feed, articles) in [(due, 1), (also_due, 1), (later, 0), (unread, 0)] {
        assert_eq!(
            row(&db, feed).await.get::<i32, _>("last_seq"),
            articles,
            "feed {feed}"
        );
    }
    // Fetched feeds are not due any more.
    assert_eq!(
        fetch::fetch_due(&db, &client, &rules(), &Events::new())
            .await
            .unwrap(),
        0
    );
    assert_eq!(site.lock().unwrap().requests, 2);
}

#[test]
fn feeds_that_publish_more_are_fetched_more_often() {
    let days = Duration::days;
    // In its first day a feed keeps the interval it has.
    assert_eq!(fetch::interval(50, Duration::hours(23)), None);
    assert_eq!(fetch::interval(0, days(30)), Some(MAX_INTERVAL));
    assert_eq!(fetch::interval(30 * 20, days(30)), Some(MIN_INTERVAL));
    assert_eq!(fetch::interval(30 * 15, days(30)), Some(MIN_INTERVAL));
    assert_eq!(fetch::interval(30 * 10, days(30)), Some(2 * MIN_INTERVAL));
    assert_eq!(fetch::interval(30 * 6, days(30)), Some(3 * MIN_INTERVAL));
    assert_eq!(fetch::interval(30 * 2, days(30)), Some(4 * MIN_INTERVAL));
    assert_eq!(fetch::interval(1, days(3650)), Some(4 * MIN_INTERVAL));
    for articles in [0, 1, 100, 100_000] {
        for age in [days(1), days(2), days(400)] {
            let interval = fetch::interval(articles, age).unwrap();
            assert!((MIN_INTERVAL..=MAX_INTERVAL).contains(&interval));
        }
    }
}
