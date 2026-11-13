//! Accounts, passwords, sessions and the login over the API.

mod common;

use std::net::IpAddr;

use axum::{
    body::Body,
    http::{HeaderMap, HeaderName, Request, StatusCode, header},
};
use http_body_util::BodyExt;
use rueder::{
    AppState,
    accounts::{self, Invalid},
    login::{Key, PER_ADDRESS, PER_NAME},
    router, sessions,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tower::ServiceExt;

const PASSWORD: &str = "correct horse battery";

async fn send(state: &AppState, request: Request<Body>) -> (StatusCode, HeaderMap, String) {
    let app = router(state.clone(), "missing".as_ref());
    let response = app.oneshot(request).await.unwrap();
    let (status, headers) = (response.status(), response.headers().clone());
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, headers, String::from_utf8(body.to_vec()).unwrap())
}

fn login_request(name: &str, password: &str) -> Request<Body> {
    Request::post("/api/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "username": name, "password": password }).to_string(),
        ))
        .unwrap()
}

async fn log_in(state: &AppState, name: &str, password: &str) -> (StatusCode, HeaderMap, String) {
    send(state, login_request(name, password)).await
}

/// What the browser would send back after this answer: `name=token`.
fn cookie(headers: &HeaderMap) -> String {
    let set = headers.get(header::SET_COOKIE).unwrap().to_str().unwrap();
    set.split(';').next().unwrap().to_owned()
}

async fn me(state: &AppState, cookie: Option<&str>) -> (StatusCode, String) {
    let mut request = Request::get("/api/me");
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    let (status, _, body) = send(state, request.body(Body::empty()).unwrap()).await;
    (status, body)
}

#[sqlx::test]
async fn a_password_is_kept_as_its_argon2id_hash(db: PgPool) {
    let id = accounts::create(&db, " Ada ", Some(PASSWORD), true)
        .await
        .unwrap()
        .unwrap();
    let (name, hash): (String, String) =
        sqlx::query_as("SELECT username, password_hash FROM accounts WHERE id = $1")
            .bind(id)
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(name, "Ada");
    assert!(
        hash.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"),
        "{hash}"
    );
    assert!(!hash.contains(PASSWORD));

    let ada = accounts::login(&db, "ada", PASSWORD)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (ada.id, ada.username.as_str(), ada.is_admin),
        (id, "Ada", true)
    );
    assert_eq!(
        accounts::login(&db, "ADA ", PASSWORD)
            .await
            .unwrap()
            .map(|a| a.id),
        Some(id)
    );
    for wrong in [
        "",
        "correct horse",
        "correct horse battery ",
        &"x".repeat(5000),
    ] {
        assert_eq!(
            accounts::login(&db, "Ada", wrong).await.unwrap(),
            None,
            "{wrong}"
        );
    }
    assert_eq!(accounts::login(&db, "Grace", PASSWORD).await.unwrap(), None);
    assert_eq!(
        accounts::login(&db, &"a".repeat(500), PASSWORD)
            .await
            .unwrap(),
        None
    );

    // The same password twice gives two hashes: each has its own salt.
    let other = accounts::hash(PASSWORD).await.unwrap();
    assert_ne!(other, hash);
    assert!(accounts::verify(PASSWORD, Some(other)).await.unwrap());
    // What is no hash matches nothing.
    assert!(
        !accounts::verify(PASSWORD, Some("plain".into()))
            .await
            .unwrap()
    );
    assert!(
        !accounts::verify(PASSWORD, Some(String::new()))
            .await
            .unwrap()
    );
}

#[sqlx::test]
async fn an_account_without_a_password_cannot_log_in_with_one(db: PgPool) {
    // As the import of the old rueder leaves accounts.
    accounts::create(&db, "imported", None, false)
        .await
        .unwrap()
        .unwrap();
    for password in ["", "imported", PASSWORD] {
        assert_eq!(
            accounts::login(&db, "imported", password).await.unwrap(),
            None
        );
    }
    let listed = accounts::list(&db).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert!(!listed[0].has_password);

    assert_eq!(
        accounts::set_password(&db, "Imported", PASSWORD)
            .await
            .unwrap(),
        Ok(true)
    );
    assert!(
        accounts::login(&db, "imported", PASSWORD)
            .await
            .unwrap()
            .is_some()
    );
    assert!(accounts::list(&db).await.unwrap()[0].has_password);
    assert_eq!(
        accounts::set_password(&db, "nobody", PASSWORD)
            .await
            .unwrap(),
        Ok(false)
    );
}

