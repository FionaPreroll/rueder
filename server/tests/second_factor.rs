//! The second factor: setting it up, logging in with it, recovery codes,
//! switching it off, and admins who need it.

mod common;

use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    body::Body,
    http::{HeaderMap, Method, Request, StatusCode, header},
};
use common::call;
use http_body_util::BodyExt;
use rueder::{AppState, accounts, second_factor};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

const PASSWORD: &str = "correct horse battery";

async fn log_in(
    state: &AppState,
    name: &str,
    code: Option<&str>,
) -> (StatusCode, HeaderMap, String) {
    let mut body = json!({ "username": name, "password": PASSWORD });
    if let Some(code) = code {
        body["code"] = json!(code);
    }
    let request = Request::post("/api/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let app = rueder::router(state.clone(), "missing".as_ref());
    let response = app.oneshot(request).await.unwrap();
    let (status, headers) = (response.status(), response.headers().clone());
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, headers, String::from_utf8(bytes.to_vec()).unwrap())
}

fn cookie(headers: &HeaderMap) -> String {
    let set = headers.get(header::SET_COOKIE).unwrap().to_str().unwrap();
    set.split(';').next().unwrap().to_owned()
}

fn now_step() -> u64 {
    let unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    second_factor::step_of(unix)
}

/// The code an app would show for the account's secret, `ahead` steps on.
async fn app_code(db: &PgPool, account: i32, pending: bool, ahead: u64) -> String {
    let query = if pending {
        "SELECT totp_pending FROM accounts WHERE id = $1"
    } else {
        "SELECT totp_secret FROM accounts WHERE id = $1"
    };
    let secret: Vec<u8> = sqlx::query_scalar(query)
        .bind(account)
        .fetch_one(db)
        .await
        .unwrap();
    format!("{:06}", second_factor::code_at(&secret, now_step() + ahead))
}

