//! The status of the server across restarts and over longer times, and how
//! long it is kept.

mod common;

use std::time::{Duration, Instant};

use axum::http::{Method, StatusCode};
use common::call;
use rueder::{
    AppState, accounts, history,
    monitor::{self, Fetched, Looked, Requests, Severity},
    retention,
};
use serde_json::Value;
use sqlx::PgPool;
use time::OffsetDateTime;

async fn status(state: &AppState, cookie: &str, hours: Option<u32>) -> (StatusCode, Value) {
    let path = match hours {
        Some(hours) => format!("/api/status?hours={hours}"),
        None => "/api/status".to_owned(),
    };
    call(state, Method::GET, &path, Some(cookie), None).await
}

async fn admin(db: &PgPool) -> String {
    let root = common::account(db, "root").await;
    accounts::set_admin(db, "root", true).await.unwrap();
    common::session(db, root).await
}

fn route<'a>(body: &'a Value, name: &str) -> Option<&'a Value> {
    body["requests"]["routes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|route| route["route"] == name)
}

/// A part of the hour as it is saved, `ago` before now, with `count`
/// requests to a route.
async fn saved_requests(db: &PgPool, ago: time::Duration, route: &str, count: u64) {
    let mut requests = Requests::default();
    for _ in 0..count {
        requests.timing.note(Duration::from_millis(5));
    }
    requests.answers[1] = count;
    let number = monitor::slot_at(OffsetDateTime::now_utc() - ago);
    sqlx::query(
        "INSERT INTO status_slots (starts_at, kind, route, counts) VALUES ($1, 'requests', $2, $3)",
    )
    .bind(monitor::slot_start(number))
    .bind(route)
    .bind(serde_json::to_value(&requests).unwrap())
    .execute(db)
    .await
    .unwrap();
}

#[sqlx::test]
async fn the_status_comes_back_after_a_restart(db: PgPool) {
    let admin = admin(&db).await;
    let before = AppState::new(db.clone());
    for _ in 0..3 {
        let (answer, _) = call(&before, Method::GET, "/api/tree", Some(&admin), None).await;
        assert_eq!(answer, StatusCode::OK);
    }
    let now = Instant::now();
    let monitor = &before.monitor;
    monitor.fetch(Fetched::Fetched { new: 4 }, Duration::from_millis(30), now);
    monitor.fetch(Fetched::Failed, Duration::from_millis(30), now);
    monitor.lookup(Looked::Found, Duration::from_millis(80), now);
    monitor
        .problems
        .note(Severity::Error, "rueder", "the disk is full", now);
    monitor
        .problems
        .note(Severity::Warning, "rueder::fetch", "slow site", now);
    history::save(&db, monitor, None).await.unwrap();
    // Saving again with nothing new changes nothing.
    history::save(&db, monitor, None).await.unwrap();

    // A new server: what the old one counted is there again.
    let after = AppState::new(db.clone());
    after.monitor.problems.note(
        Severity::Error,
        "rueder",
        "the disk is full",
        Instant::now(),
    );
    history::load(&db, &after.monitor, None).await.unwrap();
    let (answer, body) = status(&after, &admin, None).await;
    assert_eq!(answer, StatusCode::OK);
    assert_eq!(body["hours"], 1);
    assert_eq!(body["kept_days"], 7);
    let tree = route(&body, "GET /api/tree").unwrap();
    assert_eq!(tree["took"]["count"], 3);
    assert_eq!(body["fetcher"]["fetched"], 1);
    assert_eq!(body["fetcher"]["failed"], 1);
    assert_eq!(body["fetcher"]["new_articles"], 4);
    assert_eq!(body["fetcher"]["per_hour"], 2);
    assert_eq!(body["finder"]["found"], 1);
    let recent = body["problems"]["recent"].as_array().unwrap();
    assert_eq!(recent.len(), 2);
    // The same problem before and after the restart is one.
    assert_eq!(recent[0]["text"], "the disk is full");
    assert_eq!(recent[0]["count"], 2);
    assert_eq!(recent[1]["text"], "slow site");
    assert_eq!(body["problems"]["errors"], 2);
    assert_eq!(body["problems"]["warnings"], 1);
    assert!(body["hints"].to_string().contains("\"errors\""));

    // What is saved holds no name of an account.
    let saved: Vec<String> = sqlx::query_scalar("SELECT counts::text || route FROM status_slots")
        .fetch_all(&db)
        .await
        .unwrap();
    assert!(!saved.is_empty());
    assert!(saved.iter().all(|row| !row.contains("root")), "{saved:?}");
}

