//! "Forgot password" (work package 5.5): the same answer for whoever is asked
//! for, the link that runs out and works once, the end of every session, the
//! second factor that is still asked for, and the limits.

mod common;

use std::time::Duration;

use axum::{
    body::Body,
    http::{HeaderName, Method, Request, StatusCode, header},
};
use common::call;
use http_body_util::BodyExt;
use rueder::{
    AppState, accounts,
    email::Mode,
    mail::{Language, Mailer, Outbox, Sent, Template, render},
    rate::{Rate, Rates},
    registration,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

const PASSWORD: &str = "correct horse battery";
const FRESH: &str = "a brand new password";
const SITE: &str = "https://reader.example";

async fn set_mode(db: &PgPool, mode: Mode) {
    let mut settings = registration::settings(db).await.unwrap();
    settings.email = mode;
    registration::set_settings(db, &settings).await.unwrap();
}

/// A server that keeps addresses and mails to an outbox; the sender is named
/// in `X-Forwarded-For`, so tests can be several.
async fn server(db: &PgPool, mode: Mode) -> (AppState, Outbox) {
    set_mode(db, mode).await;
    let (mailer, outbox) = Mailer::in_memory(SITE).unwrap();
    // Room for the mails of a test; the tests of the limits set their own.
    let hour = Duration::from_secs(3600);
    let room = Rate::new(100, hour);
    let mut state = AppState::new(db.clone())
        .with_mail(mailer)
        .with_rates(Rates::default().with_mails(room, room, room));
    state.client_ip_header = Some(HeaderName::from_static("x-forwarded-for"));
    (state, outbox)
}

/// An account with a password and, if given, a confirmed address.
async fn account(db: &PgPool, name: &str, address: Option<&str>) -> i32 {
    let id = accounts::create(db, name, Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap();
    sqlx::query("UPDATE accounts SET email = $2 WHERE id = $1")
        .bind(id)
        .bind(address)
        .execute(db)
        .await
        .unwrap();
    id
}

/// Sends a request from this sender and returns the answer as it came.
async fn send(
    state: &AppState,
    path: &str,
    from: &str,
    body: Value,
) -> (StatusCode, Vec<(String, String)>, String) {
    let request = Request::builder()
        .method(Method::POST)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-forwarded-for", from)
        .body(Body::from(body.to_string()))
        .unwrap();
    let app = rueder::router(state.clone(), "missing".as_ref());
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    // Everything but the date, which differs by the second.
    let headers = response
        .headers()
        .iter()
        .filter(|(name, _)| *name != header::DATE)
        .map(|(name, value)| (name.to_string(), value.to_str().unwrap_or("").to_owned()))
        .collect();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        headers,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

async fn forgot(state: &AppState, who: &str) -> (StatusCode, Vec<(String, String)>, String) {
    send(
        state,
        "/api/password/forgot",
        "192.0.2.1",
        json!({ "name": who }),
    )
    .await
}

async fn reset(state: &AppState, token: &str, fresh: &str) -> StatusCode {
    let body = json!({ "token": token, "new": fresh });
    send(state, "/api/password/reset", "192.0.2.1", body)
        .await
        .0
}

async fn log_in(state: &AppState, name: &str, password: &str) -> (StatusCode, String) {
    let body = json!({ "username": name, "password": password });
    let (status, _, text) = send(state, "/api/login", "192.0.2.1", body).await;
    (status, text)
}

/// The mails once `count` of them are there, waiting for the background, and
/// a moment longer to see that no more come.
async fn mails(outbox: &Outbox, count: usize) -> Vec<Sent> {
    for _ in 0..100 {
        if outbox.lock().unwrap().len() >= count {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    tokio::time::sleep(Duration::from_millis(150)).await;
    let mails = outbox.lock().unwrap().clone();
    assert_eq!(mails.len(), count, "{mails:#?}");
    mails
}

/// The token of the link in a mail.
fn token(mail: &Sent) -> String {
    let start = mail.body.find("?reset=").expect("a link") + "?reset=".len();
    mail.body[start..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

fn subject(template: Template, language: Language) -> String {
    render(template.text(language), &[]).0
}

#[sqlx::test]
async fn without_addresses_there_is_no_way(db: PgPool) {
    account(&db, "ada", Some("ada@example.org")).await;
    // Addresses are off, though there is a mailer.
    let (state, outbox) = server(&db, Mode::Off).await;
    let (_, ways) = call(&state, Method::GET, "/api/login/ways", None, None).await;
    assert_eq!(ways["password_reset"], json!(false));
    assert_eq!(forgot(&state, "ada").await.0, StatusCode::NOT_FOUND);
    assert_eq!(
        reset(&state, "anything", FRESH).await,
        StatusCode::NOT_FOUND
    );
    // And with addresses but no mailer.
    set_mode(&db, Mode::Optional).await;
    let without = AppState::new(db.clone());
    let (_, ways) = call(&without, Method::GET, "/api/login/ways", None, None).await;
    assert_eq!(ways["password_reset"], json!(false));
    assert_eq!(forgot(&without, "ada").await.0, StatusCode::NOT_FOUND);
    assert!(mails(&outbox, 0).await.is_empty());
}

#[sqlx::test]
async fn whoever_is_asked_for_gets_the_same_answer(db: PgPool) {
    let (state, outbox) = server(&db, Mode::Optional).await;
    let (_, ways) = call(&state, Method::GET, "/api/login/ways", None, None).await;
    assert_eq!(ways["password_reset"], json!(true));
    account(&db, "ada", Some("ada@example.org")).await;
    account(&db, "grace", None).await;
    let linus = account(&db, "linus", Some("linus@example.org")).await;
    accounts::set_disabled(&db, "linus", true).await.unwrap();
    // Logs in only through OpenID Connect: no password, no link.
    let sso = account(&db, "sso", Some("sso@example.org")).await;
    sqlx::query("UPDATE accounts SET password_hash = NULL WHERE id = $1")
        .bind(sso)
        .execute(&db)
        .await
        .unwrap();
    let waits = account(&db, "waits", None).await;
    sqlx::query("UPDATE accounts SET awaiting_email = true WHERE id = $1")
        .bind(waits)
        .execute(&db)
        .await
        .unwrap();
    assert!(linus > 0);

    let known = forgot(&state, "ada").await;
    assert_eq!(known.0, StatusCode::ACCEPTED);
    // One after the other, so that the second link is the newer one.
    let first = mails(&outbox, 1).await;
    for who in [
        " ADA@Example.org ",
        "nobody",
        "nobody@example.org",
        "grace",
        "linus",
        "linus@example.org",
        "sso",
        "waits",
    ] {
        assert_eq!(forgot(&state, who).await, known, "{who}");
    }
    // Two mails went, both to Ada, for her name and her address.
    let sent = mails(&outbox, 2).await;
    assert!(
        sent.iter().all(|mail| mail.to == "ada@example.org"),
        "{sent:#?}"
    );
    assert_eq!(sent[0], first[0]);
    assert_eq!(sent[0].subject, subject(Template::Reset, Language::En));
    assert!(sent[0].body.contains("https://reader.example/?reset="));
    assert!(sent[0].body.contains("30 minutes"));
    // Only the newest counts.
    assert_eq!(
        reset(&state, &token(&sent[0]), FRESH).await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        reset(&state, &token(&sent[1]), FRESH).await,
        StatusCode::NO_CONTENT
    );

    // Nothing to give is no question.
    assert_eq!(forgot(&state, "   ").await.0, StatusCode::BAD_REQUEST);
    let long = "a".repeat(300);
    assert_eq!(forgot(&state, &long).await.0, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn a_new_password_ends_every_session_and_the_second_factor_stays(db: PgPool) {
    let (state, outbox) = server(&db, Mode::Required).await;
    let ada = account(&db, "ada", Some("ada@example.org")).await;
    let grace = account(&db, "grace", Some("grace@example.org")).await;
    let ada_cookie = common::session(&db, ada).await;
    let grace_cookie = common::session(&db, grace).await;
    sqlx::query(
        "INSERT INTO app_passwords (account_id, name, secret_hash)
         VALUES ($1, 'Reeder', sha256('secret'::bytea))",
    )
    .bind(ada)
    .execute(&db)
    .await
    .unwrap();

    forgot(&state, "ada").await;
    let link = token(&mails(&outbox, 1).await[0]);
    // A password that cannot be one is refused, and the link stays good.
    assert_eq!(reset(&state, &link, "short").await, StatusCode::BAD_REQUEST);
    assert_eq!(reset(&state, &link, FRESH).await, StatusCode::NO_CONTENT);
    // Once only.
    assert_eq!(
        reset(&state, &link, "yet another password").await,
        StatusCode::NOT_FOUND
    );

    // Every session and app password of Ada's is gone; Grace's stays.
    let (status, _) = call(&state, Method::GET, "/api/me", Some(&ada_cookie), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = call(&state, Method::GET, "/api/me", Some(&grace_cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    let apps: i64 = sqlx::query_scalar("SELECT count(*) FROM app_passwords WHERE account_id = $1")
        .bind(ada)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(apps, 0);

    // The address hears of it.
    let sent = mails(&outbox, 2).await;
    assert_eq!(sent[1].to, "ada@example.org");
    assert_eq!(
        sent[1].subject,
        subject(Template::PasswordChanged, Language::En)
    );

    // The old password opens nothing, the new one does; the reset logged
    // nobody in.
    assert_eq!(
        log_in(&state, "ada", PASSWORD).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(log_in(&state, "ada", FRESH).await.0, StatusCode::OK);

    // With a second factor the login still asks for it.
    sqlx::query("UPDATE accounts SET totp_secret = '\\x0102030405' WHERE id = $1")
        .bind(grace)
        .execute(&db)
        .await
        .unwrap();
    forgot(&state, "grace@example.org").await;
    let link = token(&mails(&outbox, 3).await[2]);
    assert_eq!(reset(&state, &link, FRESH).await, StatusCode::NO_CONTENT);
    let (status, said) = log_in(&state, "grace", FRESH).await;
    assert_eq!(
        (status, said.as_str()),
        (StatusCode::UNAUTHORIZED, "second factor needed")
    );
}

#[sqlx::test]
async fn a_link_runs_out_and_goes_with_the_address(db: PgPool) {
    let (state, outbox) = server(&db, Mode::Optional).await;
    let ada = account(&db, "ada", Some("ada@example.org")).await;
    forgot(&state, "ada").await;
    let link = token(&mails(&outbox, 1).await[0]);
    sqlx::query("UPDATE password_resets SET expires_at = now() - interval '1 second'")
        .execute(&db)
        .await
        .unwrap();
    assert_eq!(reset(&state, &link, FRESH).await, StatusCode::NOT_FOUND);

    // A link to an address the account no longer has is no good.
    forgot(&state, "ada").await;
    let link = token(&mails(&outbox, 2).await[1]);
    sqlx::query("UPDATE accounts SET email = 'ada@new.example' WHERE id = $1")
        .bind(ada)
        .execute(&db)
        .await
        .unwrap();
    assert_eq!(reset(&state, &link, FRESH).await, StatusCode::NOT_FOUND);

    // Nor one from before a new password that came another way.
    sqlx::query("UPDATE accounts SET email = 'ada@example.org' WHERE id = $1")
        .bind(ada)
        .execute(&db)
        .await
        .unwrap();
    forgot(&state, "ada").await;
    let link = token(&mails(&outbox, 3).await[2]);
    accounts::set_password(&db, "ada", "set by the operator")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(reset(&state, &link, FRESH).await, StatusCode::NOT_FOUND);

    // Unknown, empty and overlong tokens are the same no.
    for made_up in ["", "nothing-like-it", &"a".repeat(5000)] {
        assert_eq!(reset(&state, made_up, FRESH).await, StatusCode::NOT_FOUND);
    }
    // The hourly run takes what ran out.
    sqlx::query("UPDATE password_resets SET expires_at = now() - interval '1 second'")
        .execute(&db)
        .await
        .unwrap();
    forgot(&state, "ada").await;
    mails(&outbox, 4).await;
    rueder::recovery::sweep(&db).await.unwrap();
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM password_resets")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(left, 1, "the newest is still good");
}

#[sqlx::test]
async fn asking_has_the_limits_of_the_login_and_does_not_lock_it(db: PgPool) {
    let (state, outbox) = server(&db, Mode::Optional).await;
    let state = state.with_rates(Rates::default());
    account(&db, "ada", Some("ada@example.org")).await;
    // Per name, known or not: ten in a quarter of an hour.
    for who in ["ada", "nobody"] {
        for n in 0..10 {
            assert_eq!(
                forgot(&state, who).await.0,
                StatusCode::ACCEPTED,
                "{who} {n}"
            );
        }
        let (status, headers, _) = forgot(&state, who).await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{who}");
        assert!(headers.iter().any(|(name, _)| name == "retry-after"));
    }
    // Ada still logs in: the count is a different one.
    assert_eq!(log_in(&state, "ada", PASSWORD).await.0, StatusCode::OK);

    // Per sender: thirty, whichever names.
    let from = "198.51.100.7";
    for n in 0..30 {
        let body = json!({ "name": format!("name-{n}") });
        let (status, _, _) = send(&state, "/api/password/forgot", from, body).await;
        assert_eq!(status, StatusCode::ACCEPTED, "{n}");
    }
    let body = json!({ "name": "one-more" });
    let (status, _, _) = send(&state, "/api/password/forgot", from, body).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);

    // The limits on mail count silently: Ada got three of her ten, as many
    // as may go to one address at once.
    assert_eq!(mails(&outbox, 3).await.len(), 3);
}

#[sqlx::test]
async fn the_limits_on_mail_say_nothing(db: PgPool) {
    let hour = Duration::from_secs(3600);
    let (state, outbox) = server(&db, Mode::Optional).await;
    let state = state.with_rates(Rates::default().with_mails(
        Rate::new(1, hour),
        Rate::new(9, hour),
        Rate::new(9, hour),
    ));
    account(&db, "ada", Some("ada@example.org")).await;
    let first = forgot(&state, "ada").await;
    assert_eq!(first, forgot(&state, "ada").await);
    assert_eq!(first, forgot(&state, "nobody").await);
    mails(&outbox, 1).await;
}

#[sqlx::test]
async fn mail_goes_in_the_language_of_the_account(db: PgPool) {
    let (state, outbox) = server(&db, Mode::Optional).await;
    let nina = account(&db, "nina", Some("nina@example.org")).await;
    sqlx::query("UPDATE accounts SET language = 'de' WHERE id = $1")
        .bind(nina)
        .execute(&db)
        .await
        .unwrap();
    forgot(&state, "nina").await;
    let sent = mails(&outbox, 1).await;
    assert_eq!(sent[0].subject, subject(Template::Reset, Language::De));
    assert!(sent[0].body.contains("30 Minuten"), "{}", sent[0].body);
    assert_eq!(
        reset(&state, &token(&sent[0]), FRESH).await,
        StatusCode::NO_CONTENT
    );
    let sent = mails(&outbox, 2).await;
    assert_eq!(
        sent[1].subject,
        subject(Template::PasswordChanged, Language::De)
    );
}

#[sqlx::test]
async fn of_links_asked_for_at_once_one_is_left(db: PgPool) {
    let (state, outbox) = server(&db, Mode::Optional).await;
    account(&db, "ada", Some("ada@example.org")).await;
    let asked: Vec<_> = (0..8)
        .map(|_| {
            let state = state.clone();
            tokio::spawn(async move { forgot(&state, "ada").await.0 })
        })
        .collect();
    for asking in asked {
        assert_eq!(asking.await.unwrap(), StatusCode::ACCEPTED);
    }
    mails(&outbox, 8).await;
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM password_resets")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(left, 1);
}