/// Sets the second factor up for a logged-in account and returns its
/// recovery codes.
async fn set_up(state: &AppState, account: i32, cookie: &str) -> Vec<String> {
    let body = json!({ "password": PASSWORD });
    let (status, setup) = call(
        state,
        Method::POST,
        "/api/second-factor/setup",
        Some(cookie),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{setup}");
    let code = app_code(&state.db, account, true, 0).await;
    let (status, codes) = call(
        state,
        Method::POST,
        "/api/second-factor/confirm",
        Some(cookie),
        Some(json!({ "code": code })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{codes}");
    codes["codes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|code| code.as_str().unwrap().to_owned())
        .collect()
}

#[sqlx::test]
async fn it_is_set_up_with_the_password_and_a_code(db: PgPool) {
    let ada = accounts::create(&db, "ada", Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap();
    let state = AppState::new(db.clone());
    let (_, headers, _) = log_in(&state, "ada", None).await;
    let ada_cookie = cookie(&headers);
    let cookie = Some(ada_cookie.as_str());

    let (status, off) = call(&state, Method::GET, "/api/second-factor", cookie, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        off,
        json!({ "on": false, "recovery_codes_left": 0, "required_for_admins": false })
    );

    // Not without the password.
    let wrong = json!({ "password": "wrong" });
    let (status, _) = call(
        &state,
        Method::POST,
        "/api/second-factor/setup",
        cookie,
        Some(wrong),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    // Nothing to confirm before it is set up.
    let code = json!({ "code": "123456" });
    let (status, _) = call(
        &state,
        Method::POST,
        "/api/second-factor/confirm",
        cookie,
        Some(code),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    let body = json!({ "password": PASSWORD });
    let (status, setup) = call(
        &state,
        Method::POST,
        "/api/second-factor/setup",
        cookie,
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let secret = setup["secret"].as_str().unwrap();
    assert_eq!(secret.len(), 32);
    assert_eq!(
        setup["uri"],
        format!(
            "otpauth://totp/rueder%3Aada?secret={secret}&issuer=rueder&algorithm=SHA1&digits=6&period=30"
        )
    );
    assert!(setup["qr_svg"].as_str().unwrap().starts_with("<?xml"));
    // Waiting for its code, it is not on yet: logging in needs no code.
    assert_eq!(log_in(&state, "ada", None).await.0, StatusCode::OK);

    // A wrong code does not confirm it.
    let pending = app_code(&db, ada, true, 0).await;
    let wrong = format!("{:06}", (pending.parse::<u32>().unwrap() + 1) % 1_000_000);
    let (status, _) = call(
        &state,
        Method::POST,
        "/api/second-factor/confirm",
        cookie,
        Some(json!({ "code": wrong })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, codes) = call(
        &state,
        Method::POST,
        "/api/second-factor/confirm",
        cookie,
        Some(json!({ "code": pending })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let codes = codes["codes"].as_array().unwrap();
    assert_eq!(codes.len(), second_factor::RECOVERY_CODES);
    // Only their hashes are kept.
    let kept: Vec<Vec<u8>> = sqlx::query_scalar("SELECT code_hash FROM recovery_codes")
        .fetch_all(&db)
        .await
        .unwrap();
    assert_eq!(kept.len(), 10);
    for hash in &kept {
        assert_eq!(hash.len(), 32);
    }
    let (_, on) = call(&state, Method::GET, "/api/second-factor", cookie, None).await;
    assert_eq!(on["on"], true);
    assert_eq!(on["recovery_codes_left"], 10);

    // Another account sees nothing of it and has its own.
    let grace = common::account(&db, "grace").await;
    let grace_cookie = common::session(&db, grace).await;
    let (_, theirs) = call(
        &state,
        Method::GET,
        "/api/second-factor",
        Some(&grace_cookie),
        None,
    )
    .await;
    assert_eq!(theirs["on"], false);
    assert_eq!(theirs["recovery_codes_left"], 0);
    // Nor any of it without a session.
    for (method, path) in [
        (Method::GET, "/api/second-factor"),
        (Method::POST, "/api/second-factor/setup"),
        (Method::POST, "/api/second-factor/confirm"),
        (Method::DELETE, "/api/second-factor"),
    ] {
        let (status, _) = call(&state, method, path, None, Some(json!({}))).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{path}");
    }
}

#[sqlx::test]
async fn logging_in_takes_a_code_once_or_a_recovery_code(db: PgPool) {
    let ada = accounts::create(&db, "ada", Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap();
    let state = AppState::new(db.clone());
    let first = cookie(&log_in(&state, "ada", None).await.1);
    let codes = set_up(&state, ada, &first).await;

    // The password alone is no longer enough, and says what is missing.
    let (status, headers, body) = log_in(&state, "ada", None).await;
    assert_eq!(
        (status, body.as_str()),
        (StatusCode::UNAUTHORIZED, "second factor needed")
    );
    assert!(headers.get(header::SET_COOKIE).is_none());
    // A wrong password still says only that.
    let wrong = Request::post("/api/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "username": "ada", "password": "nope", "code": "123456" }).to_string(),
        ))
        .unwrap();
    let response = rueder::router(state.clone(), "missing".as_ref())
        .oneshot(wrong)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // The code the confirmation used counts no more; the next one does, once.
    let used = app_code(&db, ada, false, 0).await;
    assert_eq!(
        log_in(&state, "ada", Some(&used)).await.0,
        StatusCode::UNAUTHORIZED
    );
    let next = app_code(&db, ada, false, 1).await;
    let (status, headers, me) = log_in(&state, "ada", Some(&next)).await;
    assert_eq!(status, StatusCode::OK, "{me}");
    let with_code = cookie(&headers);
    let second: bool =
        sqlx::query_scalar("SELECT second_factor FROM sessions ORDER BY created_at DESC LIMIT 1")
            .fetch_one(&db)
            .await
            .unwrap();
    assert!(second);
    assert_eq!(
        log_in(&state, "ada", Some(&next)).await.0,
        StatusCode::UNAUTHORIZED
    );

    // A recovery code counts once, written any way.
    let recovery = codes[3].to_lowercase().replace('-', " ");
    assert_eq!(
        log_in(&state, "ada", Some(&recovery)).await.0,
        StatusCode::OK
    );
    assert_eq!(
        log_in(&state, "ada", Some(&codes[3])).await.0,
        StatusCode::UNAUTHORIZED
    );
    let (_, left) = call(
        &state,
        Method::GET,
        "/api/second-factor",
        Some(&with_code),
        None,
    )
    .await;
    assert_eq!(left["recovery_codes_left"], 9);
    // Something too long is not even looked at.
    let long = "1".repeat(1000);
    assert_eq!(
        log_in(&state, "ada", Some(&long)).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test]
async fn wrong_codes_count_like_wrong_passwords(db: PgPool) {
    let ada = accounts::create(&db, "ada", Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap();
    let state = AppState::new(db.clone());
    let first = cookie(&log_in(&state, "ada", None).await.1);
    set_up(&state, ada, &first).await;
    let right = app_code(&db, ada, false, 1).await;
    let wrong = format!(
        "{:06}",
        (right.parse::<u32>().unwrap() + 500_000) % 1_000_000
    );

    // Ten wrong tries for a name in fifteen minutes, as for passwords; the
    // one that asked for the code took nothing of them.
    let mut answers = Vec::new();
    for _ in 0..11 {
        answers.push(log_in(&state, "ada", Some(&wrong)).await.0);
    }
    assert!(
        answers[..9]
            .iter()
            .all(|status| *status == StatusCode::UNAUTHORIZED),
        "{answers:?}"
    );
    assert_eq!(answers[10], StatusCode::TOO_MANY_REQUESTS, "{answers:?}");
    // Then not even the right code gets through.
    assert_eq!(
        log_in(&state, "ada", Some(&right)).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[sqlx::test]
async fn it_is_switched_off_with_the_password_and_a_code_or_on_the_command_line(db: PgPool) {
    let ada = accounts::create(&db, "ada", Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap();
    let state = AppState::new(db.clone());
    let first = cookie(&log_in(&state, "ada", None).await.1);
    let codes = set_up(&state, ada, &first).await;
    let off = |body: Value| {
        let (state, first) = (state.clone(), first.clone());
        async move {
            call(
                &state,
                Method::DELETE,
                "/api/second-factor",
                Some(&first),
                Some(body),
            )
            .await
            .0
        }
    };
    let next = app_code(&db, ada, false, 1).await;

    // A wrong password, the password alone, a wrong code: all refused.
    assert_eq!(
        off(json!({ "password": "wrong", "code": next })).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        off(json!({ "password": PASSWORD })).await,
        StatusCode::FORBIDDEN
    );
    let wrong = if next == "000000" { "111111" } else { "000000" };
    assert_eq!(
        off(json!({ "password": PASSWORD, "code": wrong })).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        off(json!({ "password": PASSWORD, "code": codes[0] })).await,
        StatusCode::NO_CONTENT
    );
    let (_, now) = call(
        &state,
        Method::GET,
        "/api/second-factor",
        Some(&first),
        None,
    )
    .await;
    assert_eq!(now["on"], false);
    assert_eq!(now["recovery_codes_left"], 0);
    assert_eq!(log_in(&state, "ada", None).await.0, StatusCode::OK);
    // The sessions do not count as ones with it any more.
    let any: bool = sqlx::query_scalar("SELECT bool_or(second_factor) FROM sessions")
        .fetch_one(&db)
        .await
        .unwrap();
    assert!(!any);

    // A code of the app does it too.
    set_up(&state, ada, &first).await;
    let next = app_code(&db, ada, false, 1).await;
    assert_eq!(
        off(json!({ "password": PASSWORD, "code": next })).await,
        StatusCode::NO_CONTENT
    );

    // On the command line, for somebody who lost both app and codes.
    set_up(&state, ada, &first).await;
    assert!(second_factor::switch_off_named(&db, " ADA ").await.unwrap());
    assert!(
        !second_factor::switch_off_named(&db, "nobody")
            .await
            .unwrap()
    );
    assert_eq!(log_in(&state, "ada", None).await.0, StatusCode::OK);
}

#[sqlx::test]
async fn admins_need_it_where_the_operator_says_so(db: PgPool) {
    let root = accounts::create(&db, "root", Some(PASSWORD), true)
        .await
        .unwrap()
        .unwrap();
    let lax = AppState::new(db.clone());
    let strict = AppState::new(db.clone()).with_admin_second_factor();
    let without = cookie(&log_in(&strict, "root", None).await.1);

    // As rueder comes, an admin is one with the password alone.
    let (status, _) = call(&lax, Method::GET, "/api/status", Some(&without), None).await;
    assert_eq!(status, StatusCode::OK);
    // Where the operator wants it, not without it, and the page says why.
    let (status, _) = call(&strict, Method::GET, "/api/status", Some(&without), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (_, me) = call(&strict, Method::GET, "/api/me", Some(&without), None).await;
    assert_eq!(
        (&me["is_admin"], &me["admin_needs_second_factor"]),
        (&Value::Bool(false), &Value::Bool(true))
    );
    let (_, state) = call(
        &strict,
        Method::GET,
        "/api/second-factor",
        Some(&without),
        None,
    )
    .await;
    assert_eq!(state["required_for_admins"], true);

    // Setting it up makes the session that confirmed it one with it.
    set_up(&strict, root, &without).await;
    let (status, _) = call(&strict, Method::GET, "/api/status", Some(&without), None).await;
    assert_eq!(status, StatusCode::OK);
    // And so is a login with a code.
    let code = app_code(&db, root, false, 1).await;
    let (status, headers, me) = log_in(&strict, "root", Some(&code)).await;
    assert_eq!(status, StatusCode::OK);
    let me: Value = serde_json::from_str(&me).unwrap();
    assert_eq!(
        (&me["is_admin"], &me["admin_needs_second_factor"]),
        (&Value::Bool(true), &Value::Bool(false))
    );
    let (status, _) = call(
        &strict,
        Method::GET,
        "/api/status",
        Some(&cookie(&headers)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // A session without it, as through the provider, is no admin's.
    let other = common::session(&db, root).await;
    let (status, _) = call(&strict, Method::GET, "/api/status", Some(&other), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    // A reader is not held back by any of it.
    let grace = common::account(&db, "grace").await;
    let reader = common::session(&db, grace).await;
    let (_, me) = call(&strict, Method::GET, "/api/me", Some(&reader), None).await;
    assert_eq!(me["admin_needs_second_factor"], false);
}
