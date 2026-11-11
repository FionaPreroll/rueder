//! The own account: sessions, a new password, and what only an admin does.

mod common;

use axum::http::{Method, StatusCode, header};
use common::{call, raw};
use rueder::{AppState, accounts, sessions};
use serde_json::{Value, json};
use sqlx::PgPool;

async fn get(state: &AppState, path: &str, cookie: &str) -> (StatusCode, Value) {
    call(state, Method::GET, path, Some(cookie), None).await
}

async fn answer(state: &AppState, method: Method, path: &str, cookie: &str) -> StatusCode {
    call(state, method, path, Some(cookie), None).await.0
}

async fn logged_in(state: &AppState, cookie: &str) -> bool {
    get(state, "/api/me", cookie).await.0 == StatusCode::OK
}

#[sqlx::test]
async fn a_reader_sees_and_ends_their_own_sessions(db: PgPool) {
    let ada = common::account(&db, "ada").await;
    let grace = common::account(&db, "grace").await;
    let state = AppState::new(db.clone());
    let laptop = common::session(&db, ada).await;
    let phone = common::session(&db, ada).await;
    let tablet = common::session(&db, ada).await;
    let theirs = common::session(&db, grace).await;

    let (status, list) = get(&state, "/api/sessions", &laptop).await;
    assert_eq!(status, StatusCode::OK);
    let list = list.as_array().unwrap().clone();
    assert_eq!(list.len(), 3);
    assert_eq!(list.iter().filter(|s| s["current"] == true).count(), 1);
    // What names a session is not its token and says nothing about it.
    for session in &list {
        let id = session["id"].as_str().unwrap();
        assert_eq!(id.len(), 16);
        for cookie in [&laptop, &phone, &tablet] {
            assert!(!cookie.contains(id));
        }
        assert!(session["created_at"].is_string() && session["last_seen_at"].is_string());
    }
    let (_, phones) = get(&state, "/api/sessions", &phone).await;
    let other = |list: &Value| {
        let sessions = list.as_array().unwrap().iter();
        let other = sessions.clone().find(|s| s["current"] == true).unwrap();
        other["id"].as_str().unwrap().to_owned()
    };
    let phone_id = other(&phones);

    // Grace sees her one session, and cannot end Ada's.
    let (_, graces) = get(&state, "/api/sessions", &theirs).await;
    assert_eq!(graces.as_array().unwrap().len(), 1);
    let path = format!("/api/sessions/{phone_id}");
    assert_eq!(
        answer(&state, Method::DELETE, &path, &theirs).await,
        StatusCode::NOT_FOUND
    );
    assert!(logged_in(&state, &phone).await);

    // Ada ends the phone's from the laptop.
    assert_eq!(
        answer(&state, Method::DELETE, &path, &laptop).await,
        StatusCode::NO_CONTENT
    );
    assert!(!logged_in(&state, &phone).await);
    assert!(logged_in(&state, &laptop).await && logged_in(&state, &tablet).await);
    assert_eq!(
        answer(&state, Method::DELETE, &path, &laptop).await,
        StatusCode::NOT_FOUND
    );

    // And all others at once.
    assert_eq!(
        answer(&state, Method::DELETE, "/api/sessions", &laptop).await,
        StatusCode::NO_CONTENT
    );
    assert!(logged_in(&state, &laptop).await);
    assert!(!logged_in(&state, &tablet).await);
    assert!(logged_in(&state, &theirs).await);

    for (method, path) in [
        (Method::GET, "/api/sessions"),
        (Method::DELETE, "/api/sessions"),
        (Method::DELETE, path.as_str()),
    ] {
        let (status, _) = call(&state, method, path, None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}

/// Asks for a new password and returns the status and the new cookie, if any.
async fn change(state: &AppState, cookie: &str, body: Value) -> (StatusCode, Option<String>) {
    let body = Some(("application/json", body.to_string().into_bytes()));
    let (status, headers, _) = raw(state, Method::PUT, "/api/password", Some(cookie), body).await;
    let cookie = headers.get(header::SET_COOKIE).map(|cookie| {
        let cookie = cookie.to_str().unwrap();
        assert!(cookie.contains("HttpOnly") && cookie.contains("Secure"));
        cookie.split(';').next().unwrap().to_owned()
    });
    (status, cookie)
}

#[sqlx::test]
async fn a_new_password_needs_the_old_one_and_ends_the_other_sessions(db: PgPool) {
    let ada = accounts::create(&db, "ada", Some("correct horse"), false)
        .await
        .unwrap()
        .unwrap();
    let grace = accounts::create(&db, "grace", Some("battery staple"), false)
        .await
        .unwrap()
        .unwrap();
    let state = AppState::new(db.clone());
    let laptop = common::session(&db, ada).await;
    let phone = common::session(&db, ada).await;
    let theirs = common::session(&db, grace).await;

    for (wrong, expected) in [
        (
            json!({ "current": "battery staple", "new": "a new password" }),
            StatusCode::FORBIDDEN,
        ),
        (json!({ "new": "a new password" }), StatusCode::FORBIDDEN),
        (
            json!({ "current": "correct horse", "new": "short" }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "current": "correct horse" }),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ] {
        let (status, cookie) = change(&state, &laptop, wrong.clone()).await;
        assert_eq!((status, cookie), (expected, None), "{wrong}");
    }
    assert!(
        accounts::login(&db, "ada", "correct horse")
            .await
            .unwrap()
            .is_some()
    );
    assert!(logged_in(&state, &phone).await);

    let right = json!({ "current": "correct horse", "new": "a new password" });
    let (status, new) = change(&state, &laptop, right).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let new = new.unwrap();
    assert!(new.starts_with(&format!("{}=", sessions::COOKIE)));
    // Whoever was logged in with the old password is out; this browser has a
    // session that is new.
    assert!(logged_in(&state, &new).await);
    assert!(!logged_in(&state, &laptop).await && !logged_in(&state, &phone).await);
    assert!(
        accounts::login(&db, "ada", "correct horse")
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        accounts::login(&db, "ada", "a new password")
            .await
            .unwrap()
            .is_some()
    );
    // Nothing of that touched Grace.
    assert!(logged_in(&state, &theirs).await);
    assert!(
        accounts::login(&db, "grace", "battery staple")
            .await
            .unwrap()
            .is_some()
    );

    let body = json!({ "current": "a new password", "new": "yet another one" });
    let (status, _) = call(&state, Method::PUT, "/api/password", None, Some(body)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn a_stolen_session_cannot_try_passwords_without_end(db: PgPool) {
    let ada = accounts::create(&db, "ada", Some("correct horse"), false)
        .await
        .unwrap()
        .unwrap();
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;
    let mut last = StatusCode::OK;
    for n in 0..rueder::login::PER_NAME + 1 {
        let guess = json!({ "current": format!("guess number {n}"), "new": "a new password" });
        last = change(&state, &cookie, guess).await.0;
    }
    assert_eq!(last, StatusCode::TOO_MANY_REQUESTS);
    // Now not even the right one gets through, for a while.
    let right = json!({ "current": "correct horse", "new": "a new password" });
    assert_eq!(
        change(&state, &cookie, right).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert!(
        accounts::login(&db, "ada", "correct horse")
            .await
            .unwrap()
            .is_some()
    );
}

#[sqlx::test]
async fn an_account_without_a_password_can_set_one(db: PgPool) {
    // As after a login through OpenID Connect.
    let ada = common::account(&db, "ada").await;
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;
    let (status, new) = change(&state, &cookie, json!({ "new": "a first password" })).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(logged_in(&state, &new.unwrap()).await);
    assert!(
        accounts::login(&db, "ada", "a first password")
            .await
            .unwrap()
            .is_some()
    );
}

#[sqlx::test]
async fn only_an_admin_lists_and_makes_accounts(db: PgPool) {
    let root = accounts::create(&db, "root", Some("correct horse"), true)
        .await
        .unwrap()
        .unwrap();
    let ada = common::account(&db, "ada").await;
    let state = AppState::new(db.clone());
    let (admin, reader) = (
        common::session(&db, root).await,
        common::session(&db, ada).await,
    );
    let new = json!({ "username": "grace", "password": "battery staple" });
    let add = async |cookie: Option<&str>, body: &Value| {
        call(
            &state,
            Method::POST,
            "/api/accounts",
            cookie,
            Some(body.clone()),
        )
        .await
    };

    // A reader neither sees who else is here nor lets anybody in.
    assert_eq!(
        get(&state, "/api/accounts", &reader).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(add(Some(&reader), &new).await.0, StatusCode::FORBIDDEN);
    assert_eq!(add(None, &new).await.0, StatusCode::UNAUTHORIZED);
    let (status, _) = call(&state, Method::GET, "/api/accounts", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(
        accounts::login(&db, "grace", "battery staple")
            .await
            .unwrap()
            .is_none()
    );

    let (status, made) = add(Some(&admin), &new).await;
    assert_eq!(status, StatusCode::CREATED);
    let grace = accounts::login(&db, "grace", "battery staple")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (made["id"].as_i64(), grace.is_admin),
        (Some(grace.id as i64), false)
    );

    for wrong in [
        json!({ "username": "Grace", "password": "battery staple" }),
        json!({ "username": "heidi", "password": "short" }),
        json!({ "username": "  ", "password": "battery staple" }),
    ] {
        assert_eq!(
            add(Some(&admin), &wrong).await.0,
            StatusCode::BAD_REQUEST,
            "{wrong}"
        );
    }
    assert_eq!(
        add(Some(&admin), &json!({ "username": "heidi" })).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let (status, list) = get(&state, "/api/accounts", &admin).await;
    assert_eq!(status, StatusCode::OK);
    let seen: Vec<_> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|a| {
            (
                a["username"].as_str().unwrap(),
                a["is_admin"] == true,
                a["has_password"] == true,
            )
        })
        .collect();
    assert_eq!(
        seen,
        [
            ("ada", false, false),
            ("grace", false, true),
            ("root", true, true)
        ]
    );
    // The list has no hashes.
    assert!(!list.to_string().contains("argon2"));
}
