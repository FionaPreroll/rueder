//! The login through an OpenID Connect provider, against a made-up provider.

mod common;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{
    Form, Json, Router,
    body::Body,
    extract::State,
    http::{HeaderMap, Request, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use http_body_util::BodyExt;
use rueder::{
    AppState,
    http_client::{self, HttpClient},
    oidc::{self, Config, Oidc},
    router, sessions,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use time::OffsetDateTime;
use tower::ServiceExt;

const SECRET: &str = "s3cret/with:odd chars";
const REDIRECT: &str = "https://reader.example/api/oidc/callback";

/// What the made-up provider says and what it was asked.
struct Provider {
    discovery: Value,
    token_status: StatusCode,
    token_body: String,
    posted: Vec<(HashMap<String, String>, Option<String>)>,
}

type Shared = Arc<Mutex<Provider>>;
/// A change to the claims of an ID token, and a body made for a nonce.
type Change = Box<dyn Fn(&mut Value)>;
type Answer<'a> = Box<dyn Fn(&str) -> String + 'a>;

async fn discovery(State(provider): State<Shared>) -> Json<Value> {
    Json(provider.lock().unwrap().discovery.clone())
}

async fn token(
    State(provider): State<Shared>,
    headers: HeaderMap,
    Form(form): Form<HashMap<String, String>>,
) -> Response {
    let mut provider = provider.lock().unwrap();
    let authorization = headers
        .get(header::AUTHORIZATION)
        .map(|value| value.to_str().unwrap().to_owned());
    provider.posted.push((form, authorization));
    let mut response = (provider.token_status, provider.token_body.clone()).into_response();
    if provider.token_status == StatusCode::FOUND {
        let elsewhere = "/elsewhere".parse().unwrap();
        response.headers_mut().insert(header::LOCATION, elsewhere);
    }
    response
}

/// A server with a provider next to it. Returns the provider's address too.
async fn setup(db: &PgPool) -> (AppState, Shared, String) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let issuer = format!("http://{}", listener.local_addr().unwrap());
    let provider = Arc::new(Mutex::new(Provider {
        discovery: json!({
            "issuer": issuer,
            "authorization_endpoint": format!("{issuer}/authorize?tenant=a"),
            "token_endpoint": format!("{issuer}/token"),
        }),
        token_status: StatusCode::OK,
        token_body: String::new(),
        posted: Vec::new(),
    }));
    let app = Router::new()
        .route("/.well-known/openid-configuration", get(discovery))
        .route("/token", post(token))
        .route("/elsewhere", post(token))
        .with_state(provider.clone());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let client = HttpClient::new(http_client::Config {
        allow: vec!["127.0.0.1".parse().unwrap()],
        ..Default::default()
    })
    .unwrap();
    let config = Config {
        issuer: issuer.clone(),
        client_id: "rueder".into(),
        client_secret: SECRET.into(),
        redirect_url: REDIRECT.into(),
        allow_http: true,
    };
    let oidc = Arc::new(Oidc::new(config, client).unwrap());
    (AppState::new(db.clone()).with_oidc(oidc), provider, issuer)
}

async fn send(state: &AppState, path: &str, cookies: &[&str]) -> (StatusCode, HeaderMap, String) {
    let mut request = Request::get(path);
    if !cookies.is_empty() {
        request = request.header(header::COOKIE, cookies.join("; "));
    }
    let app = router(state.clone(), "missing".as_ref());
    let response = app
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let (status, headers) = (response.status(), response.headers().clone());
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, headers, String::from_utf8(body.to_vec()).unwrap())
}

fn location(headers: &HeaderMap) -> &str {
    headers.get(header::LOCATION).unwrap().to_str().unwrap()
}

/// The cookies an answer sets, as `name=value`.
fn cookies(headers: &HeaderMap) -> Vec<String> {
    let all = headers.get_all(header::SET_COOKIE).iter();
    all.map(|set| set.to_str().unwrap().split(';').next().unwrap().to_owned())
        .collect()
}

/// One login on its way: what the browser keeps and what the provider got.
struct Flow {
    cookie: String,
    state: String,
    nonce: String,
    challenge: String,
}

async fn start(state: &AppState, path: &str) -> Flow {
    let (status, headers, _) = send(state, path, &[]).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    let to = reqwest::Url::parse(location(&headers)).unwrap();
    let asked: HashMap<_, _> = to.query_pairs().into_owned().collect();
    Flow {
        cookie: cookies(&headers).remove(0),
        state: asked["state"].clone(),
        nonce: asked["nonce"].clone(),
        challenge: asked["code_challenge"].clone(),
    }
}

fn id_token(claims: &Value) -> String {
    let part = |text: &str| URL_SAFE_NO_PAD.encode(text);
    format!(
        "{}.{}.{}",
        part(r#"{"alg":"RS256"}"#),
        part(&claims.to_string()),
        part("signature")
    )
}

fn claims(issuer: &str, subject: &str, nonce: &str) -> Value {
    json!({
        "iss": issuer,
        "sub": subject,
        "aud": "rueder",
        "exp": OffsetDateTime::now_utc().unix_timestamp() + 300,
        "iat": OffsetDateTime::now_utc().unix_timestamp(),
        "nonce": nonce,
    })
}

fn answer_with(provider: &Shared, claims: &Value) {
    provider.lock().unwrap().token_body = json!({
        "access_token": "unused", "token_type": "Bearer", "id_token": id_token(claims),
    })
    .to_string();
}

/// The whole way for one subject: start, the provider's answer, back.
async fn log_in(
    state: &AppState,
    provider: &Shared,
    issuer: &str,
    subject: &str,
    start_at: &str,
    session: Option<&str>,
) -> (HeaderMap, Flow) {
    let flow = start(state, start_at).await;
    answer_with(provider, &claims(issuer, subject, &flow.nonce));
    let path = format!("/api/oidc/callback?code=the-code&state={}", flow.state);
    let mut bring = vec![flow.cookie.as_str()];
    bring.extend(session);
    let (status, headers, _) = send(state, &path, &bring).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    (headers, flow)
}

async fn count(db: &PgPool, table: &str) -> i64 {
    let query = match table {
        "accounts" => "SELECT count(*) FROM accounts",
        "sessions" => "SELECT count(*) FROM sessions",
        _ => "SELECT count(*) FROM account_identities",
    };
    sqlx::query_scalar(query).fetch_one(db).await.unwrap()
}

#[sqlx::test]
async fn the_browser_is_sent_to_the_provider(db: PgPool) {
    let (state, _, issuer) = setup(&db).await;
    let (status, headers, _) = send(&state, "/api/oidc/start", &[]).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    let to = reqwest::Url::parse(location(&headers)).unwrap();
    assert!(
        to.as_str()
            .starts_with(&format!("{issuer}/authorize?tenant=a&"))
    );
    let asked: HashMap<_, _> = to.query_pairs().into_owned().collect();
    assert_eq!(asked["response_type"], "code");
    assert_eq!(asked["client_id"], "rueder");
    assert_eq!(asked["redirect_uri"], REDIRECT);
    assert_eq!(asked["scope"], "openid");
    assert_eq!(asked["code_challenge_method"], "S256");
    for random in ["state", "nonce", "code_challenge"] {
        assert_eq!(asked[random].len(), 43, "{random}");
    }
    // The secret and what proves the code later stay behind.
    assert!(!to.as_str().contains("s3cret"));
    assert_eq!(asked.get("code_verifier"), None);

    let set = headers.get(header::SET_COOKIE).unwrap().to_str().unwrap();
    let (pair, attributes) = set.split_once("; ").unwrap();
    assert!(pair.starts_with("__Host-rueder-oidc="));
    assert_eq!(
        attributes,
        "Path=/; Max-Age=600; HttpOnly; Secure; SameSite=Lax"
    );

    // Every login has its own secrets.
    let (one, two) = (
        start(&state, "/api/oidc/start").await,
        start(&state, "/api/oidc/start").await,
    );
    assert_ne!(one.state, two.state);
    assert_ne!(one.nonce, two.nonce);
    assert_ne!(one.challenge, two.challenge);
}

#[sqlx::test]
async fn a_known_identity_is_logged_in(db: PgPool) {
    let (state, provider, issuer) = setup(&db).await;
    let ada = common::account(&db, "ada").await;
    common::account(&db, "grace").await;
    assert!(oidc::link(&db, " Ada", &issuer, "subject-1").await.unwrap());
    // The same identity cannot belong to a second account, and nobody can be
    // tied to an account that is not there.
    assert!(
        !oidc::link(&db, "grace", &issuer, "subject-1")
            .await
            .unwrap()
    );
    assert!(
        !oidc::link(&db, "nobody", &issuer, "subject-2")
            .await
            .unwrap()
    );

    let (headers, flow) = log_in(
        &state,
        &provider,
        &issuer,
        "subject-1",
        "/api/oidc/start",
        None,
    )
    .await;
    assert_eq!(location(&headers), "/");
    let set = cookies(&headers);
    assert_eq!(set[0], "__Host-rueder-oidc=");
    assert!(set[1].starts_with("__Host-rueder="), "{set:?}");
    let (status, _, body) = send(&state, "/api/me", &[&set[1]]).await;
    assert_eq!(status, StatusCode::OK);
    let me: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        (me["id"].as_i64(), &me["username"]),
        (Some(i64::from(ada)), &json!("ada"))
    );

    // What the provider got: the code, the proof that this server asked for
    // it, and the secret in the header, not in the form.
    let (form, authorization) = provider.lock().unwrap().posted.remove(0);
    assert_eq!(form["grant_type"], "authorization_code");
    assert_eq!(form["code"], "the-code");
    assert_eq!(form["redirect_uri"], REDIRECT);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(form["code_verifier"].as_bytes()));
    assert_eq!(challenge, flow.challenge);
    assert_eq!(form.get("client_secret"), None);
    let basic = authorization.unwrap();
    let basic = STANDARD
        .decode(basic.strip_prefix("Basic ").unwrap())
        .unwrap();
    assert_eq!(
        String::from_utf8(basic).unwrap(),
        "rueder:s3cret%2Fwith%3Aodd+chars"
    );

