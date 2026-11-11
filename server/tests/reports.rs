//! Reports of errors from browsers: only with consent, scrubbed, counted, and
//! on the status page without naming anybody.

mod common;

use std::time::Duration;

use axum::http::{Method, StatusCode};
use common::call;
use rueder::{
    AppState, accounts,
    rate::{Rate, Rates},
    reports,
};
use serde_json::{Value, json};
use sqlx::PgPool;

async fn put_consent(state: &AppState, cookie: &str, on: bool) -> StatusCode {
    let body = json!({ "on": on });
    let path = "/api/reports/consent";
    call(state, Method::PUT, path, Some(cookie), Some(body))
        .await
        .0
}

async fn consent(state: &AppState, cookie: &str) -> Value {
    let path = "/api/reports/consent";
    call(state, Method::GET, path, Some(cookie), None).await.1
}

async fn send(state: &AppState, cookie: &str, reports: Value) -> StatusCode {
    let body = json!({ "reports": reports });
    call(
        state,
        Method::POST,
        "/api/reports",
        Some(cookie),
        Some(body),
    )
    .await
    .0
}

fn report(message: &str) -> Value {
    json!({
        "kind": "error",
        "message": message,
        "stack": "_app/immutable/chunks/list.js:12:34\nhttps://tracker.example/x.js:1:1",
        "version": "1731340800000",
        "browser": "Firefox 140",
    })
}

async fn rows(db: &PgPool) -> Vec<(i32, String, String, i32)> {
    sqlx::query_as(
        "SELECT account_id, message, stack, count FROM client_reports ORDER BY account_id, id",
    )
    .fetch_all(db)
    .await
    .unwrap()
}

