//! Reports of feeds and articles, their limits, and blocking a feed for
//! everybody and lifting the block again.

mod common;

use std::{sync::Arc, time::Duration};

use axum::http::{Method, StatusCode};
use common::{call, raw};
use rueder::{
    AppState, accounts,
    articles::{self, NewArticle},
    events::Events,
    fetch,
    http_client::{self, HttpClient},
    images::{Config, Format, ImageProxy, Picture, Refused, Size},
    moderation,
    monitor::Monitor,
    rate::{Rate, Rates},
};
use serde_json::{Value, json};
use sqlx::PgPool;

async fn report(state: &AppState, cookie: &str, path: &str, body: Value) -> StatusCode {
    call(state, Method::POST, path, Some(cookie), Some(body))
        .await
        .0
}

async fn get(state: &AppState, cookie: &str, path: &str) -> (StatusCode, Value) {
    call(state, Method::GET, path, Some(cookie), None).await
}

async fn reports(db: &PgPool) -> Vec<(i32, i32, Option<i32>, String, String)> {
    sqlx::query_as(
        "SELECT account_id, feed_id, seq, reason, text FROM feed_reports ORDER BY account_id, id",
    )
    .fetch_all(db)
    .await
    .unwrap()
}

/// What the status page says about reported feeds.
async fn reported(state: &AppState, admin: &str) -> Value {
    let (status, answer) = get(state, admin, "/api/status").await;
    assert_eq!(status, StatusCode::OK);
    answer["reports"].clone()
}

async fn admin(db: &PgPool) -> String {
    let id = accounts::create(db, "root", None, true)
        .await
        .unwrap()
        .unwrap();
    common::session(db, id).await
}

