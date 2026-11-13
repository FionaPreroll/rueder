//! App passwords: made with the account's password and second factor, listed
//! without the secret, revoked, and ended with a new password or the account.

mod common;

use axum::http::{Method, StatusCode};
use common::call;
use rueder::{AppState, accounts, app_passwords, second_factor};
use serde_json::{Value, json};
use sqlx::PgPool;

const PASSWORD: &str = "correct horse battery";

async fn make(state: &AppState, cookie: &str, body: Value) -> (StatusCode, Value) {
    call(
        state,
        Method::POST,
        "/api/app-passwords",
        Some(cookie),
        Some(body),
    )
    .await
}

#[sqlx::test]
async fn an_app_password_is_made_with_the_password_and_shown_once(db: PgPool) {
    let ada = accounts::create(&db, "ada", Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap();
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;

    // Without the password, or with a wrong one, there is none.
    let reeder = json!({ "name": "Reeder", "password": "wrong" });
    assert_eq!(make(&state, &cookie, reeder).await.0, StatusCode::FORBIDDEN);
    let unnamed = json!({ "name": " ", "password": PASSWORD });
    assert_eq!(
        make(&state, &cookie, unnamed).await.0,
        StatusCode::BAD_REQUEST
    );

    let (status, made) = make(
        &state,
        &cookie,
        json!({ "name": " Reeder ", "password": PASSWORD }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    assert_eq!(made["name"], "Reeder");
    assert_eq!(made["used_at"], Value::Null);
    let secret = made["secret"].as_str().unwrap().to_owned();
    assert_eq!(secret.len(), 29);

    // The list never shows it again.
    let (status, list) = call(
        &state,
        Method::GET,
        "/api/app-passwords",
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list["max"], 20);
    assert_eq!(list["app_passwords"].as_array().unwrap().len(), 1);
    assert_eq!(list["app_passwords"][0]["name"], "Reeder");
    assert!(!list.to_string().contains(&secret));
    let stored: Vec<u8> = sqlx::query_scalar("SELECT secret_hash FROM app_passwords")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(stored.len(), 32);

    // An app logs in with it, however it is typed; the first time says so.
    let found = app_passwords::find(&db, &secret.to_uppercase())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.session.account_id, ada);
    assert_eq!(found.session.username, "ada");
    assert!(found.first_use && !found.session.is_admin);
    let again = app_passwords::find(&db, &secret.replace('-', ""))
        .await
        .unwrap()
        .unwrap();
    assert!(!again.first_use);
    assert!(app_passwords::counts(&db, found.id).await.unwrap());
    let (_, list) = call(
        &state,
        Method::GET,
        "/api/app-passwords",
        Some(&cookie),
        None,
    )
    .await;
    assert!(list["app_passwords"][0]["used_at"].is_string());
    // The account's password is no app password.
    assert!(app_passwords::find(&db, PASSWORD).await.unwrap().is_none());

    // Revoked, it lets no app in any more.
    let path = format!("/api/app-passwords/{}", found.id);
    let (status, _) = call(&state, Method::DELETE, &path, Some(&cookie), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(&state, Method::DELETE, &path, Some(&cookie), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(app_passwords::find(&db, &secret).await.unwrap().is_none());
    assert!(!app_passwords::counts(&db, found.id).await.unwrap());
}

#[sqlx::test]
async fn others_see_nothing_and_revoke_nothing(db: PgPool) {
    let ada = accounts::create(&db, "ada", Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap();
    let grace = accounts::create(&db, "grace", Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap();
    let state = AppState::new(db.clone());
    let (ours, theirs) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
    );
    let (_, made) = make(
        &state,
        &ours,
        json!({ "name": "Reeder", "password": PASSWORD }),
    )
    .await;
    let id = made["id"].as_i64().unwrap();

    let (_, list) = call(
        &state,
        Method::GET,
        "/api/app-passwords",
        Some(&theirs),
        None,
    )
    .await;
    assert_eq!(list["app_passwords"], json!([]));
    let path = format!("/api/app-passwords/{id}");
    let (status, _) = call(&state, Method::DELETE, &path, Some(&theirs), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, list) = call(&state, Method::GET, "/api/app-passwords", Some(&ours), None).await;
    assert_eq!(list["app_passwords"].as_array().unwrap().len(), 1);

    // Without a session, nothing at all.
    for (method, path) in [
        (Method::GET, "/api/app-passwords".to_owned()),
        (Method::POST, "/api/app-passwords".to_owned()),
        (Method::DELETE, path),
    ] {
        let body = Some(json!({ "name": "x", "password": PASSWORD }));
        let (status, _) = call(&state, method, &path, None, body).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{path}");
    }
}

#[sqlx::test]
async fn an_account_has_at_most_twenty(db: PgPool) {
    let ada = accounts::create(&db, "ada", Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap();
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;
    for n in 0..app_passwords::MAX_PER_ACCOUNT {
        let body = json!({ "name": format!("App {n}"), "password": PASSWORD });
        assert_eq!(make(&state, &cookie, body).await.0, StatusCode::CREATED);
    }
    let body = json!({ "name": "One more", "password": PASSWORD });
    assert_eq!(make(&state, &cookie, body).await.0, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn with_a_second_factor_it_takes_a_code(db: PgPool) {
    let ada = accounts::create(&db, "ada", Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap();
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;
    let (status, _) = call(
        &state,
        Method::POST,
        "/api/second-factor/setup",
        Some(&cookie),
        Some(json!({ "password": PASSWORD })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let secret: Vec<u8> = sqlx::query_scalar("SELECT totp_pending FROM accounts WHERE id = $1")
        .bind(ada)
        .fetch_one(&db)
        .await
        .unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let step = second_factor::step_of(now);
    let code = |step: u64| format!("{:06}", second_factor::code_at(&secret, step));
    let (status, confirmed) = call(
        &state,
        Method::POST,
        "/api/second-factor/confirm",
        Some(&cookie),
        Some(json!({ "code": code(step) })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{confirmed}");

    let without = json!({ "name": "Reeder", "password": PASSWORD });
    let (status, _) = make(&state, &cookie, without).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _, body) = common::raw(
        &state,
        Method::POST,
        "/api/app-passwords",
        Some(&cookie),
        Some((
            "application/json",
            json!({ "name": "Reeder", "password": PASSWORD })
                .to_string()
                .into_bytes(),
        )),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body, b"second factor needed");
    let wrong = json!({ "name": "Reeder", "password": PASSWORD, "code": "000000x" });
    assert_eq!(make(&state, &cookie, wrong).await.0, StatusCode::FORBIDDEN);
    let recovery = confirmed["codes"][0].as_str().unwrap();
    let right = json!({ "name": "Reeder", "password": PASSWORD, "code": recovery });
    assert_eq!(make(&state, &cookie, right).await.0, StatusCode::CREATED);
}

#[sqlx::test]
async fn a_new_password_and_disabling_end_them(db: PgPool) {
    let ada = accounts::create(&db, "ada", Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap();
    let state = AppState::new(db.clone());
    let made = |state: AppState| {
        let db = db.clone();
        async move {
            let cookie = common::session(&db, ada).await;
            let body = json!({ "name": "Reeder", "password": PASSWORD });
            let (_, made) = make(&state, &cookie, body).await;
            made["secret"].as_str().unwrap().to_owned()
        }
    };

    let secret = made(state.clone()).await;
    assert!(app_passwords::find(&db, &secret).await.unwrap().is_some());
    accounts::set_password(&db, "ada", PASSWORD)
        .await
        .unwrap()
        .unwrap();
    assert!(app_passwords::find(&db, &secret).await.unwrap().is_none());

    let secret = made(state.clone()).await;
    accounts::set_disabled(&db, "ada", true).await.unwrap();
    assert!(app_passwords::find(&db, &secret).await.unwrap().is_none());
    accounts::set_disabled(&db, "ada", false).await.unwrap();
    assert!(app_passwords::find(&db, &secret).await.unwrap().is_none());

    let secret = made(state.clone()).await;
    assert!(accounts::remove(&db, ada).await.unwrap());
    assert!(app_passwords::find(&db, &secret).await.unwrap().is_none());
}

#[sqlx::test]
async fn without_the_app_api_there_are_none(db: PgPool) {
    let ada = accounts::create(&db, "ada", Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap();
    let state = AppState::new(db.clone()).without_app_api();
    let cookie = common::session(&db, ada).await;
    let body = json!({ "name": "Reeder", "password": PASSWORD });
    assert_eq!(make(&state, &cookie, body).await.0, StatusCode::NOT_FOUND);
    let (status, _) = call(
        &state,
        Method::GET,
        "/api/app-passwords",
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // And the page knows.
    let (_, me) = call(&state, Method::GET, "/api/me", Some(&cookie), None).await;
    assert_eq!(me["app_api"], false);
    let on = AppState::new(db.clone());
    let (_, me) = call(&on, Method::GET, "/api/me", Some(&cookie), None).await;
    assert_eq!(me["app_api"], true);
}