    // Logging in again ends the session the browser had.
    let (headers, _) = log_in(
        &state,
        &provider,
        &issuer,
        "subject-1",
        "/api/oidc/start",
        Some(&set[1]),
    )
    .await;
    assert_eq!(
        send(&state, "/api/me", &[&set[1]]).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(&state, "/api/me", &[&cookies(&headers)[1]]).await.0,
        StatusCode::OK
    );
}

#[sqlx::test]
async fn a_provider_that_wants_the_secret_in_the_form_gets_it_there(db: PgPool) {
    let (state, provider, issuer) = setup(&db).await;
    common::account(&db, "ada").await;
    oidc::link(&db, "ada", &issuer, "subject-1").await.unwrap();
    provider.lock().unwrap().discovery["token_endpoint_auth_methods_supported"] =
        json!(["client_secret_post", "private_key_jwt"]);

    let (headers, _) = log_in(
        &state,
        &provider,
        &issuer,
        "subject-1",
        "/api/oidc/start",
        None,
    )
    .await;
    assert_eq!(location(&headers), "/");
    let (form, authorization) = provider.lock().unwrap().posted.remove(0);
    assert_eq!(
        (form["client_id"].as_str(), form["client_secret"].as_str()),
        ("rueder", SECRET)
    );
    assert_eq!(authorization, None);
}

