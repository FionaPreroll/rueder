//! A changed rule looks again at everything that is hidden, which can be most
//! of a large feed: it reads the texts a few at a time. Reading all of them at
//! once took 226 MiB for 20,000 hidden articles of 11 KB.
//!
//! One test, as the count of the allocator is shared by the whole program.
//! Past the budget, an allocation fails and ends the program.

mod common;

use std::alloc::System;

use axum::http::{Method, StatusCode};
use cap::Cap;
use serde_json::json;
use sqlx::PgPool;

#[global_allocator]
static ALLOCATOR: Cap<System> = Cap::new(System, usize::MAX);

/// What a changed rule may take, a third of the hidden texts.
const BUDGET: usize = 16 * 1024 * 1024;

#[sqlx::test]
async fn a_changed_rule_reads_hidden_texts_a_few_at_a_time(db: PgPool) {
    let (ada, feed) = common::reader(&db, "ada", 500).await;
    // 500 texts of 100 KiB, all of them hidden.
    sqlx::query("UPDATE article_content SET html = repeat('<p>Words of a text.</p>', 4_500)")
        .execute(&db)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO article_states (account_id, feed_id, seq, hidden)
         SELECT $1, $2, seq, true FROM articles WHERE feed_id = $2",
    )
    .bind(ada)
    .bind(feed)
    .execute(&db)
    .await
    .unwrap();
    let state = rueder::AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;
    let rule = json!({
        "scope": { "kind": "all" }, "field": "title", "match": "contains",
        "pattern": "Article", "action": "hide",
    });

    ALLOCATOR.set_limit(ALLOCATOR.allocated() + BUDGET).unwrap();
    let (status, made) = common::call(
        &state,
        Method::POST,
        "/api/rules",
        Some(&cookie),
        Some(rule),
    )
    .await;
    ALLOCATOR.set_limit(usize::MAX).unwrap();
    assert_eq!(status, StatusCode::CREATED, "{made}");
    // Every one is still hidden, by the new rule now.
    assert_eq!(
        made["changed"],
        json!({ "hidden": 0, "shown": 0, "read": 0 })
    );
}
