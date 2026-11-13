//! The status page of the admins: who sees it, and what it says.

mod common;

use std::time::Instant;

use axum::http::{Method, StatusCode};
use common::call;
use rueder::{
    AppState, accounts, fetch,
    monitor::{Fetched, Round, Severity},
};
use serde_json::Value;
use sqlx::PgPool;

async fn status(state: &AppState, cookie: Option<&str>) -> (StatusCode, Value) {
    call(state, Method::GET, "/api/status", cookie, None).await
}

#[sqlx::test]
async fn only_an_admin_sees_the_status(db: PgPool) {
    let ada = common::account(&db, "Ada").await;
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;

    assert_eq!(status(&state, None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(status(&state, Some(&cookie)).await.0, StatusCode::FORBIDDEN);

    // Made an admin on the command line, she sees it with the session she has.
    assert!(accounts::set_admin(&db, " ada ", true).await.unwrap());
    let (answer, body) = status(&state, Some(&cookie)).await;
    assert_eq!(answer, StatusCode::OK);
    assert_eq!(body["counts"]["accounts"], 1);
    assert!(accounts::list(&db).await.unwrap()[0].is_admin);

    // And no more once she is a reader again.
    assert!(accounts::set_admin(&db, "ada", false).await.unwrap());
    assert_eq!(status(&state, Some(&cookie)).await.0, StatusCode::FORBIDDEN);
    assert!(!accounts::set_admin(&db, "nobody", true).await.unwrap());
}

#[sqlx::test]
async fn the_status_counts_and_names_nobody(db: PgPool) {
    let root = common::account(&db, "root").await;
    accounts::set_admin(&db, "root", true).await.unwrap();
    let (grace, failing) = common::reader(&db, "grace-secret", 3).await;
    let (_, missing) = common::reader(&db, "linus-secret", 0).await;
    let (_, also_missing) = common::reader(&db, "ken-secret", 0).await;
    for (feed, message) in [
        (
            failing,
            "intranet-secret.example is at 10.0.0.1, which is not on the public internet (private)",
        ),
        (missing, "the site answers with 404 Not Found"),
        (also_missing, "the site answers with 404 Not Found"),
    ] {
        sqlx::query("UPDATE feeds SET error_message = $2, fetched_at = now() WHERE id = $1")
            .bind(feed)
            .bind(message)
            .execute(&db)
            .await
            .unwrap();
    }
    // What PostgreSQL estimates is right after it has looked.
    sqlx::query("ANALYZE").execute(&db).await.unwrap();
    let state = AppState::new(db.clone());
    let (admin, reader) = (
        common::session(&db, root).await,
        common::session(&db, grace).await,
    );
    for _ in 0..3 {
        let (answer, _) = call(&state, Method::GET, "/api/tree", Some(&reader), None).await;
        assert_eq!(answer, StatusCode::OK);
    }
    state.monitor.problems.note(
        Severity::Error,
        "rueder",
        "the database went away",
        Instant::now(),
    );

    let (answer, body) = status(&state, Some(&admin)).await;
    assert_eq!(answer, StatusCode::OK);
    assert_eq!(body["counts"]["accounts"], 4);
    assert_eq!(body["counts"]["subscribed_feeds"], 3);

    // Requests count by their route.
    let routes = body["requests"]["routes"].as_array().unwrap();
    let tree = routes
        .iter()
        .find(|route| route["route"] == "GET /api/tree")
        .unwrap_or_else(|| panic!("no tree in {routes:?}"));
    assert_eq!(tree["took"]["count"], 3);
    assert_eq!(tree["errors"], 0);
    assert!(tree["took"]["p95_ms"].as_f64().unwrap() >= 0.0);

    // Failing feeds count by why, the most first.
    let failing = &body["fetcher"]["failing"];
    assert_eq!(failing[0]["failure"], "status");
    assert_eq!(failing[0]["status"], 404);
    assert_eq!(failing[0]["feeds"], 2);
    assert_eq!(failing[1]["failure"], "blocked");
    assert_eq!(failing[1]["feeds"], 1);
    assert_eq!(body["fetcher"]["running"], false);

    // What the log said is there, and so is the hint at it.
    let recent = &body["problems"]["recent"];
    assert_eq!(recent[0]["text"], "the database went away");
    assert_eq!(recent[0]["error"], true);
    assert_eq!(body["problems"]["errors"], 1);
    let hints: Vec<&str> = body["hints"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hint| hint["kind"].as_str().unwrap())
        .collect();
    assert!(hints.contains(&"errors"), "{hints:?}");
    assert!(hints.contains(&"not_fetching"), "{hints:?}");
    assert!(hints.contains(&"feeds_failing"), "{hints:?}");

    // The tables are there, the articles among them.
    let tables = body["database"]["tables"].as_array().unwrap();
    let accounts = tables
        .iter()
        .find(|table| table["name"] == "accounts")
        .unwrap();
    assert!(accounts["bytes"].as_i64().unwrap() > 0, "{accounts}");
    assert_eq!(accounts["rows"], 4);
    assert_eq!(body["counts"]["articles"], 3);
    assert!(body["database"]["size_bytes"].as_i64().unwrap() > 0);

    // Nothing names a feed, an address or an account.
    let text = body.to_string();
    for secret in ["secret", "grace", "linus", "example"] {
        assert!(!text.contains(secret), "{secret} in {text}");
    }
}

#[sqlx::test]
async fn the_status_says_how_fast_feeds_are_fetched_and_how_far_behind(db: PgPool) {
    let root = common::account(&db, "root").await;
    accounts::set_admin(&db, "root", true).await.unwrap();
    let (_, waiting) = common::reader(&db, "grace", 0).await;
    let (_, also_waiting) = common::reader(&db, "linus", 0).await;
    // Twenty minutes past their time, and fetched before.
    for feed in [waiting, also_waiting] {
        sqlx::query(
            "UPDATE feeds SET next_fetch_at = now() - interval '20 minutes', fetched_at = now()
             WHERE id = $1",
        )
        .bind(feed)
        .execute(&db)
        .await
        .unwrap();
    }
    let state = AppState::new(db.clone());
    let admin = common::session(&db, root).await;
    let pace = fetch::Pace {
        round: 2,
        at_once: 4,
        every: std::time::Duration::from_secs(60),
    };
    state.monitor.fetching_runs(pace);
    let now = Instant::now();
    for fetched in [
        Fetched::Fetched { new: 3 },
        Fetched::Unchanged,
        Fetched::Failed,
    ] {
        state
            .monitor
            .fetch(fetched, std::time::Duration::from_millis(40), now);
    }
    // The last round was full: the pace, not the sites, holds the feeds back.
    state.monitor.round(Round {
        at: time::OffsetDateTime::now_utc(),
        feeds: 2,
        took: std::time::Duration::from_secs(1),
    });

    let (answer, body) = status(&state, Some(&admin)).await;
    assert_eq!(answer, StatusCode::OK);
    let fetcher = &body["fetcher"];
    assert_eq!(fetcher["running"], true);
    assert_eq!(
        fetcher["pace"],
        serde_json::json!({ "round": 2, "at_once": 4, "every_s": 60, "per_hour": 120 })
    );
    assert_eq!(fetcher["per_hour"], 3);
    let behind = fetcher["behind_s"].as_f64().unwrap();
    assert!((20.0 * 60.0..21.0 * 60.0).contains(&behind), "{behind}");
    let kinds: Vec<&str> = body["hints"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hint| hint["kind"].as_str().unwrap())
        .collect();
    assert!(kinds.contains(&"fetch_backlog"), "{kinds:?}");
    let full = body["hints"]
        .as_array()
        .unwrap()
        .iter()
        .find(|hint| hint["kind"] == "fetch_pace_full")
        .unwrap_or_else(|| panic!("{kinds:?}"));
    assert_eq!(full["per_hour"], 120);

    // A round with room to spare is no sign of the pace.
    state.monitor.round(Round {
        at: time::OffsetDateTime::now_utc(),
        feeds: 1,
        took: std::time::Duration::from_secs(1),
    });
    let (_, body) = status(&state, Some(&admin)).await;
    assert!(!body.to_string().contains("fetch_pace_full"));
    // Nothing is behind once nothing is due.
    sqlx::query("UPDATE feeds SET next_fetch_at = now() + interval '1 hour'")
        .execute(&db)
        .await
        .unwrap();
    let (_, body) = status(&state, Some(&admin)).await;
    assert_eq!(body["fetcher"]["behind_s"], 0.0);
}
