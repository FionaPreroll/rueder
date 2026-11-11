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
    let token = rueder::sessions::start(db, account_id).await.unwrap();
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