#[sqlx::test]
async fn nobody_gets_an_account_by_logging_in(db: PgPool) {
    let (state, provider, issuer) = setup(&db).await;
    let ada = common::account(&db, "ada").await;
    let grace = common::account(&db, "grace").await;
    let session = |id| {
        let db = db.clone();
        async move {
            format!(
                "{}={}",
                sessions::COOKIE,
                sessions::start(&db, id).await.unwrap()
            )
        }
    };
    let (ada_session, grace_session) = (session(ada).await, session(grace).await);

    // The provider knows this one, rueder does not.
    let (headers, _) = log_in(
        &state,
        &provider,
        &issuer,
        "subject-1",
        "/api/oidc/start",
        None,
    )
    .await;
    assert_eq!(location(&headers), "/?oidc=unknown");
    assert_eq!(cookies(&headers), ["__Host-rueder-oidc="]);
    // Not when logged in either, unless that is what was asked for, and not
    // when it was asked for by nobody.
    for (at, bring) in [
        ("/api/oidc/start", Some(ada_session.as_str())),
        ("/api/oidc/start?link", None),
        (
            "/api/oidc/start?link",
            Some("__Host-rueder=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"),
        ),
    ] {
        let (headers, _) = log_in(&state, &provider, &issuer, "subject-1", at, bring).await;
        assert_eq!(location(&headers), "/?oidc=unknown", "{at} {bring:?}");
    }
    assert_eq!(count(&db, "account_identities").await, 0);
    assert_eq!(count(&db, "accounts").await, 2);
    assert_eq!(count(&db, "sessions").await, 2);

    // Ada is logged in and ties the identity to her account.
    let (headers, _) = log_in(
        &state,
        &provider,
        &issuer,
        "subject-1",
        "/api/oidc/start?link",
        Some(&ada_session),
    )
    .await;
    assert_eq!(location(&headers), "/?oidc=linked");
    assert_eq!(cookies(&headers), ["__Host-rueder-oidc="]);
    assert_eq!(
        send(&state, "/api/me", &[&ada_session]).await.0,
        StatusCode::OK
    );

    // From now on it logs in as her, whoever else was logged in before.
    for (at, bring) in [
        ("/api/oidc/start", None),
        ("/api/oidc/start?link", Some(grace_session.as_str())),
    ] {
        let (headers, _) = log_in(&state, &provider, &issuer, "subject-1", at, bring).await;
        assert_eq!(location(&headers), "/");
        let (_, _, body) = send(&state, "/api/me", &[&cookies(&headers)[1]]).await;
        assert_eq!(
            serde_json::from_str::<Value>(&body).unwrap()["username"],
            "ada"
        );
    }
    let owners: Vec<i32> = sqlx::query_scalar("SELECT account_id FROM account_identities")
        .fetch_all(&db)
        .await
        .unwrap();
    assert_eq!(owners, [ada]);
    // Another subject of the same provider is still nobody here.
    let (headers, _) = log_in(
        &state,
        &provider,
        &issuer,
        "subject-2",
        "/api/oidc/start",
        None,
    )
    .await;
    assert_eq!(location(&headers), "/?oidc=unknown");
}

