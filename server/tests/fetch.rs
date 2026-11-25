//! Fetching feeds from a site on this machine: what is asked for, what is
//! written down at the feed, what happens when the site fails, and which feeds
//! a round takes.

mod common;

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Instant,
};

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use rueder::{
    accounts,
    cleaning::Rules,
    events::Events,
    feeds::Stored,
    fetch::{self, MAX_INTERVAL, MIN_INTERVAL, Outcome},
    http_client::{self, Config, HttpClient},
    monitor::Monitor,
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
    /// What the site says in `Retry-After`, if anything.
    retry_after: Option<&'static str>,
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
    let mut response = (site.status, [(header::ETAG, etag)], site.body.clone()).into_response();
    if let Some(after) = site.retry_after {
        let after = header::HeaderValue::from_static(after);
        response.headers_mut().insert(header::RETRY_AFTER, after);
    }
    response
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
#[allow(
    clippy::disallowed_methods,
    reason = "a made-up site of the test, not a route of rueder"
)]
async fn site(entries: &[(&str, &str, &str)]) -> (String, Shared) {
    let site = Arc::new(Mutex::new(Site {
        body: rss(entries),
        status: StatusCode::OK,
        retry_after: None,
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
        agent: http_client::AGENT.to_owned(),
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

    let outcome = fetch::fetch_feed(&db, &client, None, &rules(), feed)
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
        fetch::fetch_feed(&db, &client, None, &rules(), feed)
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
    let outcome = fetch::fetch_feed(&db, &client, None, &rules(), feed)
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

    fetch::fetch_feed(&db, &client(), None, &rules(), feed)
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
    let outcome = fetch::fetch_feed(&db, &client, None, &rules(), feed)
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
        fetch::fetch_feed(&db, &client, None, &rules(), feed)
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
    let outcome = fetch::fetch_feed(&db, &client, None, &rules(), feed)
        .await
        .unwrap();
    assert!(
        matches!(&outcome, Outcome::Failed(why) if why.starts_with("not a feed")),
        "{outcome:?}"
    );
    assert_eq!(row(&db, feed).await.get::<i32, _>("last_seq"), 0);

    // The site is back: the error is gone and the interval is short again.
    site.lock().unwrap().body = rss(&[("a", "First", "one")]);
    let outcome = fetch::fetch_feed(&db, &client, None, &rules(), feed)
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
async fn a_site_that_is_asked_too_often_is_asked_less_often_for_a_while(db: PgPool) {
    let (url, site) = site(&[("a", "First", "one")]).await;
    let feed = feed_at(&db, "ada", &url).await;
    let client = client();

    // The site says to come back in three hours, later than twice the interval.
    {
        let mut site = site.lock().unwrap();
        site.status = StatusCode::TOO_MANY_REQUESTS;
        site.retry_after = Some("10800");
    }
    let outcome = fetch::fetch_feed(&db, &client, None, &rules(), feed)
        .await
        .unwrap();
    assert_eq!(
        outcome,
        Outcome::Failed("the site answers with 429 Too Many Requests".to_owned())
    );
    let state = row(&db, feed).await;
    assert_eq!(state.get::<i32, _>("fetch_interval_s"), 7200);
    assert!(state.get::<f64, _>("wait") > 10700.0);

    // It works again, and the feed keeps the longer interval for a while
    // instead of asking as often as the site found too often.
    {
        let mut site = site.lock().unwrap();
        site.status = StatusCode::OK;
        site.retry_after = None;
    }
    let outcome = fetch::fetch_feed(&db, &client, None, &rules(), feed)
        .await
        .unwrap();
    assert!(matches!(outcome, Outcome::Fetched(_)), "{outcome:?}");
    let state = row(&db, feed).await;
    assert_eq!(state.get::<Option<String>, _>("error_message"), None);
    assert_eq!(state.get::<i32, _>("fetch_interval_s"), 7200);

    // After that time the next success shortens it again.
    sqlx::query("UPDATE feeds SET limited_at = limited_at - $2 WHERE id = $1")
        .bind(feed)
        .bind(fetch::LIMITED_FOR)
        .execute(&db)
        .await
        .unwrap();
    site.lock().unwrap().body = rss(&[("a", "First", "one"), ("b", "Second", "two")]);
    fetch::fetch_feed(&db, &client, None, &rules(), feed)
        .await
        .unwrap();
    assert_eq!(row(&db, feed).await.get::<i32, _>("fetch_interval_s"), 3600);
}

#[sqlx::test]
async fn a_feed_is_never_fetched_more_often_than_an_admin_said(db: PgPool) {
    let (url, site) = site(&[("a", "First", "one")]).await;
    let feed = feed_at(&db, "ada", &url).await;
    let client = client();
    // A feed that publishes enough for the shortest interval, and an admin
    // who said six hours at least.
    sqlx::query(
        "UPDATE feeds SET created_at = now() - interval '2 days', last_seq = 100,
             min_interval_s = 21600
         WHERE id = $1",
    )
    .bind(feed)
    .execute(&db)
    .await
    .unwrap();

    // Neither a success nor a site that says nothing changed makes it shorter.
    let outcome = fetch::fetch_feed(&db, &client, None, &rules(), feed)
        .await
        .unwrap();
    assert!(matches!(outcome, Outcome::Fetched(_)), "{outcome:?}");
    let state = row(&db, feed).await;
    assert_eq!(state.get::<i32, _>("fetch_interval_s"), 21600);
    assert!(state.get::<f64, _>("wait") > 21590.0);
    let outcome = fetch::fetch_feed(&db, &client, None, &rules(), feed)
        .await
        .unwrap();
    assert_eq!(outcome, Outcome::NotModified);
    assert!(row(&db, feed).await.get::<f64, _>("wait") > 21590.0);

    // Without it, the next success goes by what the feed publishes again.
    sqlx::query("UPDATE feeds SET min_interval_s = NULL WHERE id = $1")
        .bind(feed)
        .execute(&db)
        .await
        .unwrap();
    site.lock().unwrap().body = rss(&[("a", "First", "one"), ("b", "Second", "two")]);
    fetch::fetch_feed(&db, &client, None, &rules(), feed)
        .await
        .unwrap();
    assert_eq!(
        row(&db, feed).await.get::<i32, _>("fetch_interval_s"),
        MIN_INTERVAL
    );
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
        let outcome = fetch::fetch_feed(&db, &client, None, &rules(), feed)
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
    fetch::fetch_feed(&db, &client(), None, &rules(), feed)
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
    let monitor = Arc::new(Monitor::default());
    assert_eq!(
        fetch::fetch_due(
            &db,
            &client,
            None,
            &rules(),
            &Events::new(),
            &monitor,
            &fetch::Pace::default()
        )
        .await
        .unwrap(),
        2
    );
    assert_eq!(site.lock().unwrap().requests, 2);
    // The status page hears of the round and of its fetches.
    let fetches = monitor.fetches(Instant::now());
    assert_eq!((fetches.fetched, fetches.new_articles), (2, 2));
    assert_eq!(fetches.timing.count, 2);
    assert_eq!(monitor.last_round().map(|round| round.feeds), Some(2));
    for (feed, articles) in [(due, 1), (also_due, 1), (later, 0), (unread, 0)] {
        assert_eq!(
            row(&db, feed).await.get::<i32, _>("last_seq"),
            articles,
            "feed {feed}"
        );
    }
    // Fetched feeds are not due any more.
    assert_eq!(
        fetch::fetch_due(
            &db,
            &client,
            None,
            &rules(),
            &Events::new(),
            &monitor,
            &fetch::Pace::default()
        )
        .await
        .unwrap(),
        0
    );
    assert_eq!(site.lock().unwrap().requests, 2);
}

#[sqlx::test]
async fn a_feed_only_disabled_accounts_read_rests_until_somebody_reads_it(db: PgPool) {
    let (url, site) = site(&[("a", "First", "one")]).await;
    let alone = feed_at(&db, "ada", &url).await;
    let shared = feed_at(&db, "grace", &url).await;
    let ada: i32 = sqlx::query_scalar("SELECT id FROM accounts WHERE username = 'ada'")
        .fetch_one(&db)
        .await
        .unwrap();
    common::subscribe(&db, ada, shared).await;
    // A blocked feed is not fetched anyway, so it does not count.
    let blocked = common::feed(&db, "blocked").await;
    common::subscribe(&db, ada, blocked).await;
    sqlx::query("UPDATE feeds SET blocked_at = now() WHERE id = $1")
        .bind(blocked)
        .execute(&db)
        .await
        .unwrap();
    let (client, rules, events) = (client(), rules(), Events::new());
    let monitor = Arc::new(Monitor::default());
    let pace = fetch::Pace::default();
    let round = || fetch::fetch_due(&db, &client, None, &rules, &events, &monitor, &pace);

    // Disabled, ada reads nothing: only the feed grace reads too is fetched.
    assert_eq!(
        accounts::set_disabled(&db, "ada", true).await.unwrap(),
        Some(1)
    );
    assert_eq!(round().await.unwrap(), 1);
    assert_eq!(row(&db, shared).await.get::<i32, _>("last_seq"), 1);
    assert_eq!(row(&db, alone).await.get::<i32, _>("last_seq"), 0);
    assert_eq!(round().await.unwrap(), 0);

    // Enabled again, the feed comes back with the next round.
    assert_eq!(
        accounts::set_disabled(&db, "ada", false).await.unwrap(),
        Some(1)
    );
    assert_eq!(round().await.unwrap(), 1);
    assert_eq!(row(&db, alone).await.get::<i32, _>("last_seq"), 1);

    // So it does when somebody else subscribes.
    sqlx::query("UPDATE feeds SET next_fetch_at = now() WHERE id = $1")
        .bind(alone)
        .execute(&db)
        .await
        .unwrap();
    accounts::set_disabled(&db, "ada", true).await.unwrap();
    assert_eq!(round().await.unwrap(), 0);
    let linus = common::account(&db, "linus").await;
    common::subscribe(&db, linus, alone).await;
    assert_eq!(round().await.unwrap(), 1);
    // And then ada is not alone with any feed.
    assert_eq!(
        accounts::set_disabled(&db, "ada", true).await.unwrap(),
        Some(0)
    );
    assert_eq!(site.lock().unwrap().requests, 3);
    assert_eq!(
        accounts::set_disabled(&db, "nobody", true).await.unwrap(),
        None
    );
}

#[sqlx::test]
async fn a_round_takes_no_more_than_the_pace_says(db: PgPool) {
    let (url, site) = site(&[("a", "First", "one")]).await;
    for name in ["ada", "grace", "linus"] {
        feed_at(&db, name, &url).await;
    }
    let pace = fetch::Pace {
        round: 2,
        at_once: 1,
        ..fetch::Pace::default()
    };
    let (client, rules, events) = (client(), rules(), Events::new());
    let monitor = Arc::new(Monitor::default());
    for taken in [2, 1, 0] {
        let round = fetch::fetch_due(&db, &client, None, &rules, &events, &monitor, &pace);
        assert_eq!(round.await.unwrap(), taken);
    }
    assert_eq!(site.lock().unwrap().requests, 3);
}

#[test]
fn the_pace_comes_from_the_environment_within_bounds() {
    let vars = |pairs: &[(&str, &str)]| {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect();
        fetch::Pace::from_vars(move |name| {
            pairs
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.clone())
        })
    };
    // As rueder comes: 100 feeds every 30 seconds, 8 at once, 12,000 an hour.
    let pace = vars(&[]).unwrap();
    assert_eq!(pace, fetch::Pace::default());
    assert_eq!(
        (
            pace.round,
            pace.at_once,
            pace.every.as_secs(),
            pace.per_hour()
        ),
        (100, 8, 30, 12_000)
    );
    let large = vars(&[
        ("FETCH_ROUND", "1000"),
        ("FETCH_AT_ONCE", " 64 "),
        ("FETCH_EVERY", "10"),
    ])
    .unwrap();
    assert_eq!(
        (
            large.round,
            large.at_once,
            large.every.as_secs(),
            large.per_hour()
        ),
        (1000, 64, 10, 360_000)
    );
    // Only what is set changes; one of a host at a time, unless it is set.
    assert_eq!(vars(&[("FETCH_AT_ONCE", "16")]).unwrap().round, 100);
    assert_eq!(pace.per_host, 1);
    assert_eq!(vars(&[("FETCH_PER_HOST", "4")]).unwrap().per_host, 4);
    // At the bounds, and beyond them.
    for fine in [
        ("FETCH_ROUND", "1"),
        ("FETCH_ROUND", "10000"),
        ("FETCH_AT_ONCE", "1"),
        ("FETCH_AT_ONCE", "256"),
        ("FETCH_EVERY", "5"),
        ("FETCH_EVERY", "3600"),
        ("FETCH_PER_HOST", "1"),
        ("FETCH_PER_HOST", "32"),
    ] {
        assert!(vars(&[fine]).is_ok(), "{fine:?}");
    }
    for wrong in [
        ("FETCH_ROUND", "0"),
        ("FETCH_ROUND", "10001"),
        ("FETCH_ROUND", "-5"),
        ("FETCH_AT_ONCE", "0"),
        ("FETCH_AT_ONCE", "257"),
        ("FETCH_EVERY", "4"),
        ("FETCH_EVERY", "3601"),
        ("FETCH_EVERY", "30s"),
        ("FETCH_ROUND", ""),
        ("FETCH_AT_ONCE", "99999999999999999999999"),
        ("FETCH_PER_HOST", "0"),
        ("FETCH_PER_HOST", "33"),
    ] {
        let error = vars(&[wrong]).unwrap_err().to_string();
        assert!(error.contains(wrong.0), "{wrong:?}: {error}");
    }
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

/// A site that takes its time with every answer, under an address of its own
/// out of 127.0.0.0/8, and what it saw: how often each feed was asked for and
/// how many requests it had at once at most.
#[derive(Default)]
struct Slow {
    asked: std::collections::HashMap<u32, usize>,
    now: usize,
    most: usize,
}

#[allow(
    clippy::disallowed_methods,
    reason = "a made-up site of the test, not a route of rueder"
)]
async fn slow_site(ip: &str, delay: std::time::Duration) -> (String, Arc<Mutex<Slow>>) {
    let seen = Arc::new(Mutex::new(Slow::default()));
    let kept = seen.clone();
    let router = Router::new().route(
        "/feed/{n}",
        get(move |axum::extract::Path(n): axum::extract::Path<u32>| {
            let seen = kept.clone();
            async move {
                {
                    let mut seen = seen.lock().unwrap();
                    *seen.asked.entry(n).or_default() += 1;
                    seen.now += 1;
                    seen.most = seen.most.max(seen.now);
                }
                tokio::time::sleep(delay).await;
                seen.lock().unwrap().now -= 1;
                rss(&[(&format!("feed-{n}"), "Slow news", "Taken slowly.")])
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind(format!("{ip}:0"))
        .await
        .unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (format!("http://{addr}"), seen)
}

/// Feeds at these addresses, all read by one account.
async fn feeds_at(db: &PgPool, urls: &[String]) -> Vec<i32> {
    let reader = common::account(db, &format!("reader-{}", urls.len())).await;
    let mut ids = Vec::new();
    for url in urls {
        let feed = common::feed(db, url).await;
        sqlx::query("UPDATE feeds SET feed_url = $2 WHERE id = $1")
            .bind(feed)
            .bind(url)
            .execute(db)
            .await
            .unwrap();
        common::subscribe(db, reader, feed).await;
        ids.push(feed);
    }
    ids
}

/// A client for sites anywhere on this machine.
fn loopback_client() -> HttpClient {
    let config = Config {
        agent: http_client::AGENT.to_owned(),
        allow: vec!["127.0.0.0/8".parse().unwrap()],
        ..Config::default()
    };
    HttpClient::new(config).unwrap()
}

/// While a round still works through a slow host, the next one leaves that
/// host out and fetches the others at once; no feed is fetched twice.
#[sqlx::test]
async fn a_slow_host_holds_up_only_itself(db: PgPool) {
    let delay = std::time::Duration::from_millis(400);
    let (slow, slow_seen) = slow_site("127.0.0.2", delay).await;
    let (fast, fast_seen) = slow_site("127.0.0.1", std::time::Duration::ZERO).await;
    let urls: Vec<String> = (0..5)
        .map(|n| format!("{slow}/feed/{n}"))
        .chain((0..2).map(|n| format!("{fast}/feed/{n}")))
        .collect();
    feeds_at(&db, &urls).await;
    let pace = fetch::Pace {
        round: 20,
        at_once: 8,
        ..fetch::Pace::default()
    };
    let underway = fetch::Underway::new(&pace);
    let (client, rules, events) = (loopback_client(), rules(), Events::new());
    let monitor = Arc::new(Monitor::default());
    let first = {
        let (db, client, rules, events, monitor, underway) = (
            db.clone(),
            client.clone(),
            rules.clone(),
            events.clone(),
            monitor.clone(),
            underway.clone(),
        );
        tokio::spawn(async move {
            fetch::round(
                &db, &client, None, &rules, &events, &monitor, &pace, &underway,
            )
            .await
        })
    };
    // The first round is done with the fast host and still at the slow one;
    // everything is due again.
    while underway.busy() != ["127.0.0.2"] {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    sqlx::query("UPDATE feeds SET next_fetch_at = now()")
        .execute(&db)
        .await
        .unwrap();
    let started = Instant::now();
    let second = fetch::round(
        &db, &client, None, &rules, &events, &monitor, &pace, &underway,
    )
    .await
    .unwrap();
    assert_eq!(second, 2, "only the feeds of the fast host");
    assert!(started.elapsed() < delay * 2, "{:?}", started.elapsed());
    assert!(!first.is_finished(), "the slow host is still at it");
    assert_eq!(first.await.unwrap().unwrap(), 7);
    assert!(underway.busy().is_empty());

    let slow_seen = slow_seen.lock().unwrap();
    assert_eq!(slow_seen.most, 1, "one of a host at a time");
    assert!(
        slow_seen.asked.values().all(|&n| n == 1),
        "{:?}",
        slow_seen.asked
    );
    let fast_seen = fast_seen.lock().unwrap();
    assert!(
        fast_seen.asked.values().all(|&n| n == 2),
        "{:?}",
        fast_seen.asked
    );
}

/// `per_host` feeds of one host are fetched at once, never more.
#[sqlx::test]
async fn no_more_of_a_host_at_once_than_the_pace_says(db: PgPool) {
    let (site, seen) = slow_site("127.0.0.3", std::time::Duration::from_millis(150)).await;
    let urls: Vec<String> = (0..7).map(|n| format!("{site}/feed/{n}")).collect();
    feeds_at(&db, &urls).await;
    let (client, rules, events) = (loopback_client(), rules(), Events::new());
    let monitor = Arc::new(Monitor::default());
    for per_host in [1, 3] {
        sqlx::query("UPDATE feeds SET next_fetch_at = now()")
            .execute(&db)
            .await
            .unwrap();
        seen.lock().unwrap().most = 0;
        let pace = fetch::Pace {
            per_host,
            ..fetch::Pace::default()
        };
        let fetched = fetch::fetch_due(&db, &client, None, &rules, &events, &monitor, &pace).await;
        assert_eq!(fetched.unwrap(), 7);
        assert_eq!(seen.lock().unwrap().most, per_host);
    }
}
