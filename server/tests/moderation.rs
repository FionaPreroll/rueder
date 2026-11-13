//! Reports of feeds and articles by their readers, and blocking a feed.

mod common;

use std::{sync::Arc, time::Duration};

use axum::http::{Method, StatusCode};
use common::call;
use rueder::{
    AppState, accounts,
    events::Events,
    fetch,
    http_client::{self, HttpClient},
    images::{Cache, Config, Format, ImageProxy, Picture, Refused, Size},
    moderation,
    monitor::Monitor,
    rate::{Rate, Rates},
};
use serde_json::{Value, json};
use sqlx::PgPool;

fn reporting(db: &PgPool) -> AppState {
    AppState::new(db.clone()).with_content_reports()
}

async fn report(state: &AppState, cookie: &str, path: &str, body: Value) -> StatusCode {
    call(state, Method::POST, path, Some(cookie), Some(body))
        .await
        .0
}

#[sqlx::test]
async fn readers_report_feeds_and_articles_once_each(db: PgPool) {
    let (ada, feed) = common::reader(&db, "ada", 3).await;
    let cookie = common::session(&db, ada).await;
    let feed_path = format!("/api/feeds/{feed}/report");
    let article_path = format!("/api/feeds/{feed}/articles/2/report");
    let spam = json!({ "reason": "spam" });

    // Without CONTENT_REPORTS=on there is no such thing, and the page knows.
    let plain = AppState::new(db.clone());
    assert_eq!(
        report(&plain, &cookie, &feed_path, spam.clone()).await,
        StatusCode::NOT_FOUND
    );
    let (_, me) = call(&plain, Method::GET, "/api/me", Some(&cookie), None).await;
    assert_eq!(me["content_reports"], false);

    let state = reporting(&db);
    let (_, me) = call(&state, Method::GET, "/api/me", Some(&cookie), None).await;
    assert_eq!(me["content_reports"], true);
    assert_eq!(
        report(&state, &cookie, &feed_path, spam.clone()).await,
        StatusCode::NO_CONTENT
    );
    let note = json!({ "reason": "illegal", "note": "  a threat\u{7}, see the second  " });
    assert_eq!(
        report(&state, &cookie, &article_path, note).await,
        StatusCode::NO_CONTENT
    );
    // A second report of the same replaces the first.
    let other = json!({ "reason": "copyright" });
    assert_eq!(
        report(&state, &cookie, &feed_path, other).await,
        StatusCode::NO_CONTENT
    );
    let kept: Vec<(Option<i32>, String, String)> =
        sqlx::query_as("SELECT seq, reason, note FROM content_reports ORDER BY seq NULLS FIRST")
            .fetch_all(&db)
            .await
            .unwrap();
    assert_eq!(
        kept,
        [
            (None, "copyright".into(), String::new()),
            (Some(2), "illegal".into(), "a threat, see the second".into())
        ]
    );

    // What is no reason, a note too long, an article there is not.
    let wrong = json!({ "reason": "boring" });
    assert_eq!(
        report(&state, &cookie, &feed_path, wrong).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let long = json!({ "reason": "other", "note": "x".repeat(moderation::MAX_NOTE + 1) });
    assert_eq!(
        report(&state, &cookie, &feed_path, long).await,
        StatusCode::BAD_REQUEST
    );
    let missing = format!("/api/feeds/{feed}/articles/99/report");
    assert_eq!(
        report(&state, &cookie, &missing, spam.clone()).await,
        StatusCode::NOT_FOUND
    );

    // Only readers of a feed report it, and nobody without a session.
    let grace = common::account(&db, "grace").await;
    let theirs = common::session(&db, grace).await;
    assert_eq!(
        report(&state, &theirs, &feed_path, spam.clone()).await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        report(&state, &theirs, &article_path, spam.clone()).await,
        StatusCode::NOT_FOUND
    );
    let (status, _) = call(&state, Method::POST, &feed_path, None, Some(spam)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn an_account_reports_within_its_limit(db: PgPool) {
    let (ada, feed) = common::reader(&db, "ada", 5).await;
    let cookie = common::session(&db, ada).await;
    let state = reporting(&db)
        .with_rates(Rates::default().with_content_reports(Rate::new(2, Duration::from_secs(3600))));
    let mut answers = Vec::new();
    for seq in 1..=3 {
        let path = format!("/api/feeds/{feed}/articles/{seq}/report");
        answers.push(report(&state, &cookie, &path, json!({ "reason": "spam" })).await);
    }
    assert_eq!(
        answers,
        [
            StatusCode::NO_CONTENT,
            StatusCode::NO_CONTENT,
            StatusCode::TOO_MANY_REQUESTS
        ]
    );
}

#[sqlx::test]
async fn admins_see_reported_feeds_by_address_never_who_reported(db: PgPool) {
    let (ada, feed) = common::reader(&db, "ada-secret", 2).await;
    let grace = common::account(&db, "grace-secret").await;
    common::subscribe(&db, grace, feed).await;
    let root = common::account(&db, "root").await;
    accounts::set_admin(&db, "root", true).await.unwrap();
    let state = reporting(&db);
    for (account, reason) in [(ada, "spam"), (grace, "illegal")] {
        let cookie = common::session(&db, account).await;
        let path = format!("/api/feeds/{feed}/articles/1/report");
        assert_eq!(
            report(
                &state,
                &cookie,
                &path,
                json!({ "reason": reason, "note": "hidden note" })
            )
            .await,
            StatusCode::NO_CONTENT
        );
    }
    let admin = common::session(&db, root).await;
    let (_, status) = call(&state, Method::GET, "/api/status", Some(&admin), None).await;
    let reported = &status["reported"][0];
    assert_eq!(reported["url"], "https://ada-secret.example/feed.xml");
    assert_eq!(reported["reports"], 2);
    assert_eq!(reported["blocked"], false);
    assert_eq!(reported["reasons"].as_array().unwrap().len(), 2);
    let hint = status["hints"]
        .as_array()
        .unwrap()
        .iter()
        .find(|hint| hint["kind"] == "reported_feeds")
        .unwrap();
    assert_eq!(hint["feeds"], 1);
    // Neither who reported nor what they wrote.
    let text = status["reported"].to_string();
    for secret in ["grace", "hidden note", &ada.to_string()] {
        assert!(
            !text.contains(&format!("\"{secret}\"")),
            "{secret} in {text}"
        );
    }
    assert!(
        !text.contains("grace-secret") && !text.contains("hidden note"),
        "{text}"
    );

    // Blocked, it is looked after: no hint any more.
    moderation::block(&db, "https://ada-secret.example/feed.xml")
        .await
        .unwrap();
    let (_, status) = call(&state, Method::GET, "/api/status", Some(&admin), None).await;
    assert_eq!(status["reported"][0]["blocked"], true);
    assert!(!status["hints"].to_string().contains("reported_feeds"));
}

#[sqlx::test]
async fn a_blocked_feed_is_not_fetched_seen_or_subscribed_to(db: PgPool) {
    let (ada, feed) = common::reader(&db, "ada", 3).await;
    let (_, other) = common::reader(&db, "linus", 1).await;
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;
    // The other feed fails at once when it is fetched: the client refuses it.
    sqlx::query("UPDATE feeds SET feed_url = 'http://127.0.0.1:9/feed.xml' WHERE id = $1")
        .bind(other)
        .execute(&db)
        .await
        .unwrap();
    let star = json!({ "starred": true });
    let (status, _) = call(
        &state,
        Method::PUT,
        &format!("/api/feeds/{feed}/articles/1/state"),
        Some(&cookie),
        Some(star),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let get = |path: String| {
        let (state, cookie) = (state.clone(), cookie.clone());
        async move { call(&state, Method::GET, &path, Some(&cookie), None).await }
    };
    let titles = |list: Value| -> usize { list["articles"].as_array().map_or(0, Vec::len) };

    assert!(
        moderation::block(&db, "https://nowhere.example/")
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        moderation::block(&db, " https://ada.example/feed.xml ")
            .await
            .unwrap(),
        [feed]
    );

    // Nobody sees its articles: not in the tree's counts, not in any list,
    // not when opened, not in a search.
    let (_, tree) = get("/api/tree".into()).await;
    let shown = tree["feeds"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == feed)
        .unwrap()
        .clone();
    assert_eq!(
        (&shown["blocked"], &shown["unread"]),
        (&json!(true), &json!(0))
    );
    assert_eq!(titles(get("/api/articles".into()).await.1), 0);
    assert_eq!(titles(get("/api/starred".into()).await.1), 0);
    // Its own list is empty for its reader, who still has it in the tree.
    let (status, list) = get(format!("/api/feeds/{feed}/articles")).await;
    assert_eq!((status, titles(list)), (StatusCode::OK, 0));
    let (status, hits) = get(format!("/api/search?q=Article&feed={feed}")).await;
    assert_eq!((status, titles(hits)), (StatusCode::OK, 0));
    assert_eq!(
        get(format!("/api/feeds/{feed}/articles/1")).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        get(format!("/api/feeds/{feed}/articles/1/versions"))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(titles(get("/api/search?q=Article".into()).await.1), 0);
    // Marking it read marks nothing that is hidden.
    let (status, _) = call(
        &state,
        Method::POST,
        &format!("/api/feeds/{feed}/read"),
        Some(&cookie),
        Some(json!({ "upto": 3 })),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // It is not fetched.
    sqlx::query("UPDATE feeds SET next_fetch_at = now() - interval '1 hour'")
        .execute(&db)
        .await
        .unwrap();
    let client = rueder::http_client::HttpClient::new(Default::default()).unwrap();
    let rules = Arc::new(rueder::cleaning::Rules::built_in().unwrap());
    let monitor = Arc::new(Monitor::default());
    let pace = fetch::Pace::default();
    // The other feed is due too; it is the only one taken.
    fetch::fetch_due(&db, &client, &rules, &Events::new(), &monitor, &pace)
        .await
        .unwrap();
    let due: Vec<i32> =
        sqlx::query_scalar("SELECT id FROM feeds WHERE next_fetch_at <= now() ORDER BY id")
            .fetch_all(&db)
            .await
            .unwrap();
    assert_eq!(due, [feed]);
    assert_ne!(other, feed);

    // Nobody subscribes to it anew.
    let grace = common::account(&db, "grace").await;
    let theirs = common::session(&db, grace).await;
    let body = json!({ "url": "https://ada.example/feed.xml" });
    let (status, _) = call(
        &state,
        Method::POST,
        "/api/feeds",
        Some(&theirs),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Lifted, everything is back, and it is due at once.
    assert_eq!(
        moderation::unblock(&db, "https://ada.example/feed.xml")
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        moderation::unblock(&db, "https://ada.example/feed.xml")
            .await
            .unwrap(),
        0
    );
    assert_eq!(titles(get("/api/articles".into()).await.1), 3);
    assert_eq!(
        titles(get(format!("/api/feeds/{feed}/articles")).await.1),
        3
    );
    assert_eq!(
        get(format!("/api/feeds/{feed}/articles/1")).await.0,
        StatusCode::OK
    );
    let (_, tree) = get("/api/tree".into()).await;
    let unread = tree["feeds"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == feed)
        .map(|row| row["unread"].clone());
    assert_eq!(unread, Some(json!(3)));
    let due: bool = sqlx::query_scalar("SELECT next_fetch_at <= now() FROM feeds WHERE id = $1")
        .bind(feed)
        .fetch_one(&db)
        .await
        .unwrap();
    assert!(due);
}

#[sqlx::test]
async fn a_blocked_feed_takes_its_images_out_of_the_cache(db: PgPool) {
    let (_, feed) = common::reader(&db, "ada", 2).await;
    sqlx::query("UPDATE feeds SET icon_url = 'https://ada.example/icon.png' WHERE id = $1;")
        .bind(feed)
        .execute(&db)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE articles SET thumbnail_url = 'https://ada.example/thumb-' || seq || '.png'
         WHERE feed_id = $1",
    )
    .bind(feed)
    .execute(&db)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE article_content
         SET html = '<p><img src=\"https://ada.example/in-' || seq || '.png\">'
                    || '<img src=\"https://ada.example/icon.png\"></p>'
         WHERE feed_id = $1",
    )
    .bind(feed)
    .execute(&db)
    .await
    .unwrap();
    let images = moderation::images_of(&db, feed).await.unwrap();
    assert_eq!(
        images,
        [
            "https://ada.example/icon.png",
            "https://ada.example/in-1.png",
            "https://ada.example/in-2.png",
            "https://ada.example/thumb-1.png",
            "https://ada.example/thumb-2.png",
        ]
    );

    let dir = std::env::temp_dir().join(format!("rueder-moderation-{feed}-{}", std::process::id()));
    let cache = Cache::open(&dir, 1 << 20).unwrap();
    let picture = Picture {
        format: Format::Png,
        bytes: vec![1, 2, 3],
    };
    cache.put(Size::Thumbnail, "https://ada.example/thumb-1.png", &picture);
    cache.put(Size::Image, "https://ada.example/in-2.png", &picture);
    cache.put(Size::Image, "https://elsewhere.example/keep.png", &picture);
    let gone: usize = images.iter().map(|url| cache.forget(url)).sum();
    assert_eq!(gone, 2);
    assert!(
        cache
            .get(Size::Thumbnail, "https://ada.example/thumb-1.png")
            .is_none()
    );
    assert!(
        cache
            .get(Size::Image, "https://elsewhere.example/keep.png")
            .is_some()
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[sqlx::test]
async fn a_blocked_feed_comes_in_with_no_file(db: PgPool) {
    let (_, feed) = common::reader(&db, "ada", 1).await;
    moderation::block(&db, "https://ada.example/feed.xml")
        .await
        .unwrap();
    let grace = common::account(&db, "grace").await;
    let cookie = common::session(&db, grace).await;
    let opml = r#"<?xml version="1.0"?><opml version="2.0"><body>
        <outline type="rss" text="Ada" xmlUrl="https://ada.example/feed.xml"/>
        </body></opml>"#;
    let state = AppState::new(db.clone());
    let (status, _, _) = common::raw(
        &state,
        Method::POST,
        "/api/opml",
        Some(&cookie),
        Some(("text/x-opml", opml.as_bytes().to_vec())),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let subscribed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM subscriptions WHERE account_id = $1 AND feed_id = $2",
    )
    .bind(grace)
    .bind(feed)
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(subscribed, 0);
}

#[sqlx::test]
async fn the_proxy_does_not_fetch_the_images_of_a_blocked_feed_again(db: PgPool) {
    let (_, feed) = common::reader(&db, "ada", 1).await;
    // On this machine, which the hardened client never reaches: a fetch
    // fails at once, and is told apart from a refusal without one.
    sqlx::query(
        "UPDATE articles SET thumbnail_url = 'https://127.0.0.1/thumb.png' WHERE feed_id = $1",
    )
    .bind(feed)
    .execute(&db)
    .await
    .unwrap();
    let dir = std::env::temp_dir().join(format!("rueder-blocked-{feed}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let config = Config {
        key: Some(vec![7; 32]),
        ..Config::new(&dir)
    };
    let client = HttpClient::new(http_client::Config::default()).unwrap();
    let proxy = ImageProxy::open(config, client)
        .unwrap()
        .with_blocked_from(db.clone());
    let ask = |url: &str| {
        let address = proxy.signer().address(Size::Thumbnail, url).unwrap();
        let parts: Vec<String> = address.split('/').map(str::to_owned).collect();
        let proxy = &proxy;
        async move { proxy.get(&parts[2], &parts[3], &parts[4]).await }
    };
    let elsewhere = "https://127.0.0.2/other.png";
    assert!(matches!(
        ask("https://127.0.0.1/thumb.png").await,
        Err(Refused::Source(_))
    ));

    moderation::block(&db, "https://ada.example/feed.xml")
        .await
        .unwrap();
    let listed: i64 = sqlx::query_scalar("SELECT count(*) FROM blocked_images WHERE feed_id = $1")
        .bind(feed)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(listed, 1);
    // Its image is refused without a fetch; another one is still fetched.
    assert!(matches!(
        ask("https://127.0.0.1/thumb.png").await,
        Err(Refused::NotFound)
    ));
    assert!(matches!(ask(elsewhere).await, Err(Refused::Source(_))));

    // Lifted, the proxy fetches it again.
    moderation::unblock(&db, "https://ada.example/feed.xml")
        .await
        .unwrap();
    let listed: i64 = sqlx::query_scalar("SELECT count(*) FROM blocked_images")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(listed, 0);
    assert!(matches!(
        ask("https://127.0.0.1/thumb.png").await,
        Err(Refused::Source(_))
    ));
    let _ = std::fs::remove_dir_all(dir);
}