#[sqlx::test]
async fn what_does_not_belong_to_this_login_is_refused(db: PgPool) {
    let (state, provider, issuer) = setup(&db).await;
    common::account(&db, "ada").await;
    oidc::link(&db, "ada", &issuer, "subject-1").await.unwrap();
    let now = OffsetDateTime::now_utc().unix_timestamp();

    // Tokens that are not for this login.
    let mut wrong: Vec<(&str, Change)> = vec![
        (
            "another login's nonce",
            Box::new(|c| c["nonce"] = json!("another")),
        ),
        ("no nonce", Box::new(|c| c["nonce"] = Value::Null)),
        (
            "another issuer",
            Box::new(|c| c["iss"] = json!("https://evil.example")),
        ),
        (
            "for another client",
            Box::new(|c| c["aud"] = json!("another")),
        ),
        (
            "for others only",
            Box::new(|c| c["aud"] = json!(["a", "b"])),
        ),
        (
            "for several, asked for by another",
            Box::new(|c| {
                c["aud"] = json!(["rueder", "another"]);
                c["azp"] = json!("another");
            }),
        ),
        (
            "for several, asked for by nobody",
            Box::new(|c| c["aud"] = json!(["rueder", "another"])),
        ),
        (
            "asked for by another",
            Box::new(|c| c["azp"] = json!("another")),
        ),
        ("run out", Box::new(move |c| c["exp"] = json!(now - 600))),
        ("no end", Box::new(|c| c["exp"] = Value::Null)),
        ("nobody", Box::new(|c| c["sub"] = json!(""))),
        (
            "a very long name",
            Box::new(|c| c["sub"] = json!("s".repeat(256))),
        ),
        ("no subject", Box::new(|c| c["sub"] = Value::Null)),
    ];
    for (what, change) in wrong.drain(..) {
        let flow = start(&state, "/api/oidc/start").await;
        let mut claims = claims(&issuer, "subject-1", &flow.nonce);
        change(&mut claims);
        answer_with(&provider, &claims);
        let path = format!("/api/oidc/callback?code=c&state={}", flow.state);
        let (status, headers, _) = send(&state, &path, &[&flow.cookie]).await;
        assert_eq!(status, StatusCode::SEE_OTHER, "{what}");
        assert_eq!(location(&headers), "/?oidc=failed", "{what}");
        assert_eq!(cookies(&headers), ["__Host-rueder-oidc="], "{what}");
    }

    // Answers that are no ID token at all, or none of the provider's.
    let good = |nonce: &str| id_token(&claims(&issuer, "subject-1", nonce));
    let answers: Vec<(&str, StatusCode, Answer)> = vec![
        (
            "an error",
            StatusCode::BAD_REQUEST,
            Box::new(|n| json!({ "id_token": good(n) }).to_string()),
        ),
        (
            "a redirect",
            StatusCode::FOUND,
            Box::new(|n| json!({ "id_token": good(n) }).to_string()),
        ),
        ("no JSON", StatusCode::OK, Box::new(|_| "<html>".into())),
        (
            "no token",
            StatusCode::OK,
            Box::new(|_| json!({ "access_token": "a" }).to_string()),
        ),
        (
            "two parts",
            StatusCode::OK,
            Box::new(|_| json!({ "id_token": "a.b" }).to_string()),
        ),
        (
            "four parts",
            StatusCode::OK,
            Box::new(|n| json!({ "id_token": format!("{}.x", good(n)) }).to_string()),
        ),
        (
            "no base64",
            StatusCode::OK,
            Box::new(|_| json!({ "id_token": "a.%%%.c" }).to_string()),
        ),
        (
            "no claims",
            StatusCode::OK,
            Box::new(|_| json!({ "id_token": "e30.WzFd.c" }).to_string()),
        ),
        (
            "too long",
            StatusCode::OK,
            Box::new(|_| json!({ "id_token": "a".repeat(20_000) }).to_string()),
        ),
    ];
    for (what, status, body) in answers {
        let flow = start(&state, "/api/oidc/start").await;
        {
            let mut provider = provider.lock().unwrap();
            provider.token_status = status;
            provider.token_body = body(&flow.nonce);
        }
        let path = format!("/api/oidc/callback?code=c&state={}", flow.state);
        let (_, headers, _) = send(&state, &path, &[&flow.cookie]).await;
        assert_eq!(location(&headers), "/?oidc=failed", "{what}");
    }
    provider.lock().unwrap().token_status = StatusCode::OK;

    // A code that comes with another login's state, or to a browser that
    // started none: somebody else's login pushed onto this browser.
    let (mine, theirs) = (
        start(&state, "/api/oidc/start").await,
        start(&state, "/api/oidc/start").await,
    );
    answer_with(&provider, &claims(&issuer, "subject-1", &theirs.nonce));
    let asked_before = provider.lock().unwrap().posted.len();
    let their_answer = format!("/api/oidc/callback?code=c&state={}", theirs.state);
    for (what, path, bring) in [
        (
            "another state",
            their_answer.as_str(),
            vec![mine.cookie.as_str()],
        ),
        ("no cookie", their_answer.as_str(), vec![]),
        (
            "an empty cookie",
            "/api/oidc/callback?code=c&state=",
            vec!["__Host-rueder-oidc=..."],
        ),
        (
            "a cookie of two parts",
            "/api/oidc/callback?code=c&state=a",
            vec!["__Host-rueder-oidc=a.b"],
        ),
        (
            "no state",
            "/api/oidc/callback?code=c",
            vec![mine.cookie.as_str()],
        ),
        (
            "no code",
            "/api/oidc/callback?state=a",
            vec![mine.cookie.as_str()],
        ),
    ] {
        let (status, headers, _) = send(&state, path, &bring).await;
        assert_eq!(status, StatusCode::SEE_OTHER, "{what}");
        assert_eq!(location(&headers), "/?oidc=failed", "{what}");
    }
    // The provider was not even asked about those.
    assert_eq!(provider.lock().unwrap().posted.len(), asked_before);

    // The reader said no at the provider.
    let path = format!(
        "/api/oidc/callback?error=access_denied&state={}",
        mine.state
    );
    let (_, headers, _) = send(&state, &path, &[&mine.cookie]).await;
    assert_eq!(location(&headers), "/?oidc=denied");

    // After all of this nobody is logged in.
    assert_eq!(count(&db, "sessions").await, 0);
    // And the same browser can still log in properly.
    let (headers, _) = log_in(
        &state,
        &provider,
        &issuer,
        "subject-1",
        "/api/oidc/start",
        None,
    )
    .await;
    assert_eq!(location(&headers), "/");
}

