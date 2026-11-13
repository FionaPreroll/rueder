//! The operator's legal texts: the links, and agreeing to the terms of use at
//! registration and after a new version.

mod common;

use axum::http::{Method, StatusCode};
use common::{call, raw};
use rueder::{
    AppState,
    legal::{Legal, MAX_VERSION},
    registration,
};
use serde_json::{Value, json};
use sqlx::PgPool;

const PASSWORD: &str = "correct horse battery";

fn legal(pairs: &[(&str, &str)]) -> anyhow::Result<Legal> {
    let pairs: Vec<(String, String)> = pairs
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
    Legal::from_vars(move |name| {
        pairs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
    })
}

fn terms(version: &str) -> Legal {
    legal(&[
        ("LEGAL_IMPRINT_URL", "https://operator.example/imprint"),
        ("LEGAL_PRIVACY_URL", "https://operator.example/privacy"),
        ("LEGAL_TERMS_URL", "https://operator.example/terms"),
        ("TERMS_VERSION", version),
    ])
    .unwrap()
}

#[test]
fn the_settings_are_whole_addresses_and_a_short_version() {
    // Without them there is nothing.
    assert_eq!(legal(&[]).unwrap(), Legal::default());
    assert_eq!(
        legal(&[("LEGAL_TERMS_URL", " ")]).unwrap(),
        Legal::default()
    );
    let some = legal(&[("LEGAL_IMPRINT_URL", " https://operator.example/imprint ")]).unwrap();
    assert_eq!(
        some.imprint.as_deref(),
        Some("https://operator.example/imprint")
    );
    assert_eq!(
        (some.privacy, some.terms, some.terms_version),
        (None, None, None)
    );

    for wrong in [
        ("LEGAL_IMPRINT_URL", "operator.example/imprint"),
        ("LEGAL_PRIVACY_URL", "javascript:alert(1)"),
        ("LEGAL_TERMS_URL", "ftp://operator.example/terms"),
        ("LEGAL_TERMS_URL", "data:text/html,hi"),
    ] {
        let error = legal(&[wrong]).unwrap_err().to_string();
        assert!(error.contains(wrong.0), "{wrong:?}: {error}");
    }
    let long = format!("https://operator.example/{}", "a".repeat(2000));
    assert!(legal(&[("LEGAL_TERMS_URL", &long)]).is_err());

    // A version needs terms to agree to, and is one short line.
    assert!(legal(&[("TERMS_VERSION", "2026-11")]).is_err());
    assert!(
        legal(&[
            ("LEGAL_TERMS_URL", "https://operator.example/terms"),
            ("TERMS_VERSION", &"v".repeat(MAX_VERSION + 1)),
        ])
        .is_err()
    );
    assert!(
        legal(&[
            ("LEGAL_TERMS_URL", "https://operator.example/terms"),
            ("TERMS_VERSION", "one\ntwo"),
        ])
        .is_err()
    );
    assert_eq!(terms(" 2026-11 ").terms_version.as_deref(), Some("2026-11"));
}

#[sqlx::test]
async fn the_login_page_gets_the_links_and_nothing_without_them(db: PgPool) {
    let plain = AppState::new(db.clone());
    let (_, ways) = call(&plain, Method::GET, "/api/login/ways", None, None).await;
    assert_eq!(
        ways["legal"],
        json!({ "imprint": null, "privacy": null, "terms": null, "terms_version": null })
    );

    let state = AppState::new(db.clone()).with_legal(terms("2026-11"));
    let (status, ways) = call(&state, Method::GET, "/api/login/ways", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        ways["legal"],
        json!({
            "imprint": "https://operator.example/imprint",
            "privacy": "https://operator.example/privacy",
            "terms": "https://operator.example/terms",
            "terms_version": "2026-11"
        })
    );

    // Without TERMS_VERSION nobody is asked, and there is nothing to agree to.
    let ada = common::account(&db, "ada").await;
    let cookie = common::session(&db, ada).await;
    let (_, me) = call(&plain, Method::GET, "/api/me", Some(&cookie), None).await;
    assert_eq!(me["terms_to_accept"], Value::Null);
    let agree = json!({ "version": "2026-11" });
    let (status, _) = call(
        &plain,
        Method::PUT,
        "/api/terms",
        Some(&cookie),
        Some(agree),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn registering_takes_the_agreement_to_the_terms(db: PgPool) {
    let mut settings = registration::settings(&db).await.unwrap();
    settings.open = true;
    registration::set_settings(&db, &settings).await.unwrap();
    let state = AppState::new(db.clone()).with_legal(terms("2026-11"));
    let register = |accepted: Option<&str>| {
        let mut body = json!({ "username": "grace", "password": PASSWORD });
        if let Some(accepted) = accepted {
            body["accepted_terms"] = json!(accepted);
        }
        let body = ("application/json", body.to_string().into_bytes());
        raw(&state, Method::POST, "/api/register", None, Some(body))
    };

    // Not without agreeing, nor to another version.
    for accepted in [None, Some("2026-10"), Some("")] {
        let (status, _, word) = register(accepted).await;
        assert_eq!(
            (status, String::from_utf8(word).unwrap().as_str()),
            (StatusCode::FORBIDDEN, "terms"),
            "{accepted:?}"
        );
    }
    let accounts: i64 = sqlx::query_scalar("SELECT count(*) FROM accounts")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(accounts, 0);

    let (status, _, me) = register(Some("2026-11")).await;
    assert_eq!(status, StatusCode::CREATED);
    let me: Value = serde_json::from_slice(&me).unwrap();
    assert_eq!(me["terms_to_accept"], Value::Null);
    let (version, when): (Option<String>, Option<time::OffsetDateTime>) = sqlx::query_as(
        "SELECT terms_version, terms_accepted_at FROM accounts WHERE username = 'grace'",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(version.as_deref(), Some("2026-11"));
    assert!(when.is_some());
}

#[sqlx::test]
async fn a_new_version_is_asked_for_at_the_next_visit(db: PgPool) {
    let ada = common::account(&db, "ada").await;
    let grace = common::account(&db, "grace").await;
    let (ada_cookie, grace_cookie) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
    );
    let first = AppState::new(db.clone()).with_legal(terms("2026-11"));
    let me = |state: &AppState, cookie: &str| {
        let (state, cookie) = (state.clone(), cookie.to_owned());
        async move {
            call(&state, Method::GET, "/api/me", Some(&cookie), None)
                .await
                .1
        }
    };

    // An account the operator made agreed to nothing yet.
    assert_eq!(me(&first, &ada_cookie).await["terms_to_accept"], "2026-11");
    let old = json!({ "version": "2026-10" });
    let (status, _) = call(
        &first,
        Method::PUT,
        "/api/terms",
        Some(&ada_cookie),
        Some(old),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let agree = json!({ "version": "2026-11" });
    let (status, _) = call(
        &first,
        Method::PUT,
        "/api/terms",
        Some(&ada_cookie),
        Some(agree),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        me(&first, &ada_cookie).await["terms_to_accept"],
        Value::Null
    );
    // Each account agrees for itself.
    assert_eq!(
        me(&first, &grace_cookie).await["terms_to_accept"],
        "2026-11"
    );

    // The operator names a new version: the next visit asks again.
    let second = AppState::new(db.clone()).with_legal(terms("2027-01"));
    assert_eq!(me(&second, &ada_cookie).await["terms_to_accept"], "2027-01");

    // Nobody agrees without a session.
    let agree = json!({ "version": "2027-01" });
    let (status, _) = call(&second, Method::PUT, "/api/terms", None, Some(agree)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
