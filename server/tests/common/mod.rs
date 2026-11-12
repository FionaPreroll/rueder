//! Helpers the tests share: made-up accounts, feeds and articles.
#![allow(dead_code)]

use rueder::articles::{self, NewArticle, Original};
use sqlx::PgPool;
use time::{Duration, OffsetDateTime, macros::datetime};

pub async fn account(db: &PgPool, name: &str) -> i32 {
    sqlx::query_scalar("INSERT INTO accounts (username) VALUES ($1) RETURNING id")
        .bind(name)
        .fetch_one(db)
        .await
        .unwrap()
}

pub async fn feed(db: &PgPool, name: &str) -> i32 {
    sqlx::query_scalar("INSERT INTO feeds (feed_url, title) VALUES ($1, $2) RETURNING id")
        .bind(format!("https://{name}.example/feed.xml"))
        .bind(name)
        .fetch_one(db)
        .await
        .unwrap()
}

pub async fn subscribe(db: &PgPool, account_id: i32, feed_id: i32) {
    sqlx::query("INSERT INTO subscriptions (account_id, feed_id, position) VALUES ($1, $2, $2)")
        .bind(account_id)
        .bind(feed_id)
        .execute(db)
        .await
        .unwrap();
}

/// An account that subscribes to one new feed with `count` articles.
pub async fn reader(db: &PgPool, name: &str, count: usize) -> (i32, i32) {
    let account_id = account(db, name).await;
    let feed_id = feed(db, name).await;
    subscribe(db, account_id, feed_id).await;
    add(db, feed_id, count).await;
    (account_id, feed_id)
}

pub const START: OffsetDateTime = datetime!(2026-03-01 12:00 UTC);

/// A made-up article; `n` makes it differ from the others of its feed.
pub fn article(n: usize) -> NewArticle {
    let at = START + Duration::minutes(n as i64);
    NewArticle {
        guid: format!("guid-{n}"),
        published_at: at,
        fetched_at: at + Duration::minutes(5),
        original: Original {
            title: format!("Article {n}"),
            content: format!("<p>Text of   article {n}.</p><script>track()</script>"),
            authors: vec!["Ada".into(), "Grace".into()],
            tags: vec!["news".into(), format!("tag-{n}")],
            url: Some(format!("https://blog.example/{n}")),
        },
        teaser: format!("Text of article {n}."),
        thumbnail_url: None,
        html: format!("<p>Text of article {n}.</p>"),
        enclosures: serde_json::json!([]),
        image_url: None,
        image_title: None,
    }
}

/// Adds `count` more articles to a feed and returns the number of the first.
pub async fn add(db: &PgPool, feed_id: i32, count: usize) -> i32 {
    let have: i32 = sqlx::query_scalar("SELECT last_seq FROM feeds WHERE id = $1")
        .bind(feed_id)
        .fetch_one(db)
        .await
        .unwrap();
    let new: Vec<_> = (1..=count).map(|n| article(have as usize + n)).collect();
    let mut tx = db.begin().await.unwrap();
    let first = articles::insert(&mut tx, feed_id, &new).await.unwrap();
    tx.commit().await.unwrap();
    first
}

/// A small repeatable random number generator, so a failing run can be run again.
pub struct Random(u64);

impl Random {
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// A number from 0 up to, not including, `n`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// What a logged-in browser of this account sends as its `Cookie` header.
pub async fn session(db: &PgPool, account_id: i32) -> String {
    let token = rueder::sessions::start(db, account_id, false)
        .await
        .unwrap();
    format!("{}={token}", rueder::sessions::COOKIE)
}

pub async fn folder(db: &PgPool, account_id: i32, parent: Option<i32>, title: &str) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO folders (account_id, parent_id, title, position)
         VALUES ($1, $2, $3, 1) RETURNING id",
    )
    .bind(account_id)
    .bind(parent)
    .bind(title)
    .fetch_one(db)
    .await
    .unwrap()
}

pub async fn move_to(db: &PgPool, account_id: i32, feed_id: i32, folder_id: i32) {
    sqlx::query("UPDATE subscriptions SET folder_id = $3 WHERE account_id = $1 AND feed_id = $2")
        .bind(account_id)
        .bind(feed_id)
        .bind(folder_id)
        .execute(db)
        .await
        .unwrap();
}

/// Asks the server something, as the browser with this cookie, and returns
/// the status and the JSON of the answer (`Null` if it has none).
pub async fn call(
    state: &rueder::AppState,
    method: axum::http::Method,
    path: &str,
    cookie: Option<&str>,
    body: Option<serde_json::Value>,
) -> (axum::http::StatusCode, serde_json::Value) {
    use axum::{body::Body, http::header};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let mut request = axum::http::Request::builder().method(method).uri(path);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    let body = match body {
        Some(body) => {
            request = request.header(header::CONTENT_TYPE, "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    let app = rueder::router(state.clone(), "missing".as_ref());
    let response = app.oneshot(request.body(body).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}

/// Asks the server something with a body of the given content type, and
/// returns the answer as it came, with its headers.
pub async fn raw(
    state: &rueder::AppState,
    method: axum::http::Method,
    path: &str,
    cookie: Option<&str>,
    body: Option<(&str, Vec<u8>)>,
) -> (axum::http::StatusCode, axum::http::HeaderMap, Vec<u8>) {
    use axum::{body::Body, http::header};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let mut request = axum::http::Request::builder().method(method).uri(path);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    let body = match body {
        Some((kind, body)) => {
            request = request.header(header::CONTENT_TYPE, kind);
            Body::from(body)
        }
        None => Body::empty(),
    };
    let app = rueder::router(state.clone(), "missing".as_ref());
    let request = request.body(body).unwrap();
    let response = app.oneshot(request).await.unwrap();
    let (status, headers) = (response.status(), response.headers().clone());
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, headers, bytes.to_vec())
}