#[sqlx::test]
async fn a_reader_reports_what_they_read_and_admins_never_see_who(db: PgPool) {
    let (ada, news) = common::reader(&db, "ada", 3).await;
    let (grace, comics) = common::reader(&db, "grace", 1).await;
    common::subscribe(&db, grace, news).await;
    let state = AppState::new(db.clone());
    let (mine, theirs) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
    );
    let feed = format!("/api/feeds/{news}/report");

    assert_eq!(
        report(&state, &mine, &feed, json!({ "reason": "spam" })).await,
        StatusCode::NO_CONTENT
    );
    let article = format!("/api/feeds/{news}/articles/2/report");
    let text = "  Sells\u{0}pills\n  ";
    assert_eq!(
        report(
            &state,
            &mine,
            &article,
            json!({ "reason": "illegal", "text": text })
        )
        .await,
        StatusCode::NO_CONTENT
    );
    // Reporting again replaces the report.
    assert_eq!(
        report(
            &state,
            &mine,
            &feed,
            json!({ "reason": "hate", "text": "Threats" })
        )
        .await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        report(&state, &theirs, &feed, json!({ "reason": "hate" })).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        reports(&db).await,
        vec![
            (ada, news, None, "hate".into(), "Threats".into()),
            (ada, news, Some(2), "illegal".into(), "Sells pills".into()),
            (grace, news, None, "hate".into(), String::new()),
        ]
    );

    // Only what the account reads, only articles that are there, only the
    // reasons of the list, and a short text.
    let ada_cannot = [
        (
            format!("/api/feeds/{comics}/report"),
            json!({ "reason": "spam" }),
            StatusCode::NOT_FOUND,
        ),
        (
            format!("/api/feeds/{comics}/articles/1/report"),
            json!({ "reason": "spam" }),
            StatusCode::NOT_FOUND,
        ),
        (
            format!("/api/feeds/{news}/articles/99/report"),
            json!({ "reason": "spam" }),
            StatusCode::NOT_FOUND,
        ),
        (
            feed.clone(),
            json!({ "reason": "boring" }),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            feed.clone(),
            json!({ "reason": "other", "text": "x".repeat(501) }),
            StatusCode::BAD_REQUEST,
        ),
    ];
    for (path, body, answer) in ada_cannot {
        assert_eq!(
            report(&state, &mine, &path, body.clone()).await,
            answer,
            "{path} {body}"
        );
    }
    let (status, _) = call(
        &state,
        Method::POST,
        &feed,
        None,
        Some(json!({ "reason": "spam" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(reports(&db).await.len(), 3);

    // Readers do not see the reports; admins see the address and the numbers.
    let (status, _) = get(&state, &mine, "/api/status").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let root = admin(&db).await;
    let seen = reported(&state, &root).await;
    assert_eq!(seen["blocked"], 0);
    let feeds = seen["feeds"].as_array().unwrap();
    assert_eq!(feeds.len(), 1);
    let one = &feeds[0];
    assert_eq!(one["feed_url"], "https://ada.example/feed.xml");
    assert_eq!(
        (
            &one["reports"],
            &one["accounts"],
            &one["articles"],
            &one["blocked"]
        ),
        (&json!(3), &json!(2), &json!(1), &json!(false))
    );
    assert_eq!(
        one["reasons"],
        json!([{ "reason": "hate", "count": 2 }, { "reason": "illegal", "count": 1 }])
    );
    assert_eq!(one["texts"], json!(["Threats", "Sells pills"]));
    // Nothing in it names an account.
    let words = seen.to_string();
    for name in ["ada\"", "grace", "\"account_id"] {
        assert!(!words.contains(name), "{name} in {words}");
    }
    let (_, status) = get(&state, &root, "/api/status").await;
    let hints = status["hints"].as_array().unwrap();
    assert!(
        hints.contains(&json!({ "kind": "reported_feeds", "feeds": 1 })),
        "{hints:?}"
    );

    // Dismissed on the command line, they are gone.
    assert_eq!(
        moderation::dismiss(&db, "ada.example/feed.xml")
            .await
            .unwrap(),
        3
    );
    assert!(reports(&db).await.is_empty());
    // Deleting an account takes its reports along.
    report(&state, &theirs, &feed, json!({ "reason": "spam" })).await;
    sqlx::query("DELETE FROM accounts WHERE id = $1")
        .bind(grace)
        .execute(&db)
        .await
        .unwrap();
    assert!(reports(&db).await.is_empty());
}

#[sqlx::test]
async fn an_account_reports_only_so_much(db: PgPool) {
    let (ada, news) = common::reader(&db, "ada", 3).await;
    let state = AppState::new(db.clone())
        .with_rates(Rates::default().with_content_reports(Rate::new(2, Duration::from_secs(3600))));
    let cookie = common::session(&db, ada).await;
    let path = |seq: i32| format!("/api/feeds/{news}/articles/{seq}/report");
    let spam = json!({ "reason": "spam" });
    assert_eq!(
        report(&state, &cookie, &path(1), spam.clone()).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        report(&state, &cookie, &path(2), spam.clone()).await,
        StatusCode::NO_CONTENT
    );
    let (status, headers, _) = raw(
        &state,
        Method::POST,
        &path(3),
        Some(&cookie),
        Some(("application/json", spam.to_string().into_bytes())),
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert!(headers.contains_key("retry-after"));
    assert_eq!(reports(&db).await.len(), 2);

    // And it has only so many at once, however slowly they come.
    let state = AppState::new(db.clone());
    let other = common::feed(&db, "other").await;
    common::subscribe(&db, ada, other).await;
    common::add(&db, other, 1).await;
    sqlx::query(
        "INSERT INTO feed_reports (account_id, feed_id, seq, reason)
         SELECT $1, $2, n, 'spam' FROM generate_series(1, $3) n",
    )
    .bind(ada)
    .bind(other)
    .bind(moderation::MAX_PER_ACCOUNT as i32 - 2)
    .execute(&db)
    .await
    .unwrap();
    let full = format!("/api/feeds/{other}/report");
    assert_eq!(
        report(&state, &cookie, &full, spam.clone()).await,
        StatusCode::FORBIDDEN
    );
    // One it has already can still change.
    assert_eq!(
        report(&state, &cookie, &path(1), json!({ "reason": "other" })).await,
        StatusCode::NO_CONTENT
    );
}

/// An article with a picture, an image in its text and one more beside it.
fn pictured(n: usize) -> NewArticle {
    let mut article = common::article(n);
    article.thumbnail_url = Some(format!("https://cdn.example/thumb-{n}.png"));
    article.image_url = Some(format!("https://cdn.example/lead-{n}.png"));
    article.html =
        format!("<p>Text of article {n}.</p><img src=\"https://cdn.example/in-{n}.png\">");
    article
}

fn tiny() -> Picture {
    Picture {
        format: Format::Png,
        bytes: vec![1, 2, 3],
    }
}

fn cache_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("rueder-moderation-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[sqlx::test]
async fn a_blocked_feed_is_shown_to_nobody_and_comes_back_when_unblocked(db: PgPool) {
    let (ada, news) = common::reader(&db, "ada", 0).await;
    let grace = common::account(&db, "grace").await;
    common::subscribe(&db, grace, news).await;
    let (_, other) = common::reader(&db, "lin", 2).await;
    common::subscribe(&db, ada, other).await;
    let mut tx = db.begin().await.unwrap();
    articles::insert(&mut tx, news, &[pictured(1), pictured(2)])
        .await
        .unwrap();
    tx.commit().await.unwrap();
    sqlx::query("UPDATE feeds SET icon_url = 'https://ada.example/icon.png' WHERE id = $1")
        .bind(news)
        .execute(&db)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO article_states (account_id, feed_id, seq, starred) VALUES ($1, $2, 1, true)",
    )
    .bind(ada)
    .bind(news)
    .execute(&db)
    .await
    .unwrap();

    let dir = cache_dir("block");
    let client = HttpClient::new(http_client::Config::default()).unwrap();
    let config = Config {
        key: Some(vec![7; 32]),
        ..Config::new(&dir)
    };
    let proxy = Arc::new(
        ImageProxy::open(config, client)
            .unwrap()
            .with_blocked_from(db.clone()),
    );
    let cached = [
        (Size::Icon, "https://ada.example/icon.png"),
        (Size::Thumbnail, "https://cdn.example/thumb-1.png"),
        (Size::Image, "https://cdn.example/lead-2.png"),
        (Size::Image, "https://cdn.example/in-1.png"),
        (Size::Image, "https://lin.example/elsewhere.png"),
    ];
    for (size, url) in cached {
        proxy.cache().put(size, url, &tiny());
    }
    let state = AppState::new(db.clone()).with_images(proxy.clone());
    let cookie = common::session(&db, ada).await;

    let seen = |state: AppState, cookie: String| async move {
        let mut seen = Vec::new();
        for path in [
            "/api/articles".to_owned(),
            format!("/api/feeds/{news}/articles"),
            "/api/starred".to_owned(),
            "/api/search?q=article".to_owned(),
            format!("/api/search?q=article&feed={news}"),
        ] {
            let (status, list) = get(&state, &cookie, &path).await;
            assert_eq!(status, StatusCode::OK, "{path}");
            let of_news = list["articles"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|article| article["feed_id"] == news)
                .count();
            seen.push(of_news);
        }
        let (opened, _) = get(&state, &cookie, &format!("/api/feeds/{news}/articles/1")).await;
        let (_, tree) = get(&state, &cookie, "/api/tree").await;
        let feed = tree["feeds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|feed| feed["id"] == news)
            .unwrap()
            .clone();
        (seen, opened, feed)
    };
    let (before, opened, feed) = seen(state.clone(), cookie.clone()).await;
    assert_eq!(before, [2, 2, 1, 2, 2]);
    assert_eq!(opened, StatusCode::OK);
    assert_eq!(
        (&feed["blocked"], &feed["unread"]),
        (&json!(false), &json!(2))
    );
    assert!(feed["icon_url"].is_string());

    let blocked = moderation::block(&db, " https://ada.example/feed.xml ", Some(proxy.cache()))
        .await
        .unwrap();
    assert_eq!(
        blocked,
        moderation::Blocked {
            feeds: 1,
            made: false,
            images: 7,
            removed: 4,
            subscribers: 2,
        }
    );

    // Its articles are nowhere, for nobody; the subscription stays and says so.
    let (after, opened, feed) = seen(state.clone(), cookie.clone()).await;
    assert_eq!(after, [0, 0, 0, 0, 0]);
    assert_eq!(opened, StatusCode::NOT_FOUND);
    assert_eq!(
        (&feed["blocked"], &feed["unread"], &feed["icon_url"]),
        (&json!(true), &json!(0), &Value::Null)
    );
    let (theirs, _, _) = seen(state.clone(), common::session(&db, grace).await).await;
    assert_eq!(theirs, [0, 0, 0, 0, 0]);
    // Another feed's articles stay.
    let (_, all) = get(&state, &cookie, "/api/articles").await;
    assert_eq!(all["articles"].as_array().unwrap().len(), 2);

    // Its images left the cache, and the proxy does not fetch them again.
    for (size, url) in cached {
        let kept = proxy.cache().get(size, url).is_some();
        assert_eq!(kept, url.starts_with("https://lin."), "{url}");
        let address = proxy.signer().address(size, url).unwrap();
        let parts: Vec<&str> = address.split('/').collect();
        if !kept {
            assert!(matches!(
                proxy.get(parts[2], parts[3], parts[4]).await,
                Err(Refused::NotFound)
            ));
        }
    }

    // Nobody subscribes to it anew, alone or with an OPML file.
    let lin = common::account(&db, "lin2").await;
    let lin_cookie = common::session(&db, lin).await;
    let (status, _) = call(
        &state,
        Method::POST,
        "/api/feeds",
        Some(&lin_cookie),
        Some(json!({ "url": "https://ada.example/feed.xml" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let opml = r#"<?xml version="1.0"?><opml version="1.0"><body>
        <outline type="rss" text="News" xmlUrl="https://ada.example/feed.xml"/>
        <outline type="rss" text="Lin" xmlUrl="https://lin.example/feed.xml"/>
        </body></opml>"#;
    let (status, _, answer) = raw(
        &state,
        Method::POST,
        "/api/opml",
        Some(&lin_cookie),
        Some(("text/x-opml", opml.as_bytes().to_vec())),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let answer: Value = serde_json::from_slice(&answer).unwrap();
    assert_eq!(
        (&answer["feeds"], &answer["known"], &answer["skipped"]),
        (&json!(1), &json!(0), &json!(1))
    );
    let (_, tree) = get(&state, &lin_cookie, "/api/tree").await;
    assert_eq!(tree["feeds"].as_array().unwrap().len(), 1);

    // It is not fetched, nor counted as waiting to be.
    sqlx::query(
        "UPDATE feeds SET next_fetch_at = CASE WHEN id = $1 THEN now() - interval '1 hour'
                                          ELSE now() + interval '1 day' END",
    )
    .bind(news)
    .execute(&db)
    .await
    .unwrap();
    let pace = fetch::Pace::default();
    let fetched = fetch::fetch_due(
        &db,
        &HttpClient::new(http_client::Config::default()).unwrap(),
        &Arc::new(rueder::cleaning::Rules::built_in().unwrap()),
        &Events::new(),
        &Arc::new(Monitor::default()),
        &pace,
    )
    .await
    .unwrap();
    assert_eq!(fetched, 0);
    let root = admin(&db).await;
    let (_, status) = get(&state, &root, "/api/status").await;
    assert_eq!(status["reports"]["blocked"], 1);
    assert_eq!(status["counts"]["subscribed_feeds"], 1);

    // Lifted, everything is as it was, and it is fetched at once.
    assert_eq!(
        moderation::unblock(&db, "ada.example/feed.xml")
            .await
            .unwrap(),
        1
    );
    let (again, opened, feed) = seen(state.clone(), cookie.clone()).await;
    assert_eq!(again, before);
    assert_eq!(opened, StatusCode::OK);
    assert_eq!(
        (&feed["blocked"], &feed["unread"]),
        (&json!(false), &json!(2))
    );
    let images: i64 = sqlx::query_scalar("SELECT count(*) FROM blocked_images")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(images, 0);
    let due: bool = sqlx::query_scalar("SELECT next_fetch_at <= now() FROM feeds WHERE id = $1")
        .bind(news)
        .fetch_one(&db)
        .await
        .unwrap();
    assert!(due);
    let (status, _) = call(
        &state,
        Method::POST,
        "/api/feeds",
        Some(&lin_cookie),
        Some(json!({ "url": "https://ada.example/feed.xml" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    // Nothing was blocked a second time.
    assert!(
        moderation::unblock(&db, "ada.example/feed.xml")
            .await
            .unwrap()
            == 0
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[sqlx::test]
async fn an_address_without_a_feed_can_be_blocked_before_anybody_reads_it(db: PgPool) {
    let ada = common::account(&db, "ada").await;
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;
    let blocked = moderation::block(&db, "spam.example/rss", None)
        .await
        .unwrap();
    assert!(blocked.made);
    assert_eq!((blocked.feeds, blocked.images), (1, 0));
    // As subscribing writes it, so subscribing finds it, without fetching.
    let (status, _) = call(
        &state,
        Method::POST,
        "/api/feeds",
        Some(&cookie),
        Some(json!({ "url": "spam.example/rss" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    // Blocking it again makes nothing new.
    let again = moderation::block(&db, "https://spam.example/rss", None)
        .await
        .unwrap();
    assert!(!again.made);
    assert!(moderation::block(&db, "  ", None).await.is_err());
}
