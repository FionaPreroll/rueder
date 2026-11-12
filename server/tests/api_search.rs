//! `GET /api/search`: what it finds, page after page, and that another account
//! sees nothing of it.

mod common;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use common::call;
use rueder::{
    AppState,
    http_client::{self, HttpClient},
    images::{Config, ImageProxy},
    reading,
};
use serde_json::Value;
use sqlx::PgPool;

async fn get(state: &AppState, path: &str, cookie: &str) -> (StatusCode, Value) {
    call(state, Method::GET, path, Some(cookie), None).await
}

fn seqs(hits: &Value) -> Vec<i64> {
    let articles = hits["articles"].as_array().unwrap().iter();
    articles.map(|a| a["seq"].as_i64().unwrap()).collect()
}

/// A state with an image proxy, which these tests never let fetch anything.
fn with_images(db: &PgPool) -> AppState {
    let dir = std::env::temp_dir().join(format!("rueder-api-search-{}", std::process::id()));
    let config = Config {
        key: Some(b"a key for tests, long enough to be one".to_vec()),
        ..Config::new(dir)
    };
    let client = HttpClient::new(http_client::Config::default()).unwrap();
    let images = Arc::new(ImageProxy::open(config, client).unwrap());
    AppState::new(db.clone()).with_images(images)
}

#[sqlx::test]
async fn finds_in_the_own_feeds_page_by_page(db: PgPool) {
    let (ada, feed) = common::reader(&db, "ada", 5).await;
    let state = with_images(&db);
    let cookie = common::session(&db, ada).await;

    // Article 3 gets a picture and a label, article 2 is read, article 4 hidden.
    sqlx::query("UPDATE articles SET thumbnail_url = 'https://pics.example/3.png' WHERE seq = 3")
        .execute(&db)
        .await
        .unwrap();
    reading::set_read(&db, ada, feed, 2, true).await.unwrap();
    reading::set_hidden(&db, ada, feed, 4, true).await.unwrap();
    let (status, label) = call(
        &state,
        Method::POST,
        "/api/labels",
        Some(&cookie),
        Some(serde_json::json!({ "title": "Later", "color": "#123456" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let label = label["id"].as_i64().unwrap();
    let status = call(
        &state,
        Method::PUT,
        &format!("/api/feeds/{feed}/articles/3/labels/{label}"),
        Some(&cookie),
        None,
    )
    .await
    .0;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, hits) = get(&state, "/api/search?q=Text%20ARTICLE", &cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(seqs(&hits), [5, 3, 2, 1]);
    assert_eq!(hits["next"], Value::Null);
    let third = &hits["articles"][1];
    assert_eq!(third["labels"], serde_json::json!([label]));
    assert_eq!(hits["articles"][2]["read"], true);
    let thumbnail = third["thumbnail_url"].as_str().unwrap();
    assert!(thumbnail.starts_with("/img/"), "{thumbnail}");
    assert!(!thumbnail.contains("pics.example"), "{thumbnail}");

    // Page by page, with next.
    let (_, first) = get(&state, "/api/search?q=article&limit=3", &cookie).await;
    assert_eq!(seqs(&first), [5, 3, 2]);
    let next = first["next"].as_i64().unwrap();
    let path = format!("/api/search?q=article&limit=3&before={next}");
    let (_, second) = get(&state, &path, &cookie).await;
    assert_eq!(seqs(&second), [1]);
    assert_eq!(second["next"], Value::Null);

    // Unread only, in the feed, in a folder, by tag and author.
    let (_, unread) = get(&state, "/api/search?q=article&unread=true", &cookie).await;
    assert_eq!(seqs(&unread), [5, 3, 1]);
    let (_, in_feed) = get(&state, &format!("/api/search?q=tag-1&feed={feed}"), &cookie).await;
    assert_eq!(seqs(&in_feed), [1]);
    let folder = common::folder(&db, ada, None, "News").await;
    let path = format!("/api/search?q=grace&folder={folder}");
    assert_eq!(
        seqs(&get(&state, &path, &cookie).await.1),
        Vec::<i64>::new()
    );
    common::move_to(&db, ada, feed, folder).await;
    assert_eq!(seqs(&get(&state, &path, &cookie).await.1), [5, 3, 2, 1]);
}

#[sqlx::test]
async fn another_account_finds_nothing(db: PgPool) {
    let (ada, feed) = common::reader(&db, "ada", 3).await;
    let grace = common::account(&db, "grace").await;
    let state = AppState::new(db.clone());
    let (ada_cookie, grace_cookie) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
    );
    let folder = common::folder(&db, ada, None, "News").await;
    common::move_to(&db, ada, feed, folder).await;
    let (_, hers) = get(&state, "/api/search?q=article", &ada_cookie).await;
    assert_eq!(seqs(&hers), [3, 2, 1]);

    // Not in all feeds, not by naming ada's feed or folder.
    let (status, hits) = get(&state, "/api/search?q=article", &grace_cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(hits, serde_json::json!({ "articles": [], "next": null }));
    for path in [
        format!("/api/search?q=article&feed={feed}"),
        format!("/api/search?q=article&folder={folder}"),
    ] {
        assert_eq!(
            get(&state, &path, &grace_cookie).await.0,
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
    // Once grace reads the feed too, she finds what is in it, with her own state.
    common::subscribe(&db, grace, feed).await;
    reading::set_read(&db, ada, feed, 3, true).await.unwrap();
    let (_, hits) = get(&state, "/api/search?q=article", &grace_cookie).await;
    assert_eq!(seqs(&hits), [3, 2, 1]);
    assert_eq!(hits["articles"][0]["read"], false);
    // And nobody without a session.
    let (status, _) = call(&state, Method::GET, "/api/search?q=article", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn refuses_what_cannot_be_searched_for(db: PgPool) {
    let (ada, feed) = common::reader(&db, "ada", 1).await;
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;
    let long = "a".repeat(201);
    for path in [
        "/api/search".to_owned(),
        "/api/search?q=".to_owned(),
        "/api/search?q=%21%3F".to_owned(),
        format!("/api/search?q={long}"),
        "/api/search?q=a+b+c+d+e+f+g+h+i+j+k".to_owned(),
        format!("/api/search?q=article&feed={feed}&folder=1"),
        "/api/search?q=article&before=x".to_owned(),
    ] {
        let status = get(&state, &path, &cookie).await.0;
        assert!(
            status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY,
            "{path}: {status}"
        );
    }
}