#[sqlx::test]
async fn only_an_account_that_agreed_sends_reports(db: PgPool) {
    let ada = common::account(&db, "ada").await;
    let grace = common::account(&db, "grace").await;
    let state = AppState::new(db.clone());
    let (mine, theirs) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
    );

    // Off until the reader switches it on.
    assert_eq!(
        consent(&state, &mine).await,
        json!({ "available": true, "on": false })
    );
    assert_eq!(
        send(&state, &mine, json!([report("boom")])).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        put_consent(&state, &mine, true).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(consent(&state, &mine).await["on"], true);
    // Another account's consent is its own.
    assert_eq!(consent(&state, &theirs).await["on"], false);
    assert_eq!(
        send(&state, &theirs, json!([report("boom")])).await,
        StatusCode::FORBIDDEN
    );

    assert_eq!(
        send(&state, &mine, json!([report("boom"), report("boom")])).await,
        StatusCode::NO_CONTENT
    );
    // The same error counts up in one row, with only the frontend's own files.
    assert_eq!(
        rows(&db).await,
        vec![(
            ada,
            "boom".to_owned(),
            "_app/immutable/chunks/list.js:12:34".to_owned(),
            2
        )]
    );

    // Off again deletes what the account sent.
    assert_eq!(
        put_consent(&state, &mine, false).await,
        StatusCode::NO_CONTENT
    );
    assert!(rows(&db).await.is_empty());
    assert_eq!(
        send(&state, &mine, json!([report("boom")])).await,
        StatusCode::FORBIDDEN
    );

    // Without a session, nothing.
    let (status, _) = call(&state, Method::GET, "/api/reports/consent", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn a_report_keeps_no_address_and_breaks_no_limit(db: PgPool) {
    let ada = common::account(&db, "ada").await;
    let state = AppState::new(db.clone())
        .with_rates(Rates::default().with_reports(Rate::new(3, Duration::from_secs(3600))));
    let cookie = common::session(&db, ada).await;
    put_consent(&state, &cookie, true).await;

    let sneaky = json!({
        "kind": "request",
        "message": format!("GET https://ada:secret@example.org/feeds/12?x=1 /feeds/34/articles/5678: 502 {}", "x".repeat(900)),
        "stack": "",
        "version": "<script>1.0</script>",
        "browser": "",
    });
    assert_eq!(
        send(&state, &cookie, json!([sneaky])).await,
        StatusCode::NO_CONTENT
    );
    let (message, version, browser): (String, String, String) =
        sqlx::query_as("SELECT message, version, browser FROM client_reports")
            .fetch_one(&db)
            .await
            .unwrap();
    assert!(
        message.starts_with("GET <address> /feeds/<n>/articles/<n>: 502 xxx"),
        "{message}"
    );
    assert_eq!(message.chars().count(), reports::MAX_MESSAGE);
    assert_eq!(
        (version.as_str(), browser.as_str()),
        ("script1.0script", "unknown")
    );

    // Too many at once, of a kind nobody knows, or too often.
    let many: Vec<Value> = (0..=reports::MAX_REPORTS)
        .map(|n| report(&format!("e{n}")))
        .collect();
    assert_eq!(
        send(&state, &cookie, json!(many)).await,
        StatusCode::BAD_REQUEST
    );
    let mut odd = report("odd");
    odd["kind"] = json!("virus");
    assert_eq!(
        send(&state, &cookie, json!([odd])).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        send(&state, &cookie, json!([report("a"), report("b")])).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        send(&state, &cookie, json!([report("c")])).await,
        StatusCode::TOO_MANY_REQUESTS
    );
    // A body larger than the reports may be is refused before it is read.
    let large = json!([report(&"y".repeat(reports::MAX_BODY))]);
    assert_eq!(
        send(&state, &cookie, large).await,
        StatusCode::PAYLOAD_TOO_LARGE
    );
}

#[sqlx::test]
async fn a_server_can_take_no_reports_at_all(db: PgPool) {
    let ada = common::account(&db, "ada").await;
    let state = AppState::new(db.clone()).without_reports();
    let cookie = common::session(&db, ada).await;
    assert_eq!(
        consent(&state, &cookie).await,
        json!({ "available": false, "on": false })
    );
    assert_eq!(
        put_consent(&state, &cookie, true).await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(&state, &cookie, json!([report("boom")])).await,
        StatusCode::NOT_FOUND
    );
    // Off is always fine.
    assert_eq!(
        put_consent(&state, &cookie, false).await,
        StatusCode::NO_CONTENT
    );
}

#[sqlx::test]
async fn admins_see_the_errors_but_not_whose(db: PgPool) {
    let root = common::account(&db, "root").await;
    accounts::set_admin(&db, "root", true).await.unwrap();
    let ada = common::account(&db, "ada-secret").await;
    let grace = common::account(&db, "grace-secret").await;
    let state = AppState::new(db.clone());
    for account in [ada, grace] {
        let cookie = common::session(&db, account).await;
        put_consent(&state, &cookie, true).await;
        send(&state, &cookie, json!([report("boom")])).await;
    }
    let mut chrome = report("boom");
    chrome["browser"] = json!("Chrome 141");
    send(&state, &common::session(&db, ada).await, json!([chrome])).await;

    let admin = common::session(&db, root).await;
    let (status, body) = call(&state, Method::GET, "/api/status", Some(&admin), None).await;
    assert_eq!(status, StatusCode::OK);
    let browsers = &body["browsers"];
    assert_eq!(browsers["available"], true);
    assert_eq!(browsers["consenting"], 2);
    assert_eq!(browsers["errors"].as_array().unwrap().len(), 1);
    let error = &browsers["errors"][0];
    assert_eq!(error["message"], "boom");
    assert_eq!(error["kind"], "error");
    assert_eq!(
        (error["count"].clone(), error["accounts"].clone()),
        (json!(3), json!(2))
    );
    assert_eq!(error["browsers"], json!(["Chrome 141", "Firefox 140"]));
    let hints: Vec<&str> = body["hints"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|hint| hint["kind"].as_str())
        .collect();
    assert!(hints.contains(&"browser_errors"), "{hints:?}");
    assert!(!body.to_string().contains("secret"));
}

#[sqlx::test]
async fn reports_go_after_thirty_days(db: PgPool) {
    let ada = common::account(&db, "ada").await;
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;
    put_consent(&state, &cookie, true).await;
    send(&state, &cookie, json!([report("old"), report("new")])).await;
    sqlx::query(
        "UPDATE client_reports SET last_at = now() - interval '31 days' WHERE message = 'old'",
    )
    .execute(&db)
    .await
    .unwrap();
    assert_eq!(reports::sweep(&db).await.unwrap(), 1);
    let left: Vec<String> = rows(&db).await.into_iter().map(|row| row.1).collect();
    assert_eq!(left, ["new"]);

    // An account that goes takes its reports along.
    sqlx::query("DELETE FROM accounts WHERE id = $1")
        .bind(ada)
        .execute(&db)
        .await
        .unwrap();
    assert!(rows(&db).await.is_empty());
}
