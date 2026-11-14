//! The address of an account (work package 5.4): the three settings of the
//! operator, links that are old, someone else's or used, a new address and the
//! notice to the one before, the limits on mail, and the mail itself against
//! a relay made up for the test.

mod common;

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::http::{HeaderMap, Method, StatusCode, header};
use common::{call, raw};
use rueder::{
    AppState,
    email::{self, Mode},
    mail::{Mailer, Outbox, Sent},
    rate::{Rate, Rates},
    registration,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
};

const PASSWORD: &str = "correct horse battery";
const SITE: &str = "https://reader.example";

async fn set_mode(db: &PgPool, mode: Mode) {
    let mut settings = registration::settings(db).await.unwrap();
    settings.open = true;
    settings.email = mode;
    registration::set_settings(db, &settings).await.unwrap();
}

/// A server that sends mail to an outbox, with registration open and
/// addresses as given.
async fn server(db: &PgPool, mode: Mode) -> (AppState, Outbox) {
    set_mode(db, mode).await;
    let (mailer, outbox) = Mailer::in_memory(SITE).unwrap();
    (AppState::new(db.clone()).with_mail(mailer), outbox)
}

/// An account with a password, logged in.
async fn reader(db: &PgPool, name: &str) -> (i32, String) {
    let id = rueder::accounts::create(db, name, Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap();
    (id, common::session(db, id).await)
}

async fn register(
    state: &AppState,
    name: &str,
    address: Option<&str>,
) -> (StatusCode, HeaderMap, Value) {
    let mut body = json!({ "username": name, "password": PASSWORD, "language": "en" });
    if let Some(address) = address {
        body["email"] = json!(address);
    }
    let body = ("application/json", body.to_string().into_bytes());
    let (status, headers, answer) =
        raw(state, Method::POST, "/api/register", None, Some(body)).await;
    (
        status,
        headers,
        serde_json::from_slice(&answer).unwrap_or(Value::Null),
    )
}

async fn log_in(state: &AppState, name: &str, password: &str) -> (StatusCode, Value) {
    let body = json!({ "username": name, "password": password });
    let body = ("application/json", body.to_string().into_bytes());
    let (status, _, answer) = raw(state, Method::POST, "/api/login", None, Some(body)).await;
    let answer = serde_json::from_slice(&answer)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&answer).into_owned()));
    (status, answer)
}

fn cookie(headers: &HeaderMap) -> String {
    let set = headers.get(header::SET_COOKIE).unwrap().to_str().unwrap();
    set.split(';').next().unwrap().to_owned()
}

fn sent(outbox: &Outbox) -> Vec<Sent> {
    outbox.lock().unwrap().clone()
}

/// The token in the newest mail to this address.
fn token_for(outbox: &Outbox, address: &str) -> String {
    let mails = sent(outbox);
    let mail = mails
        .iter()
        .rev()
        .find(|mail| mail.to == address)
        .unwrap_or_else(|| panic!("no mail to {address}: {mails:?}"));
    let start = mail.body.find("?confirm=").expect("a link") + "?confirm=".len();
    mail.body[start..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

async fn confirm(state: &AppState, token: &str) -> (StatusCode, Value) {
    let body = json!({ "token": token }).to_string().into_bytes();
    let body = ("application/json", body);
    let (status, _, answer) =
        raw(state, Method::POST, "/api/email/confirm", None, Some(body)).await;
    let answer = serde_json::from_slice(&answer)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&answer).into_owned()));
    (status, answer)
}

async fn change(state: &AppState, cookie: &str, address: &str) -> (StatusCode, Value) {
    let body = json!({ "email": address, "password": PASSWORD, "language": "de" });
    call(state, Method::PUT, "/api/email", Some(cookie), Some(body)).await
}