#[sqlx::test]
async fn names_and_passwords_have_rules(db: PgPool) {
    let create = async |name: &str, password: &str| {
        accounts::create(&db, name, Some(password), false)
            .await
            .unwrap()
            .err()
    };
    assert_eq!(create("", PASSWORD).await, Some(Invalid::NameEmpty));
    assert_eq!(create("  \t ", PASSWORD).await, Some(Invalid::NameEmpty));
    assert_eq!(
        create(&"n".repeat(65), PASSWORD).await,
        Some(Invalid::NameTooLong)
    );
    assert_eq!(
        create("a\u{0}b", PASSWORD).await,
        Some(Invalid::NameCharacters)
    );
    assert_eq!(
        create("line\nbreak", PASSWORD).await,
        Some(Invalid::NameCharacters)
    );
    assert_eq!(
        create("ada", "seven77").await,
        Some(Invalid::PasswordTooShort)
    );
    assert_eq!(
        create("ada", &"x".repeat(1025)).await,
        Some(Invalid::PasswordTooLong)
    );
    assert_eq!(create(&"ü".repeat(64), "pässwörd").await, None);
    assert_eq!(create("ada", &"x".repeat(1024)).await, None);
    // Names differ by more than their case.
    assert_eq!(create("ADA", PASSWORD).await, Some(Invalid::NameTaken));
    assert_eq!(
        accounts::set_password(&db, "ada", "short").await.unwrap(),
        Err(Invalid::PasswordTooShort)
    );
    assert_eq!(accounts::list(&db).await.unwrap().len(), 2);
}

#[sqlx::test]
async fn a_login_gives_the_browser_a_session_cookie(db: PgPool) {
    let id = accounts::create(&db, "ada", Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap();
    let state = AppState::new(db.clone());

    let (status, headers, body) = log_in(&state, "Ada", PASSWORD).await;
    assert_eq!(status, StatusCode::OK);
    let body: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        body,
        json!({
            "id": id,
            "username": "ada",
            "is_admin": false,
            "has_password": true,
            "admin_needs_second_factor": false,
            "terms_to_accept": null
        })
    );

    // Scripts cannot read the cookie, it travels over HTTPS only, belongs to
    // this host alone and does not come along with requests other sites make.
    let set = headers.get(header::SET_COOKIE).unwrap().to_str().unwrap();
    let (pair, attributes) = set.split_once("; ").unwrap();
    let (name, token) = pair.split_once('=').unwrap();
    assert_eq!(name, "__Host-rueder");
    assert_eq!(token.len(), 43);
    assert_eq!(
        attributes,
        "Path=/; Max-Age=34560000; HttpOnly; Secure; SameSite=Lax"
    );

    // The database has the token's checksum and not the token.
    let stored: Vec<Vec<u8>> = sqlx::query_scalar("SELECT token_hash FROM sessions")
        .fetch_all(&db)
        .await
        .unwrap();
    assert_eq!(stored, [Sha256::digest(token.as_bytes()).to_vec()]);

    let (status, body) = me(&state, Some(pair)).await;
    assert_eq!(status, StatusCode::OK);
    let body: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        body,
        json!({
            "id": id,
            "username": "ada",
            "is_admin": false,
            "has_password": true,
            "admin_needs_second_factor": false,
            "terms_to_accept": null
        })
    );
    // Among other cookies too.
    let among = format!("theme=dark; {pair}; other=1");
    assert_eq!(me(&state, Some(&among)).await.0, StatusCode::OK);

    // Without the cookie, or with one the server did not give out, nobody is
    // logged in.
    let forged = [
        "__Host-rueder=".to_owned(),
        format!("__Host-rueder={}", "A".repeat(43)),
        format!("__Host-rueder={}", "A".repeat(5000)),
        format!("rueder={token}"),
        format!("__Host-rueder={}", &token[..42]),
    ];
    assert_eq!(me(&state, None).await.0, StatusCode::UNAUTHORIZED);
    for cookie in &forged {
        assert_eq!(
            me(&state, Some(cookie)).await.0,
            StatusCode::UNAUTHORIZED,
            "{cookie}"
        );
    }
}