#[sqlx::test]
async fn the_status_counts_over_as_many_hours_as_are_kept(db: PgPool) {
    let admin = admin(&db).await;
    let state = AppState::new(db.clone());
    saved_requests(&db, time::Duration::hours(2), "GET /api/tree", 5).await;
    saved_requests(&db, time::Duration::days(3), "GET /api/tree", 7).await;
    saved_requests(&db, time::Duration::days(3), "GET /api/search", 1).await;
    // A row that is not what the server writes is left out, not an error.
    sqlx::query(
        "INSERT INTO status_slots (starts_at, kind, route, counts)
         VALUES (now() - interval '1 day', 'fetches', '', '{\"timing\": 7}'),
                (now() - interval '1 day', 'requests', 'GET /x', '{\"timing\": {\"count\": 1,
                 \"total_us\": 1, \"max_us\": 1, \"buckets\": [1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]},
                 \"answers\": [0, 1, 0, 0, 0], \"limited\": 0}')",
    )
    .execute(&db)
    .await
    .unwrap();

    // The last hour, from memory: only the request for the status itself.
    let (answer, body) = status(&state, &admin, None).await;
    assert_eq!(answer, StatusCode::OK);
    assert!(route(&body, "GET /api/tree").is_none());

    // A day: the part two hours ago, and what is in memory, saved first.
    let (_, body) = status(&state, &admin, Some(24)).await;
    assert_eq!(body["hours"], 24);
    assert_eq!(route(&body, "GET /api/tree").unwrap()["took"]["count"], 5);
    assert_eq!(route(&body, "GET /api/status").unwrap()["took"]["count"], 1);
    assert!(route(&body, "GET /x").is_none());
    assert_eq!(body["fetcher"]["fetched"], 0);

    // Four days: the part of three days ago too.
    let (_, body) = status(&state, &admin, Some(96)).await;
    assert_eq!(body["hours"], 96);
    assert_eq!(route(&body, "GET /api/tree").unwrap()["took"]["count"], 12);
    assert_eq!(route(&body, "GET /api/search").unwrap()["took"]["count"], 1);
    assert_eq!(body["requests"]["all"]["count"], 12 + 1 + 2);
    assert_eq!(body["requests"]["answers"][1], 12 + 1 + 2);

    // No more than is kept, and no less than an hour.
    let (_, body) = status(&state, &admin, Some(100_000)).await;
    assert_eq!(body["hours"], 7 * 24);
    let (_, body) = status(&state, &admin, Some(0)).await;
    assert_eq!(body["hours"], 1);
    retention::set(&db, &retention::Settings { status_days: 2 })
        .await
        .unwrap();
    let (_, body) = status(&state, &admin, Some(96)).await;
    assert_eq!(
        (body["hours"].as_u64(), body["kept_days"].as_u64()),
        (Some(48), Some(2))
    );
    assert_eq!(route(&body, "GET /api/tree").unwrap()["took"]["count"], 5);
    let (answer, _) = status(&state, &admin, Some(u32::MAX)).await;
    assert_eq!(answer, StatusCode::OK);
}

#[sqlx::test]
async fn a_reader_sees_no_status_over_any_time(db: PgPool) {
    let _ = admin(&db).await;
    let ada = common::account(&db, "Ada").await;
    let reader = common::session(&db, ada).await;
    let state = AppState::new(db.clone());
    saved_requests(&db, time::Duration::hours(2), "GET /api/tree", 5).await;
    let (answer, body) = status(&state, &reader, Some(24)).await;
    assert_eq!(answer, StatusCode::FORBIDDEN);
    assert!(!body.to_string().contains("tree"));
}

#[sqlx::test]
async fn what_is_older_than_kept_goes(db: PgPool) {
    saved_requests(&db, time::Duration::days(3), "GET /api/tree", 1).await;
    saved_requests(&db, time::Duration::days(10), "GET /api/tree", 1).await;
    let state = AppState::new(db.clone());
    let problems = &state.monitor.problems;
    problems.note(Severity::Warning, "rueder", "now", Instant::now());
    history::save(&db, &state.monitor, None).await.unwrap();
    sqlx::query(
        "INSERT INTO status_problems (position, error, target, text, count, first_at, last_at)
         VALUES (1, true, 'rueder', 'long ago', 1, now() - interval '20 days',
                 now() - interval '10 days')",
    )
    .execute(&db)
    .await
    .unwrap();
    history::load(&db, &state.monitor, None).await.unwrap();
    assert_eq!(problems.list().len(), 2);

    // Kept for seven days, as is the default.
    assert_eq!(
        retention::settings(&db).await.unwrap(),
        retention::Settings { status_days: 7 }
    );
    assert_eq!(history::sweep(&db, problems).await.unwrap(), 1);
    let left: Vec<OffsetDateTime> = sqlx::query_scalar("SELECT starts_at FROM status_slots")
        .fetch_all(&db)
        .await
        .unwrap();
    assert!(
        left.iter()
            .all(|at| *at > OffsetDateTime::now_utc() - time::Duration::days(4))
    );
    let texts: Vec<String> = problems.list().into_iter().map(|p| p.text).collect();
    assert_eq!(texts, ["now"]);
    let kept: Vec<String> = sqlx::query_scalar("SELECT text FROM status_problems")
        .fetch_all(&db)
        .await
        .unwrap();
    assert_eq!(kept, ["now"]);

    // Kept for a day, the one of three days ago goes too.
    retention::set(&db, &retention::Settings { status_days: 1 })
        .await
        .unwrap();
    assert!(history::sweep(&db, problems).await.unwrap() >= 1);
    // Out of bounds is refused, here and in the table.
    for days in [0, 3651] {
        assert!(
            retention::set(&db, &retention::Settings { status_days: days })
                .await
                .is_err()
        );
    }
    assert!(
        sqlx::query("UPDATE retention SET status_days = 0")
            .execute(&db)
            .await
            .is_err()
    );
    assert_eq!(retention::settings(&db).await.unwrap().status_days, 1);
}