#[sqlx::test]
async fn the_provider_has_to_be_the_one_that_was_set(db: PgPool) {
    // It calls itself something else.
    let (state, provider, _) = setup(&db).await;
    provider.lock().unwrap().discovery["issuer"] = json!("https://another.example");
    let (status, headers, _) = send(&state, "/api/oidc/start", &[]).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location(&headers), "/?oidc=failed");

    // It does not say where to log in.
    let (state, provider, issuer) = setup(&db).await;
    provider.lock().unwrap().discovery = json!({ "issuer": issuer });
    let (_, headers, _) = send(&state, "/api/oidc/start", &[]).await;
    assert_eq!(location(&headers), "/?oidc=failed");

    // It sends logins somewhere that is no web address.
    let (state, provider, _) = setup(&db).await;
    provider.lock().unwrap().discovery["token_endpoint"] = json!("file:///etc/passwd");
    let (_, headers, _) = send(&state, "/api/oidc/start", &[]).await;
    assert_eq!(location(&headers), "/?oidc=failed");

    // It is somewhere this server may not reach.
    let client = HttpClient::new(http_client::Config::default()).unwrap();
    let config = Config {
        issuer: "http://127.0.0.1:9".into(),
        client_id: "rueder".into(),
        client_secret: SECRET.into(),
        redirect_url: REDIRECT.into(),
        allow_http: true,
    };
    let oidc = Arc::new(Oidc::new(config.clone(), client.clone()).unwrap());
    let state = AppState::new(db.clone()).with_oidc(oidc);
    let (_, headers, _) = send(&state, "/api/oidc/start", &[]).await;
    assert_eq!(location(&headers), "/?oidc=failed");

    // Without TLS nothing vouches for the ID token: such a provider is not
    // taken, and a server set up with one does not start.
    let plain = Config {
        allow_http: false,
        ..config.clone()
    };
    assert!(Oidc::new(plain, client.clone()).is_err());
    for issuer in ["", "accounts.example", "ftp://accounts.example"] {
        let config = Config {
            issuer: issuer.into(),
            ..config.clone()
        };
        assert!(Oidc::new(config, client.clone()).is_err(), "{issuer}");
    }
    let config = Config {
        issuer: "https://accounts.example".into(),
        redirect_url: "javascript:alert(1)".into(),
        allow_http: false,
        ..config
    };
    assert!(Oidc::new(config, client).is_err());
}

#[sqlx::test]
async fn without_a_provider_there_is_no_such_login(db: PgPool) {
    let state = AppState::new(db);
    for path in ["/api/oidc/start", "/api/oidc/callback?code=c&state=s"] {
        let (status, headers, _) = send(&state, path, &[]).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(headers.get(header::SET_COOKIE), None);
    }
}