#[sqlx::test]
async fn a_wrong_name_and_a_wrong_password_get_the_same_answer(db: PgPool) {
    accounts::create(&db, "ada", Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap();
    accounts::create(&db, "imported", None, false)
        .await
        .unwrap()
        .unwrap();
    let state = AppState::new(db.clone());

    let wrong_password = log_in(&state, "ada", "not the password").await;
    let wrong_name = log_in(&state, "grace", PASSWORD).await;
    let no_password = log_in(&state, "imported", "").await;
    assert_eq!(wrong_password.0, StatusCode::UNAUTHORIZED);
    assert_eq!(wrong_password.2, "wrong name or password");
    for other in [wrong_name, no_password] {
        assert_eq!((other.0, &other.2), (wrong_password.0, &wrong_password.2));
        assert_eq!(other.1.get(header::SET_COOKIE), None);
    }
    let sessions: i64 = sqlx::query_scalar("SELECT count(*) FROM sessions")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(sessions, 0);

    // What is no login at all is refused before anything is checked.
    let request = Request::post("/api/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"username": "ada"}"#))
        .unwrap();
    assert_eq!(
        send(&state, request).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let request = Request::get("/api/login").body(Body::empty()).unwrap();
    assert_eq!(
        send(&state, request).await.0,
        StatusCode::METHOD_NOT_ALLOWED
    );
}

#[sqlx::test]
async fn logging_out_ends_this_session_and_no_other(db: PgPool) {
    accounts::create(&db, "ada", Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap();
    accounts::create(&db, "grace", Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap();
    let state = AppState::new(db.clone());
    let laptop = cookie(&log_in(&state, "ada", PASSWORD).await.1);
    let phone = cookie(&log_in(&state, "ada", PASSWORD).await.1);
    let grace = cookie(&log_in(&state, "grace", PASSWORD).await.1);
    assert_ne!(laptop, phone);

    // Each sees who they are and nobody else.
    let (_, body) = me(&state, Some(&grace)).await;
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap()["username"],
        "grace"
    );
    let (_, body) = me(&state, Some(&laptop)).await;
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap()["username"],
        "ada"
    );

    let request = Request::post("/api/logout")
        .header(header::COOKIE, &laptop)
        .body(Body::empty())
        .unwrap();
    let (status, headers, _) = send(&state, request).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        headers.get(header::SET_COOKIE).unwrap(),
        "__Host-rueder=; Path=/; Max-Age=0; HttpOnly; Secure; SameSite=Lax"
    );
    assert_eq!(me(&state, Some(&laptop)).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(me(&state, Some(&phone)).await.0, StatusCode::OK);
    assert_eq!(me(&state, Some(&grace)).await.0, StatusCode::OK);

    // Logging out without a session is no error.
    let request = Request::post("/api/logout").body(Body::empty()).unwrap();
    assert_eq!(send(&state, request).await.0, StatusCode::NO_CONTENT);

    // Logging in again from a browser that has a session replaces it.
    let request = Request::post("/api/login")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, &phone)
        .body(Body::from(
            json!({ "username": "ada", "password": PASSWORD }).to_string(),
        ))
        .unwrap();
    let (status, headers, _) = send(&state, request).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me(&state, Some(&phone)).await.0, StatusCode::UNAUTHORIZED);
    let phone = cookie(&headers);
    assert_eq!(me(&state, Some(&phone)).await.0, StatusCode::OK);

    // A new password ends what is left of ada's sessions, and only hers.
    assert_eq!(
        accounts::set_password(&db, "ada", "another password")
            .await
            .unwrap(),
        Ok(true)
    );
    assert_eq!(me(&state, Some(&phone)).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(me(&state, Some(&grace)).await.0, StatusCode::OK);
    assert_eq!(
        log_in(&state, "ada", PASSWORD).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        log_in(&state, "ada", "another password").await.0,
        StatusCode::OK
    );
}

#[sqlx::test]
async fn a_session_runs_out_when_it_is_not_used(db: PgPool) {
    let id = common::account(&db, "ada").await;
    let used = sessions::start(&db, id, false).await.unwrap();
    let unused = sessions::start(&db, id, false).await.unwrap();
    let left = async |token: &str| -> f64 {
        sqlx::query_scalar(
            "SELECT extract(epoch FROM expires_at - now())::float8 FROM sessions
             WHERE token_hash = sha256(convert_to($1, 'UTF8'))",
        )
        .bind(token)
        .fetch_one(&db)
        .await
        .unwrap()
    };
    let days = |n: f64| n * 86400.0;
    assert!((left(&used).await - days(30.0)).abs() < 60.0);

    // 29 days later: one was used an hour ago, the other never.
    sqlx::query(
        "UPDATE sessions SET created_at = created_at - interval '29 days',
             last_seen_at = last_seen_at - interval '29 days',
             expires_at = expires_at - interval '29 days'",
    )
    .execute(&db)
    .await
    .unwrap();
    assert!(sessions::find(&db, &used).await.unwrap().is_some());
    assert!((left(&used).await - days(30.0)).abs() < 60.0);
    assert!((left(&unused).await - days(1.0)).abs() < 60.0);
    // Used again right away, nothing is written.
    sqlx::query("UPDATE sessions SET expires_at = expires_at - interval '1 day'")
        .execute(&db)
        .await
        .unwrap();
    assert!(sessions::find(&db, &used).await.unwrap().is_some());
    assert!((left(&used).await - days(29.0)).abs() < 60.0);

    // The unused one is over now, although its row is still there.
    assert_eq!(sessions::find(&db, &unused).await.unwrap(), None);
    let state = AppState::new(db.clone());
    let cookie = format!("{}={unused}", sessions::COOKIE);
    assert_eq!(me(&state, Some(&cookie)).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(sessions::sweep(&db).await.unwrap(), 1);
    assert_eq!(sessions::sweep(&db).await.unwrap(), 0);
    assert!(sessions::find(&db, &used).await.unwrap().is_some());

    // A session goes with its account.
    sqlx::query("DELETE FROM accounts WHERE id = $1")
        .bind(id)
        .execute(&db)
        .await
        .unwrap();
    assert_eq!(sessions::find(&db, &used).await.unwrap(), None);
}

#[sqlx::test]
async fn wrong_passwords_are_counted_for_the_name(db: PgPool) {
    accounts::create(&db, "ada", Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap();
    accounts::create(&db, "grace", Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap();
    let state = AppState::new(db);

    // A login that works forgets the wrong tries before it.
    for _ in 0..PER_NAME - 1 {
        assert_eq!(
            log_in(&state, "ada", "wrong").await.0,
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(log_in(&state, "ada", PASSWORD).await.0, StatusCode::OK);

    for _ in 0..PER_NAME {
        assert_eq!(
            log_in(&state, " ADA", "wrong").await.0,
            StatusCode::UNAUTHORIZED
        );
    }
    // Now even the right password has to wait.
    let (status, headers, _) = log_in(&state, "ada", PASSWORD).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    let wait: u64 = headers
        .get(header::RETRY_AFTER)
        .unwrap()
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!((1..=900).contains(&wait), "{wait}");
    assert_eq!(headers.get(header::SET_COOKIE), None);

    // Names that do not exist are counted like the others, so the limit does
    // not tell them apart.
    for _ in 0..PER_NAME {
        assert_eq!(
            log_in(&state, "nobody", "wrong").await.0,
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        log_in(&state, "nobody", "wrong").await.0,
        StatusCode::TOO_MANY_REQUESTS
    );

    // Another account is not held up by this.
    assert_eq!(log_in(&state, "grace", PASSWORD).await.0, StatusCode::OK);
}

#[sqlx::test]
async fn wrong_passwords_sent_at_once_are_counted_all_the_same(db: PgPool) {
    accounts::create(&db, "ada", Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap();
    let state = AppState::new(db);
    // All of them are under way before the first password is checked.
    let tries = (0..PER_NAME * 3).map(|n| {
        let state = state.clone();
        tokio::spawn(async move { log_in(&state, "ada", &format!("wrong-{n}")).await.0 })
    });
    let mut answers = Vec::new();
    for answer in futures_util::future::join_all(tries).await {
        answers.push(answer.unwrap());
    }
    let checked = answers
        .iter()
        .filter(|status| **status == StatusCode::UNAUTHORIZED)
        .count();
    let held = answers
        .iter()
        .filter(|status| **status == StatusCode::TOO_MANY_REQUESTS)
        .count();
    assert_eq!((checked, held), (PER_NAME as usize, 2 * PER_NAME as usize));
    assert_eq!(
        log_in(&state, "ada", PASSWORD).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[sqlx::test]
async fn wrong_passwords_are_counted_for_the_address(db: PgPool) {
    accounts::create(&db, "ada", Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap();
    let from = |forwarded: &'static str| {
        let mut request = login_request("ada", PASSWORD);
        let headers = request.headers_mut();
        headers.insert("x-forwarded-for", forwarded.parse().unwrap());
        request
    };
    let noisy: IpAddr = "203.0.113.9".parse().unwrap();
    let now = std::time::Instant::now();

    // Behind a proxy that names the sender, the address it names counts.
    let state =
        AppState::new(db.clone()).with_client_ip_header(HeaderName::from_static("x-forwarded-for"));
    for _ in 0..PER_ADDRESS {
        state.logins.failed(Key::address(noisy), now);
    }
    assert_eq!(
        send(&state, from("203.0.113.9")).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    // What the sender wrote in front of the proxy's entry does not help it.
    assert_eq!(
        send(&state, from("198.51.100.1, 203.0.113.9")).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        send(&state, from("203.0.113.9, 198.51.100.1")).await.0,
        StatusCode::OK
    );
    assert_eq!(send(&state, from("no address")).await.0, StatusCode::OK);

    // Without that setting the header means nothing: all of these come from
    // the same place, here from nowhere.
    let state = AppState::new(db);
    for _ in 0..PER_ADDRESS {
        state.logins.failed(Key::address(noisy), now);
    }
    assert_eq!(send(&state, from("203.0.113.9")).await.0, StatusCode::OK);
    for _ in 0..PER_ADDRESS {
        state
            .logins
            .failed(Key::address("::".parse().unwrap()), now);
    }
    assert_eq!(
        send(&state, from("198.51.100.1")).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[sqlx::test]
async fn an_account_has_only_so_many_sessions(db: PgPool) {
    let ada = common::account(&db, "ada").await;
    let grace = common::account(&db, "grace").await;
    let state = AppState::new(db.clone());
    let theirs = common::session(&db, grace).await;
    let first = common::session(&db, ada).await;
    let second = common::session(&db, ada).await;
    // The first began earlier, but the second was used longer ago.
    let token = second.split_once('=').unwrap().1;
    sqlx::query(
        "UPDATE sessions SET last_seen_at = now() - interval '2 hours' WHERE token_hash = $1",
    )
    .bind(Sha256::digest(token.as_bytes()).to_vec())
    .execute(&db)
    .await
    .unwrap();
    let count = async || -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM sessions WHERE account_id = $1")
            .bind(ada)
            .fetch_one(&db)
            .await
            .unwrap()
    };

    let mut newest = String::new();
    for _ in 2..sessions::MAX_PER_ACCOUNT {
        newest = common::session(&db, ada).await;
    }
    assert_eq!(count().await, sessions::MAX_PER_ACCOUNT);

    // One more: the one that was used longest ago ends, and no other.
    let one_more = common::session(&db, ada).await;
    assert_eq!(count().await, sessions::MAX_PER_ACCOUNT);
    assert_eq!(me(&state, Some(&second)).await.0, StatusCode::UNAUTHORIZED);
    for cookie in [&first, &newest, &one_more, &theirs] {
        assert_eq!(me(&state, Some(cookie)).await.0, StatusCode::OK);
    }
}