async fn me(state: &AppState, cookie: &str) -> Value {
    let (status, me) = call(state, Method::GET, "/api/me", Some(cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    me
}

async fn address_of(db: &PgPool, id: i32) -> Option<String> {
    sqlx::query_scalar("SELECT email FROM accounts WHERE id = $1")
        .bind(id)
        .fetch_one(db)
        .await
        .unwrap()
}

async fn links(db: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM email_links")
        .fetch_one(db)
        .await
        .unwrap()
}

#[sqlx::test]
async fn as_rueder_comes_there_are_no_addresses(db: PgPool) {
    let (state, outbox) = server(&db, Mode::Off).await;
    let (_, asked) = call(&state, Method::GET, "/api/register", None, None).await;
    assert_eq!(asked["email"], json!("off"));

    // An address sent along anyway is not looked at.
    let (status, headers, made) = register(&state, "ada", Some("ada@example.org")).await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    assert_eq!(made["email_mode"], json!("off"));
    assert_eq!(
        (&made["email"], &made["email_pending"]),
        (&Value::Null, &Value::Null)
    );
    assert!(sent(&outbox).is_empty());

    let cookie = cookie(&headers);
    let (status, _) = change(&state, &cookie, "ada@example.org").await;
    assert_eq!(status, StatusCode::CONFLICT);
    let body = json!({ "password": PASSWORD });
    let (status, _) = call(
        &state,
        Method::DELETE,
        "/api/email",
        Some(&cookie),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(sent(&outbox).is_empty());
    assert_eq!(links(&db).await, 0);
}

#[sqlx::test]
async fn where_it_is_optional_the_account_logs_in_at_once(db: PgPool) {
    let (state, outbox) = server(&db, Mode::Optional).await;
    let (_, asked) = call(&state, Method::GET, "/api/register", None, None).await;
    assert_eq!(asked["email"], json!("optional"));

    // Without an address, as before.
    let (status, _, made) = register(&state, "grace", None).await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    assert_eq!(made["email"], Value::Null);
    assert!(sent(&outbox).is_empty());

    // With one: logged in, and the address waits for its link.
    let (status, headers, made) = register(&state, "ada", Some(" Ada@Example.org ")).await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    assert_eq!(made["email_pending"], json!("Ada@Example.org"));
    assert_eq!(made["email"], Value::Null);
    let mails = sent(&outbox);
    assert_eq!(mails.len(), 1);
    assert_eq!(mails[0].to, "Ada@Example.org");
    assert!(mails[0].body.contains("ada"), "{}", mails[0].body);
    assert!(mails[0].body.contains("https://reader.example/?confirm="));
    assert!(mails[0].body.contains("24"));

    let cookie = cookie(&headers);
    let token = token_for(&outbox, "Ada@Example.org");
    let (status, confirmed) = confirm(&state, &token).await;
    assert_eq!(status, StatusCode::OK, "{confirmed}");
    assert_eq!(
        confirmed,
        json!({ "username": "ada", "email": "Ada@Example.org", "activated": false })
    );
    let me = me(&state, &cookie).await;
    assert_eq!(
        (&me["email"], &me["email_pending"]),
        (&json!("Ada@Example.org"), &Value::Null)
    );

    // Once only.
    assert_eq!(confirm(&state, &token).await.0, StatusCode::NOT_FOUND);

    // A wrong address is refused, and no account made for it.
    let (status, _, _) = register(&state, "linus", Some("not an address")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        rueder::accounts::id_of(&db, "linus")
            .await
            .unwrap()
            .is_none()
    );

    // Optional: it can go again, with the password.
    let wrong = json!({ "password": "wrong password" });
    let (status, _) = call(
        &state,
        Method::DELETE,
        "/api/email",
        Some(&cookie),
        Some(wrong),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let body = json!({ "password": PASSWORD });
    let (status, _) = call(
        &state,
        Method::DELETE,
        "/api/email",
        Some(&cookie),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let id = rueder::accounts::id_of(&db, "ada").await.unwrap().unwrap();
    assert_eq!(address_of(&db, id).await, None);
}

#[sqlx::test]
async fn where_it_is_required_the_account_waits_for_its_link(db: PgPool) {
    let (state, outbox) = server(&db, Mode::Required).await;
    let (_, asked) = call(&state, Method::GET, "/api/register", None, None).await;
    assert_eq!(asked["email"], json!("required"));

    let (status, _, _) = register(&state, "ada", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(rueder::accounts::id_of(&db, "ada").await.unwrap().is_none());

    let (status, headers, made) = register(&state, "ada", Some("ada@example.org")).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{made}");
    assert_eq!(made, json!({ "awaiting_email": "ada@example.org" }));
    assert!(headers.get(header::SET_COOKIE).is_none(), "no session yet");

    // It does not log in: said only after the right password.
    assert_eq!(
        log_in(&state, "ada", "wrong password").await.0,
        StatusCode::UNAUTHORIZED
    );
    let (status, said) = log_in(&state, "ada", PASSWORD).await;
    assert_eq!(
        (status, said),
        (StatusCode::FORBIDDEN, json!("unconfirmed"))
    );

    // It asks for the link again; the first one is no good any more.
    let first = token_for(&outbox, "ada@example.org");
    let body = json!({ "username": "ada", "password": "wrong password" });
    let (status, _) = call(&state, Method::POST, "/api/email/resend", None, Some(body)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let body = json!({ "username": "ada", "password": PASSWORD });
    let (status, again) = call(&state, Method::POST, "/api/email/resend", None, Some(body)).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{again}");
    assert_eq!(sent(&outbox).len(), 2);
    let second = token_for(&outbox, "ada@example.org");
    assert_ne!(first, second);
    assert_eq!(confirm(&state, &first).await.0, StatusCode::NOT_FOUND);

    let (status, confirmed) = confirm(&state, &second).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(confirmed["activated"], json!(true));
    let (status, me) = log_in(&state, "ada", PASSWORD).await;
    assert_eq!(status, StatusCode::OK, "{me}");
    assert_eq!(me["email"], json!("ada@example.org"));

    // An account that waits for nothing does not get the link again.
    let body = json!({ "username": "ada", "password": PASSWORD });
    let (status, _) = call(&state, Method::POST, "/api/email/resend", None, Some(body)).await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Required: it cannot go.
    let id = rueder::accounts::id_of(&db, "ada").await.unwrap().unwrap();
    let cookie = common::session(&db, id).await;
    let body = json!({ "password": PASSWORD });
    let (status, _) = call(
        &state,
        Method::DELETE,
        "/api/email",
        Some(&cookie),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        address_of(&db, id).await.as_deref(),
        Some("ada@example.org")
    );
}

#[sqlx::test]
async fn accounts_from_before_are_asked_and_waiting_ones_let_in_later(db: PgPool) {
    let (before, cookie) = reader(&db, "before").await;
    let (state, outbox) = server(&db, Mode::Required).await;
    // An account from before logs in as it did, and the page asks it.
    let me = me(&state, &cookie).await;
    assert_eq!(
        (&me["email_mode"], &me["email"]),
        (&json!("required"), &Value::Null)
    );
    let (status, pending) = change(&state, &cookie, "before@example.org").await;
    assert_eq!(status, StatusCode::ACCEPTED, "{pending}");
    assert_eq!(pending, json!({ "pending": "before@example.org" }));
    // Its first address says the language of its mail: the page's.
    assert!(
        sent(&outbox)[0].subject.contains("Adresse"),
        "{:?}",
        sent(&outbox)
    );
    let language: String = sqlx::query_scalar("SELECT language FROM accounts WHERE id = $1")
        .bind(before)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(language, "de");

    // One that waits logs in once the operator no longer asks for addresses.
    let (status, _, _) = register(&state, "late", Some("late@example.org")).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(
        log_in(&state, "late", PASSWORD).await.0,
        StatusCode::FORBIDDEN
    );
    set_mode(&db, Mode::Optional).await;
    assert_eq!(log_in(&state, "late", PASSWORD).await.0, StatusCode::OK);
    // Its link still confirms the address.
    let (status, confirmed) = confirm(&state, &token_for(&outbox, "late@example.org")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(confirmed["activated"], json!(false));
}

#[sqlx::test]
async fn mail_is_in_the_language_the_account_was_made_in(db: PgPool) {
    let (state, outbox) = server(&db, Mode::Optional).await;
    let body = json!({
        "username": "nina", "password": PASSWORD, "email": "nina@example.org", "language": "de-AT",
    });
    let (status, _) = call(&state, Method::POST, "/api/register", None, Some(body)).await;
    assert_eq!(status, StatusCode::CREATED);
    let english = json!({
        "username": "ada", "password": PASSWORD, "email": "ada@example.org", "language": "fr",
    });
    let (status, _) = call(&state, Method::POST, "/api/register", None, Some(english)).await;
    assert_eq!(status, StatusCode::CREATED);
    let mails = sent(&outbox);
    let german = rueder::mail::render(
        rueder::mail::Template::Confirm.text(rueder::mail::Language::De),
        &[],
    );
    let english = rueder::mail::render(
        rueder::mail::Template::Confirm.text(rueder::mail::Language::En),
        &[],
    );
    assert_eq!(mails[0].subject, german.0);
    assert_eq!(mails[1].subject, english.0);
    assert_ne!(german.0, english.0);
}

#[sqlx::test]
async fn old_unknown_and_someone_elses_links(db: PgPool) {
    let (state, outbox) = server(&db, Mode::Optional).await;
    let (ada, ada_cookie) = reader(&db, "ada").await;
    let (grace, grace_cookie) = reader(&db, "grace").await;
    assert_eq!(
        change(&state, &ada_cookie, "ada@example.org").await.0,
        StatusCode::ACCEPTED
    );

    // Nobody else sees the address that waits.
    let (_, theirs) = call(&state, Method::GET, "/api/email", Some(&grace_cookie), None).await;
    assert_eq!(
        theirs,
        json!({ "mode": "optional", "email": null, "pending": null })
    );
    let (_, mine) = call(&state, Method::GET, "/api/email", Some(&ada_cookie), None).await;
    assert_eq!(mine["pending"], json!("ada@example.org"));
    let (status, _) = call(&state, Method::GET, "/api/email", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Unknown, empty and overlong tokens are all the same no.
    for token in ["", "   ", "nothing-like-it", &"a".repeat(5000)] {
        assert_eq!(
            confirm(&state, token).await.0,
            StatusCode::NOT_FOUND,
            "{token:.20}"
        );
    }

    // A link that is a day old is no good, nor is it after a newer one.
    let token = token_for(&outbox, "ada@example.org");
    sqlx::query("UPDATE email_links SET expires_at = now() - interval '1 second'")
        .execute(&db)
        .await
        .unwrap();
    assert_eq!(confirm(&state, &token).await.0, StatusCode::NOT_FOUND);
    let (_, mine) = call(&state, Method::GET, "/api/email", Some(&ada_cookie), None).await;
    assert_eq!(
        mine["pending"],
        Value::Null,
        "an old link waits for nothing"
    );

    // Someone else's link, opened in another account's browser, is for the
    // account it was made for and does nothing to the other.
    assert_eq!(
        change(&state, &ada_cookie, "ada@example.org").await.0,
        StatusCode::ACCEPTED
    );
    let token = token_for(&outbox, "ada@example.org");
    let body = json!({ "token": token });
    let (status, confirmed) = call(
        &state,
        Method::POST,
        "/api/email/confirm",
        Some(&grace_cookie),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(confirmed["username"], json!("ada"));
    assert_eq!(
        address_of(&db, ada).await.as_deref(),
        Some("ada@example.org")
    );
    assert_eq!(address_of(&db, grace).await, None);
    assert_eq!(links(&db).await, 0);
}

#[sqlx::test]
async fn a_new_address_counts_once_confirmed_and_the_old_one_hears_of_it(db: PgPool) {
    let (state, outbox) = server(&db, Mode::Optional).await;
    // Made by the page in English; the address is asked for in German.
    let (status, headers, _) = register(&state, "ada", Some("ada@old.example")).await;
    assert_eq!(status, StatusCode::CREATED);
    let cookie = cookie(&headers);
    let ada = rueder::accounts::id_of(&db, "ada").await.unwrap().unwrap();
    confirm(&state, &token_for(&outbox, "ada@old.example")).await;
    outbox.lock().unwrap().clear();

    // Not without the password, not an address that is none, not the same.
    let wrong = json!({ "email": "ada@new.example", "password": "wrong password" });
    let (status, _) = call(
        &state,
        Method::PUT,
        "/api/email",
        Some(&cookie),
        Some(wrong),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = change(&state, &cookie, "ada at new").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = change(&state, &cookie, "ADA@old.example").await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(sent(&outbox).is_empty());

    // Until the link is opened the old one stays.
    assert_eq!(
        change(&state, &cookie, "ada@new.example").await.0,
        StatusCode::ACCEPTED
    );
    let me = me(&state, &cookie).await;
    assert_eq!(
        (&me["email"], &me["email_pending"]),
        (&json!("ada@old.example"), &json!("ada@new.example"))
    );
    let (status, _) = confirm(&state, &token_for(&outbox, "ada@new.example")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        address_of(&db, ada).await.as_deref(),
        Some("ada@new.example")
    );

    // The old one is told, without the whole new address. The account's
    // mail stays in the language it was made in, not the page's.
    let mails = sent(&outbox);
    assert_eq!(mails.len(), 2, "{mails:?}");
    let notice = &mails[1];
    assert_eq!(notice.to, "ada@old.example");
    assert!(notice.body.contains("a***@new.example"), "{}", notice.body);
    assert!(!notice.body.contains("ada@new.example"), "{}", notice.body);
    let english = rueder::mail::render(
        rueder::mail::Template::Changed.text(rueder::mail::Language::En),
        &[],
    );
    assert_eq!(notice.subject, english.0);
}

#[sqlx::test]
async fn an_address_belongs_to_one_account(db: PgPool) {
    let (state, outbox) = server(&db, Mode::Optional).await;
    let (_, ada) = reader(&db, "ada").await;
    let (grace_id, grace) = reader(&db, "grace").await;
    assert_eq!(
        change(&state, &ada, "shared@example.org").await.0,
        StatusCode::ACCEPTED
    );
    let ada_token = token_for(&outbox, "shared@example.org");
    // Asking says nothing of who has it: the link goes all the same.
    assert_eq!(
        change(&state, &grace, "Shared@Example.org").await.0,
        StatusCode::ACCEPTED
    );
    let grace_token = token_for(&outbox, "Shared@Example.org");
    assert_eq!(confirm(&state, &ada_token).await.0, StatusCode::OK);
    let (status, said) = confirm(&state, &grace_token).await;
    assert_eq!((status, said), (StatusCode::CONFLICT, json!("taken")));
    assert_eq!(address_of(&db, grace_id).await, None);
}

#[sqlx::test]
async fn mail_has_limits_per_account_address_and_day(db: PgPool) {
    let hour = Duration::from_secs(3600);
    let rates =
        Rates::default().with_mails(Rate::new(2, hour), Rate::new(3, hour), Rate::new(5, hour));
    let (state, outbox) = server(&db, Mode::Optional).await;
    let state = state.with_rates(rates);
    let (_, ada) = reader(&db, "ada").await;
    let (_, grace) = reader(&db, "grace").await;
    let (_, linus) = reader(&db, "linus").await;

    // Per account.
    assert_eq!(
        change(&state, &ada, "a1@example.org").await.0,
        StatusCode::ACCEPTED
    );
    assert_eq!(
        change(&state, &ada, "a2@example.org").await.0,
        StatusCode::ACCEPTED
    );
    assert_eq!(
        change(&state, &ada, "a3@example.org").await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    // Per address, whichever account asks.
    assert_eq!(
        change(&state, &grace, "x@example.org").await.0,
        StatusCode::ACCEPTED
    );
    assert_eq!(
        change(&state, &linus, "X@example.org").await.0,
        StatusCode::ACCEPTED
    );
    assert_eq!(
        change(&state, &grace, "x@example.org").await.0,
        StatusCode::ACCEPTED
    );
    assert_eq!(
        change(&state, &linus, "x@example.org").await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    // For the whole server: five went, the sixth does not.
    assert_eq!(sent(&outbox).len(), 5);
    let (_, nina) = reader(&db, "nina").await;
    let (status, _) = change(&state, &nina, "nina@example.org").await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(sent(&outbox).len(), 5);
}

#[sqlx::test]
async fn without_mail_there_is_no_link_and_no_account_that_waits(db: PgPool) {
    set_mode(&db, Mode::Required).await;
    let state = AppState::new(db.clone());
    let (status, _, _) = register(&state, "ada", Some("ada@example.org")).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(rueder::accounts::id_of(&db, "ada").await.unwrap().is_none());
    assert_eq!(links(&db).await, 0);
}

#[sqlx::test]
async fn the_hourly_sweep_takes_old_links_and_accounts_that_never_confirmed(db: PgPool) {
    let (state, _) = server(&db, Mode::Required).await;
    let (_, cookie) = reader(&db, "before").await;
    assert_eq!(
        change(&state, &cookie, "before@example.org").await.0,
        StatusCode::ACCEPTED
    );
    for name in ["waits", "gave-up"] {
        let (status, _, _) = register(&state, name, Some(&format!("{name}@example.org"))).await;
        assert_eq!(status, StatusCode::ACCEPTED);
    }
    sqlx::query("UPDATE email_links SET expires_at = now() - interval '1 hour'")
        .execute(&db)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE accounts SET created_at = now() - interval '8 days' WHERE username = 'gave-up'",
    )
    .execute(&db)
    .await
    .unwrap();
    email::sweep(&db).await.unwrap();
    // The old link of an account that is in goes; one that waits keeps its
    // link to send again; one that waited a week goes, and its name is free.
    let left: Vec<String> = sqlx::query_scalar(
        "SELECT a.username FROM email_links l JOIN accounts a ON a.id = l.account_id",
    )
    .fetch_all(&db)
    .await
    .unwrap();
    assert_eq!(left, ["waits"]);
    assert!(
        rueder::accounts::id_of(&db, "gave-up")
            .await
            .unwrap()
            .is_none()
    );
    let body = json!({ "username": "waits", "password": PASSWORD });
    let (status, _) = call(&state, Method::POST, "/api/email/resend", None, Some(body)).await;
    assert_eq!(status, StatusCode::ACCEPTED);
}

#[sqlx::test]
async fn neither_the_status_page_nor_a_move_takes_the_address(db: PgPool) {
    let (state, outbox) = server(&db, Mode::Optional).await;
    let (id, cookie) = reader(&db, "root").await;
    sqlx::query("UPDATE accounts SET is_admin = true WHERE id = $1")
        .bind(id)
        .execute(&db)
        .await
        .unwrap();
    assert_eq!(
        change(&state, &cookie, "root@example.org").await.0,
        StatusCode::ACCEPTED
    );
    confirm(&state, &token_for(&outbox, "root@example.org")).await;
    assert_eq!(
        address_of(&db, id).await.as_deref(),
        Some("root@example.org")
    );

    for path in ["/api/status", "/api/export"] {
        let (status, _, body) = raw(&state, Method::GET, path, Some(&cookie), None).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        let body = String::from_utf8_lossy(&body);
        assert!(!body.contains("example.org"), "{path}: {body}");
    }

    // An account that goes takes its links along.
    assert_eq!(
        change(&state, &cookie, "other@example.org").await.0,
        StatusCode::ACCEPTED
    );
    assert_eq!(links(&db).await, 1);
    rueder::accounts::remove(&db, id).await.unwrap();
    assert_eq!(links(&db).await, 0);
}

/// A relay as far as the test needs one: it takes every mail, or refuses
/// every recipient, and keeps what it was given.
async fn relay(refuse: bool) -> (u16, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let got = Arc::new(Mutex::new(Vec::new()));
    let kept = got.clone();
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            let kept = kept.clone();
            tokio::spawn(async move {
                let (read, mut write) = stream.into_split();
                let mut lines = BufReader::new(read).lines();
                write.write_all(b"220 relay.test ESMTP\r\n").await.unwrap();
                let mut data: Option<String> = None;
                while let Ok(Some(line)) = lines.next_line().await {
                    if let Some(text) = data.as_mut() {
                        if line == "." {
                            kept.lock().unwrap().push(data.take().unwrap());
                            write.write_all(b"250 2.0.0 taken\r\n").await.unwrap();
                        } else {
                            text.push_str(&line);
                            text.push('\n');
                        }
                        continue;
                    }
                    let word = line.split_whitespace().next().unwrap_or("").to_uppercase();
                    let answer: &[u8] = match word.as_str() {
                        "EHLO" | "HELO" => b"250-relay.test\r\n250 8BITMIME\r\n",
                        "MAIL" => b"250 2.1.0 ok\r\n",
                        "RCPT" if refuse => b"550 5.1.1 <nobody@example.org>: no such user\r\n",
                        "RCPT" => b"250 2.1.5 ok\r\n",
                        "DATA" => {
                            data = Some(String::new());
                            b"354 go ahead\r\n"
                        }
                        "RSET" | "NOOP" => b"250 ok\r\n",
                        "QUIT" => {
                            write.write_all(b"221 bye\r\n").await.unwrap();
                            return;
                        }
                        _ => b"502 5.5.2 no\r\n",
                    };
                    write.write_all(answer).await.unwrap();
                }
            });
        }
    });
    (port, got)
}

/// A mail's text as it was before it was put quoted-printable (RFC 2045).
fn quoted_printable(text: &str) -> String {
    let joined = text.replace("=\n", "");
    let mut bytes = Vec::new();
    let mut rest = joined.as_bytes();
    while let Some((&byte, after)) = rest.split_first() {
        let hex = after.get(..2).and_then(|two| std::str::from_utf8(two).ok());
        match hex.and_then(|hex| u8::from_str_radix(hex, 16).ok()) {
            Some(decoded) if byte == b'=' => {
                bytes.push(decoded);
                rest = &after[2..];
            }
            _ => {
                bytes.push(byte);
                rest = after;
            }
        }
    }
    String::from_utf8(bytes).unwrap()
}

#[sqlx::test]
async fn the_mail_goes_to_the_relay(db: PgPool) {
    let (port, got) = relay(false).await;
    let mailer = Mailer::smtp(
        &format!("smtp://127.0.0.1:{port}"),
        "rueder <rueder@reader.example>",
        SITE,
    )
    .unwrap();
    set_mode(&db, Mode::Optional).await;
    let state = AppState::new(db.clone()).with_mail(mailer);
    let body = json!({
        "username": "nina", "password": PASSWORD, "email": "nina@example.org", "language": "de",
    });
    let (status, made) = call(&state, Method::POST, "/api/register", None, Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    let mails = got.lock().unwrap().clone();
    assert_eq!(mails.len(), 1);
    let mail = &mails[0];
    assert!(
        mail.contains("From: rueder <rueder@reader.example>"),
        "{mail}"
    );
    assert!(mail.contains("To: nina@example.org"), "{mail}");
    assert!(
        mail.contains("Content-Type: text/plain; charset=utf-8"),
        "{mail}"
    );
    assert!(mail.contains("Subject: "), "{mail}");
    // The German text goes quoted-printable.
    assert!(
        mail.contains("Content-Transfer-Encoding: quoted-printable"),
        "{mail}"
    );
    let unfolded = quoted_printable(mail);
    assert!(unfolded.contains("Hallo nina"), "{unfolded}");
    assert!(
        unfolded.contains("https://reader.example/?confirm="),
        "{unfolded}"
    );

    // The link from the relay's mail confirms the address.
    let start = unfolded.find("?confirm=").unwrap() + "?confirm=".len();
    let token: String = unfolded[start..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    let (status, confirmed) = confirm(&state, &token).await;
    assert_eq!(status, StatusCode::OK, "{confirmed}");
}

#[sqlx::test]
async fn a_relay_that_refuses_leaves_no_link(db: PgPool) {
    let (port, got) = relay(true).await;
    let mailer = Mailer::smtp(
        &format!("smtp://127.0.0.1:{port}"),
        "rueder <rueder@reader.example>",
        SITE,
    )
    .unwrap();
    set_mode(&db, Mode::Optional).await;
    let state = AppState::new(db.clone()).with_mail(mailer);
    let (_, cookie) = reader(&db, "ada").await;
    let (status, _) = change(&state, &cookie, "nobody@example.org").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(got.lock().unwrap().is_empty());
    assert_eq!(links(&db).await, 0);
    let me = me(&state, &cookie).await;
    assert_eq!(me["email_pending"], Value::Null);
}

#[sqlx::test]
async fn of_links_asked_for_at_once_one_is_left(db: PgPool) {
    let hour = Duration::from_secs(3600);
    let room = Rate::new(100, hour);
    let (state, outbox) = server(&db, Mode::Optional).await;
    let state = state.with_rates(Rates::default().with_mails(room, room, room));
    let (_, cookie) = reader(&db, "ada").await;
    let asked: Vec<_> = (0..8)
        .map(|n| {
            let (state, cookie) = (state.clone(), cookie.clone());
            tokio::spawn(async move {
                change(&state, &cookie, &format!("ada{n}@example.org"))
                    .await
                    .0
            })
        })
        .collect();
    for asking in asked {
        assert_eq!(asking.await.unwrap(), StatusCode::ACCEPTED);
    }
    assert_eq!(sent(&outbox).len(), 8);
    assert_eq!(links(&db).await, 1);
}
