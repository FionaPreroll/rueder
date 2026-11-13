//! Passkeys: adding one, logging in with it without a name, showing with it
//! that the owner is there, and removing it, against a device made here that
//! signs as a real one does (P-256, attestation `none`). And what a device or
//! a page that is not the one it claims sends: other origins, other sites,
//! old challenges, a missing user verification, bad signatures, a counter
//! that goes back.

mod common;

use axum::http::{HeaderMap, Method, StatusCode, header};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use common::{call, raw};
use p256::ecdsa::{Signature, SigningKey, signature::Signer};
use rueder::{AppState, accounts, login, passkeys::Passkeys, second_factor};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

const PASSWORD: &str = "correct horse battery";
const SITE: &str = "https://reader.example";

fn b64(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

fn unb64(text: &Value) -> Vec<u8> {
    URL_SAFE_NO_PAD.decode(text.as_str().unwrap()).unwrap()
}

/// The parts of CBOR that WebAuthn needs, written by hand.
fn head(major: u8, len: usize) -> Vec<u8> {
    let major = major << 5;
    match len {
        0..24 => vec![major | len as u8],
        24..256 => vec![major | 24, len as u8],
        _ => {
            let mut out = vec![major | 25];
            out.extend_from_slice(&(len as u16).to_be_bytes());
            out
        }
    }
}

fn text(value: &str) -> Vec<u8> {
    let mut out = head(3, value.len());
    out.extend_from_slice(value.as_bytes());
    out
}

fn bytes(value: &[u8]) -> Vec<u8> {
    let mut out = head(2, value.len());
    out.extend_from_slice(value);
    out
}

/// A passkey's device: one key, kept with the user it was made for.
struct Device {
    key: SigningKey,
    id: Vec<u8>,
    user: Vec<u8>,
    counter: u32,
    /// Where the page that asks says it is.
    origin: String,
    /// The flags it sets: user present and verified.
    flags: u8,
}

impl Device {
    fn new() -> Self {
        let mut secret = [0u8; 32];
        getrandom::fill(&mut secret).unwrap();
        let mut id = [0u8; 32];
        getrandom::fill(&mut id).unwrap();
        Self {
            key: SigningKey::from_slice(&secret).unwrap(),
            id: id.to_vec(),
            user: Vec::new(),
            counter: 0,
            origin: SITE.into(),
            flags: 0x01 | 0x04,
        }
    }

    fn cose_key(&self) -> Vec<u8> {
        let point = self.key.verifying_key().to_sec1_point(false);
        let mut out = head(5, 5);
        out.extend([0x01, 0x02]); // kty: EC2
        out.extend([0x03, 0x26]); // alg: ES256 (-7)
        out.extend([0x20, 0x01]); // crv: P-256
        out.push(0x21); // x
        out.extend(bytes(point.x().unwrap()));
        out.push(0x22); // y
        out.extend(bytes(point.y().unwrap()));
        out
    }

    fn client_data(&self, kind: &str, challenge: &Value) -> Vec<u8> {
        json!({
            "type": kind,
            "challenge": challenge,
            "origin": self.origin,
            "crossOrigin": false,
        })
        .to_string()
        .into_bytes()
    }

    /// What `navigator.credentials.create()` gives the page.
    fn create(&mut self, challenge: &Value) -> Value {
        let options = &challenge["options"]["publicKey"];
        self.user = unb64(&options["user"]["id"]);
        let rp = options["rp"]["id"].as_str().unwrap();
        let client_data = self.client_data("webauthn.create", &options["challenge"]);
        let mut auth_data = Sha256::digest(rp.as_bytes()).to_vec();
        auth_data.push(self.flags | 0x40); // attested credential data
        auth_data.extend_from_slice(&self.counter.to_be_bytes());
        auth_data.extend_from_slice(&[0u8; 16]); // AAGUID
        auth_data.extend_from_slice(&(self.id.len() as u16).to_be_bytes());
        auth_data.extend_from_slice(&self.id);
        auth_data.extend(self.cose_key());
        let mut attestation = head(5, 3);
        attestation.extend(text("fmt"));
        attestation.extend(text("none"));
        attestation.extend(text("attStmt"));
        attestation.extend(head(5, 0));
        attestation.extend(text("authData"));
        attestation.extend(bytes(&auth_data));
        json!({
            "ceremony": challenge["ceremony"],
            "credential": {
                "id": b64(&self.id),
                "rawId": b64(&self.id),
                "type": "public-key",
                "response": {
                    "attestationObject": b64(&attestation),
                    "clientDataJSON": b64(&client_data),
                    "transports": ["internal"],
                },
                "extensions": {},
            },
        })
    }

    /// What `navigator.credentials.get()` gives the page.
    fn get(&mut self, challenge: &Value) -> Value {
        let options = &challenge["options"]["publicKey"];
        let rp = options["rpId"].as_str().unwrap();
        self.counter += 1;
        let client_data = self.client_data("webauthn.get", &options["challenge"]);
        let mut auth_data = Sha256::digest(rp.as_bytes()).to_vec();
        auth_data.push(self.flags);
        auth_data.extend_from_slice(&self.counter.to_be_bytes());
        let mut signed = auth_data.clone();
        signed.extend(Sha256::digest(&client_data));
        let signature: Signature = self.key.sign(&signed);
        json!({
            "ceremony": challenge["ceremony"],
            "credential": {
                "id": b64(&self.id),
                "rawId": b64(&self.id),
                "type": "public-key",
                "response": {
                    "authenticatorData": b64(&auth_data),
                    "clientDataJSON": b64(&client_data),
                    "signature": b64(&signature.to_der().to_bytes()),
                    "userHandle": b64(&self.user),
                },
                "extensions": {},
            },
        })
    }
}

fn with_passkeys(db: &PgPool) -> AppState {
    AppState::new(db.clone()).with_passkeys(Passkeys::new(SITE).unwrap())
}

async fn ada(db: &PgPool) -> i32 {
    accounts::create(db, "ada", Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap()
}

/// Starts a ceremony, as the page does.
async fn start(state: &AppState, path: &str, cookie: Option<&str>, body: Value) -> Value {
    let (status, challenge) = call(state, Method::POST, path, cookie, Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{path}: {challenge}");
    challenge
}

/// Adds a passkey of this device with the password, and returns the answer.
async fn add(state: &AppState, cookie: &str, device: &mut Device, proof: Value) -> Value {
    let mut body = json!({ "name": "Phone" });
    body.as_object_mut()
        .unwrap()
        .extend(proof.as_object().unwrap().clone());
    let challenge = start(state, "/api/passkeys/register", Some(cookie), body).await;
    let (status, added) = call(
        state,
        Method::POST,
        "/api/passkeys",
        Some(cookie),
        Some(device.create(&challenge)),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{added}");
    added
}

/// Logs in with the device, as the login page does; the cookie if it worked.
async fn log_in(state: &AppState, device: &mut Device) -> (StatusCode, Option<String>, Value) {
    let challenge = start(state, "/api/login/passkey", None, json!({})).await;
    finish_login(state, device.get(&challenge)).await
}

async fn finish_login(state: &AppState, answer: Value) -> (StatusCode, Option<String>, Value) {
    let body = ("application/json", answer.to_string().into_bytes());
    let (status, headers, body) = raw(
        state,
        Method::POST,
        "/api/login/passkey/finish",
        None,
        Some(body),
    )
    .await;
    let json = serde_json::from_slice(&body).unwrap_or(Value::Null);
    (status, cookie_of(&headers), json)
}

fn cookie_of(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::SET_COOKIE)?.to_str().ok()?;
    let pair = value.split(';').next()?;
    (!pair.ends_with('=')).then(|| pair.to_owned())
}

/// A passkey's answer that the owner is there, for a change that needs it.
async fn proof(state: &AppState, cookie: &str, device: &mut Device) -> Value {
    let challenge = start(state, "/api/passkeys/proof", Some(cookie), json!({})).await;
    json!({ "passkey": device.get(&challenge) })
}

async fn second_factor_of(state: &AppState, cookie: &str) -> Value {
    call(state, Method::GET, "/api/second-factor", Some(cookie), None)
        .await
        .1
}

async fn session_has_second_factor(db: &PgPool, cookie: &str) -> bool {
    let token = cookie.split_once('=').unwrap().1;
    sqlx::query_scalar("SELECT second_factor FROM sessions WHERE token_hash = sha256($1::bytea)")
        .bind(token.as_bytes())
        .fetch_one(db)
        .await
        .unwrap()
}

#[sqlx::test]
async fn without_a_public_url_there_are_none(db: PgPool) {
    let ada = ada(&db).await;
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;
    for (method, path) in [
        (Method::GET, "/api/passkeys"),
        (Method::POST, "/api/passkeys"),
        (Method::POST, "/api/passkeys/register"),
        (Method::POST, "/api/passkeys/proof"),
        (Method::DELETE, "/api/passkeys/1"),
        (Method::POST, "/api/login/passkey"),
        (Method::POST, "/api/login/passkey/finish"),
    ] {
        let body =
            json!({ "name": "Phone", "password": PASSWORD, "ceremony": "x", "credential": {} });
        let (status, _) = call(&state, method, path, Some(&cookie), Some(body)).await;
        assert!(
            status == StatusCode::NOT_FOUND || status == StatusCode::UNPROCESSABLE_ENTITY,
            "{path}: {status}"
        );
    }
    let (_, me) = call(&state, Method::GET, "/api/me", Some(&cookie), None).await;
    assert_eq!(me["passkeys"], false);
    let (_, ways) = call(&state, Method::GET, "/api/login/ways", None, None).await;
    assert_eq!(ways["passkeys"], false);
    let state = with_passkeys(&db);
    let (_, me) = call(&state, Method::GET, "/api/me", Some(&cookie), None).await;
    assert_eq!(me["passkeys"], true);
    let (_, ways) = call(&state, Method::GET, "/api/login/ways", None, None).await;
    assert_eq!(ways["passkeys"], true);
}

#[sqlx::test]
async fn a_passkey_is_added_with_the_password_and_logs_in_without_a_name(db: PgPool) {
    let ada = ada(&db).await;
    let state = with_passkeys(&db);
    let cookie = common::session(&db, ada).await;
    let mut device = Device::new();

    // Not without the password, nor with a wrong one, nor without a name.
    for (body, wanted) in [
        (json!({ "name": "Phone" }), StatusCode::FORBIDDEN),
        (
            json!({ "name": "Phone", "password": "wrong" }),
            StatusCode::FORBIDDEN,
        ),
        (
            json!({ "name": " ", "password": PASSWORD }),
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let (status, _) = call(
            &state,
            Method::POST,
            "/api/passkeys/register",
            Some(&cookie),
            Some(body),
        )
        .await;
        assert_eq!(status, wanted);
    }

    // What the device is asked: a passkey it keeps itself, with its PIN or
    // biometrics, for this site only, and nothing about the device.
    let challenge = start(
        &state,
        "/api/passkeys/register",
        Some(&cookie),
        json!({ "name": " Phone ", "password": PASSWORD }),
    )
    .await;
    let options = &challenge["options"]["publicKey"];
    assert_eq!(options["rp"]["id"], "reader.example");
    assert_eq!(options["user"]["name"], "ada");
    assert_eq!(options["attestation"], "none");
    let selection = &options["authenticatorSelection"];
    assert_eq!(selection["residentKey"], "required");
    assert_eq!(selection["userVerification"], "required");
    // The user handle is random, not the account's number or name.
    let user = unb64(&options["user"]["id"]);
    assert_eq!(user.len(), 16);
    assert!(!String::from_utf8_lossy(&user).contains("ada"));

    let (status, added) = call(
        &state,
        Method::POST,
        "/api/passkeys",
        Some(&cookie),
        Some(device.create(&challenge)),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{added}");
    assert_eq!(added["passkey"]["name"], "Phone");
    assert_eq!(added["passkey"]["used_at"], Value::Null);
    // The second factor was off: the first passkey brings the recovery
    // codes, this once, and this session counts as one with it.
    let codes = added["recovery_codes"].as_array().unwrap().clone();
    assert_eq!(codes.len(), second_factor::RECOVERY_CODES);
    assert!(session_has_second_factor(&db, &cookie).await);
    let status = second_factor_of(&state, &cookie).await;
    assert_eq!(status["on"], true);
    assert_eq!(status["app"], false);
    assert_eq!(status["passkeys"], 1);
    assert_eq!(status["recovery_codes_left"], 10);
    let (_, list) = call(&state, Method::GET, "/api/passkeys", Some(&cookie), None).await;
    assert_eq!(list["max"], 10);
    assert_eq!(list["passkeys"].as_array().unwrap().len(), 1);
    // What is kept is the public key, never more than the device sent.
    let stored: String = sqlx::query_scalar("SELECT credential::text FROM passkeys")
        .fetch_one(&db)
        .await
        .unwrap();
    assert!(!stored.contains("\"ada\""), "{stored}");

    // Logging in needs neither name nor password, and counts as a second
    // factor.
    let (status, logged_in, me) = log_in(&state, &mut device).await;
    assert_eq!(status, StatusCode::OK, "{me}");
    assert_eq!(me["username"], "ada");
    assert_eq!(me["has_password"], true);
    let logged_in = logged_in.unwrap();
    assert!(session_has_second_factor(&db, &logged_in).await);
    let (_, list) = call(&state, Method::GET, "/api/passkeys", Some(&logged_in), None).await;
    assert!(list["passkeys"][0]["used_at"].is_string());

    // With a passkey the password alone opens nothing; a recovery code
    // still does.
    let login = |code: Option<&str>| {
        let state = state.clone();
        let body = json!({ "username": "ada", "password": PASSWORD, "code": code });
        async move {
            let body = ("application/json", body.to_string().into_bytes());
            let (status, _, said) = raw(&state, Method::POST, "/api/login", None, Some(body)).await;
            (status, String::from_utf8_lossy(&said).into_owned())
        }
    };
    let (status, said) = login(None).await;
    assert_eq!(
        (status, said.as_str()),
        (StatusCode::UNAUTHORIZED, login::CODE_NEEDED)
    );
    let (status, _) = login(Some(codes[0].as_str().unwrap())).await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn a_passkey_shows_the_owner_is_there(db: PgPool) {
    let ada = ada(&db).await;
    let state = with_passkeys(&db);
    let cookie = common::session(&db, ada).await;
    let mut device = Device::new();
    add(
        &state,
        &cookie,
        &mut device,
        json!({ "password": PASSWORD }),
    )
    .await;

    // A second passkey: the password alone is no longer enough, the first
    // passkey is.
    let mut laptop = Device::new();
    let (status, said) = call(
        &state,
        Method::POST,
        "/api/passkeys/register",
        Some(&cookie),
        Some(json!({ "name": "Laptop", "password": PASSWORD })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{said}");
    let shown = proof(&state, &cookie, &mut device).await;
    let added = add(&state, &cookie, &mut laptop, shown).await;
    // The second factor was on already: no new codes.
    assert!(added.get("recovery_codes").is_none());

    // An app password with a passkey instead of password and code.
    let mut body = proof(&state, &cookie, &mut laptop).await;
    body["name"] = "Reeder".into();
    let (status, made) = call(
        &state,
        Method::POST,
        "/api/app-passwords",
        Some(&cookie),
        Some(body.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    // An answer counts once.
    let (status, _) = call(
        &state,
        Method::POST,
        "/api/app-passwords",
        Some(&cookie),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Removing one takes the proof too; the other one stays, and with it the
    // second factor.
    let (_, list) = call(&state, Method::GET, "/api/passkeys", Some(&cookie), None).await;
    let laptop_id = list["passkeys"][1]["id"].as_i64().unwrap();
    let path = format!("/api/passkeys/{laptop_id}");
    let (status, _) = call(
        &state,
        Method::DELETE,
        &path,
        Some(&cookie),
        Some(json!({ "password": PASSWORD })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let shown = proof(&state, &cookie, &mut laptop).await;
    let (status, _) = call(&state, Method::DELETE, &path, Some(&cookie), Some(shown)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(second_factor_of(&state, &cookie).await["passkeys"], 1);
    // The removed one logs in no more.
    let (status, cookie_of_laptop, _) = log_in(&state, &mut laptop).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(cookie_of_laptop.is_none());

    // Switching the second factor off is the app's: with a passkey left it
    // stays on, and so do the recovery codes.
    let shown = proof(&state, &cookie, &mut device).await;
    let (status, _) = call(
        &state,
        Method::DELETE,
        "/api/second-factor",
        Some(&cookie),
        Some(shown),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let status = second_factor_of(&state, &cookie).await;
    assert_eq!(
        (status["on"].clone(), status["recovery_codes_left"].clone()),
        (json!(true), json!(10))
    );

    // The last passkey goes with the second factor and the codes.
    let (_, list) = call(&state, Method::GET, "/api/passkeys", Some(&cookie), None).await;
    let path = format!("/api/passkeys/{}", list["passkeys"][0]["id"]);
    let shown = proof(&state, &cookie, &mut device).await;
    let (status, _) = call(&state, Method::DELETE, &path, Some(&cookie), Some(shown)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let status = second_factor_of(&state, &cookie).await;
    assert_eq!(
        (status["on"].clone(), status["recovery_codes_left"].clone()),
        (json!(false), json!(0))
    );
    assert!(!session_has_second_factor(&db, &cookie).await);

    // Deleting the account with a passkey.
    let mut device = Device::new();
    add(
        &state,
        &cookie,
        &mut device,
        json!({ "password": PASSWORD }),
    )
    .await;
    let shown = proof(&state, &cookie, &mut device).await;
    let (status, _) = call(
        &state,
        Method::DELETE,
        "/api/account",
        Some(&cookie),
        Some(shown),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM passkeys")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(left, 0);
}

#[sqlx::test]
async fn others_see_nothing_and_use_nothing(db: PgPool) {
    let ada = ada(&db).await;
    let grace = accounts::create(&db, "grace", Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap();
    let state = with_passkeys(&db);
    let (ours, theirs) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
    );
    let mut device = Device::new();
    add(&state, &ours, &mut device, json!({ "password": PASSWORD })).await;
    let (_, list) = call(&state, Method::GET, "/api/passkeys", Some(&ours), None).await;
    let id = list["passkeys"][0]["id"].as_i64().unwrap();

    let (_, list) = call(&state, Method::GET, "/api/passkeys", Some(&theirs), None).await;
    assert_eq!(list["passkeys"], json!([]));
    // Grace has no passkey to show anything with.
    let (status, _) = call(
        &state,
        Method::POST,
        "/api/passkeys/proof",
        Some(&theirs),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    // Not Ada's passkey, whatever Grace shows.
    let path = format!("/api/passkeys/{id}");
    let (status, _) = call(
        &state,
        Method::DELETE,
        &path,
        Some(&theirs),
        Some(json!({ "password": PASSWORD })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // An answer to Ada's ceremony counts in no session of Grace's.
    let shown = proof(&state, &ours, &mut device).await;
    let mut body = shown.clone();
    body["name"] = "Reeder".into();
    let (status, _) = call(
        &state,
        Method::POST,
        "/api/app-passwords",
        Some(&theirs),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    // Nor in another session of Ada's than the one that asked.
    let other = common::session(&db, ada).await;
    let shown = proof(&state, &ours, &mut device).await;
    let (status, _) = call(&state, Method::DELETE, &path, Some(&other), Some(shown)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    // A registration another session began is not finished in this one.
    let challenge = start(
        &state,
        "/api/passkeys/register",
        Some(&theirs),
        json!({ "name": "Phone", "password": PASSWORD }),
    )
    .await;
    let (status, _) = call(
        &state,
        Method::POST,
        "/api/passkeys",
        Some(&ours),
        Some(Device::new().create(&challenge)),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Without a session, nothing at all but the login.
    for (method, path) in [
        (Method::GET, "/api/passkeys"),
        (Method::POST, "/api/passkeys"),
        (Method::POST, "/api/passkeys/register"),
        (Method::POST, "/api/passkeys/proof"),
        (Method::DELETE, path.as_str()),
    ] {
        let body =
            json!({ "name": "Phone", "password": PASSWORD, "ceremony": "x", "credential": {} });
        let (status, _) = call(&state, method, path, None, Some(body)).await;
        assert!(
            status == StatusCode::UNAUTHORIZED || status == StatusCode::UNPROCESSABLE_ENTITY,
            "{path}: {status}"
        );
    }
}

#[sqlx::test]
async fn what_breaks_the_rules_logs_nobody_in(db: PgPool) {
    let ada = ada(&db).await;
    let state = with_passkeys(&db);
    let cookie = common::session(&db, ada).await;
    let mut device = Device::new();
    add(
        &state,
        &cookie,
        &mut device,
        json!({ "password": PASSWORD }),
    )
    .await;

    let refused = |answer: Value| {
        let state = state.clone();
        async move {
            let (status, cookie, _) = finish_login(&state, answer).await;
            assert!(cookie.is_none());
            status
        }
    };
    let challenge = || start(&state, "/api/login/passkey", None, json!({}));

    // A page elsewhere that relays the challenge: the device says where it is.
    let mut elsewhere = Device {
        origin: "https://reader.example.evil".into(),
        ..Device::new()
    };
    elsewhere.key = device.key.clone();
    elsewhere.id = device.id.clone();
    elsewhere.user = device.user.clone();
    elsewhere.counter = 10;
    assert_eq!(
        refused(elsewhere.get(&challenge().await)).await,
        StatusCode::UNAUTHORIZED
    );
    // A key made for another site.
    let mut answer = device.get(&challenge().await);
    let other_site = Sha256::digest(b"evil.example");
    let mut data = unb64(&answer["credential"]["response"]["authenticatorData"]);
    data[..32].copy_from_slice(&other_site);
    answer["credential"]["response"]["authenticatorData"] = b64(&data).into();
    assert_eq!(refused(answer).await, StatusCode::UNAUTHORIZED);
    // Without the PIN or biometrics, and without the reader touching it.
    for flags in [0x01, 0x04, 0x00] {
        device.flags = flags;
        assert_eq!(
            refused(device.get(&challenge().await)).await,
            StatusCode::UNAUTHORIZED,
            "{flags}"
        );
    }
    device.flags = 0x05;
    // A signature that does not fit what was signed.
    let mut answer = device.get(&challenge().await);
    let mut signature = unb64(&answer["credential"]["response"]["signature"]);
    let last = signature.len() - 1;
    signature[last] ^= 1;
    answer["credential"]["response"]["signature"] = b64(&signature).into();
    assert_eq!(refused(answer).await, StatusCode::UNAUTHORIZED);
    // The answer to another challenge.
    let first = device.get(&challenge().await);
    let mut other = device.get(&challenge().await);
    other["credential"] = first["credential"].clone();
    assert_eq!(refused(other).await, StatusCode::UNAUTHORIZED);
    // An answer that logged in counts once.
    let (status, logged_in, _) = finish_login(&state, first.clone()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(logged_in.is_some());
    assert_eq!(refused(first).await, StatusCode::CONFLICT);
    // A made-up ceremony, one far too long, and none at all.
    let mut answer = device.get(&challenge().await);
    for ceremony in [json!("made-up"), json!("x".repeat(100_000))] {
        answer["ceremony"] = ceremony;
        assert_eq!(refused(answer.clone()).await, StatusCode::CONFLICT);
    }
    // Another account's user handle with this key.
    let mut answer = device.get(&challenge().await);
    answer["credential"]["response"]["userHandle"] = b64(&[7u8; 16]).into();
    assert_eq!(refused(answer).await, StatusCode::UNAUTHORIZED);
    // A key nobody added.
    let mut stranger = Device::new();
    stranger.user = device.user.clone();
    assert_eq!(
        refused(stranger.get(&challenge().await)).await,
        StatusCode::UNAUTHORIZED
    );
    // A counter that goes back: the key may have been copied.
    device.counter = 2;
    assert_eq!(
        refused(device.get(&challenge().await)).await,
        StatusCode::UNAUTHORIZED
    );
    device.counter = 1000;
    // An answer that is no answer.
    for broken in [
        json!({}),
        json!({ "ceremony": "x" }),
        json!({ "ceremony": "x", "credential": { "id": "x" } }),
        json!({ "ceremony": "x", "credential": "x".repeat(300_000) }),
    ] {
        let status = refused(broken).await;
        assert!(status.is_client_error(), "{status}");
    }
    // After all of that, the right one still logs in.
    let (status, logged_in, _) = log_in(&state, &mut device).await;
    assert_eq!(status, StatusCode::OK);
    assert!(logged_in.is_some());

    // What a device sends when a passkey is made is checked as well: where
    // it is, and that it asks for the PIN.
    for broken in [
        Device {
            origin: "https://evil.example".into(),
            ..Device::new()
        },
        Device {
            flags: 0x01,
            ..Device::new()
        },
    ] {
        let mut broken = broken;
        let body = json!({ "name": "Phone", "passkey": proof(&state, &cookie, &mut device).await["passkey"] });
        let challenge = start(&state, "/api/passkeys/register", Some(&cookie), body).await;
        let (status, _) = call(
            &state,
            Method::POST,
            "/api/passkeys",
            Some(&cookie),
            Some(broken.create(&challenge)),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    // The same key twice is one passkey: the device is told to leave it out,
    // and if it does not, the server does.
    let body =
        json!({ "name": "Again", "passkey": proof(&state, &cookie, &mut device).await["passkey"] });
    let challenge = start(&state, "/api/passkeys/register", Some(&cookie), body).await;
    let excluded = &challenge["options"]["publicKey"]["excludeCredentials"];
    assert_eq!(excluded[0]["id"], b64(&device.id));
    let mut again = Device::new();
    again.id = device.id.clone();
    let (status, _) = call(
        &state,
        Method::POST,
        "/api/passkeys",
        Some(&cookie),
        Some(again.create(&challenge)),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    // A key of Ada's never becomes one of another account.
    let grace = accounts::create(&db, "grace", Some(PASSWORD), false)
        .await
        .unwrap()
        .unwrap();
    let theirs = common::session(&db, grace).await;
    let challenge = start(
        &state,
        "/api/passkeys/register",
        Some(&theirs),
        json!({ "name": "Phone", "password": PASSWORD }),
    )
    .await;
    let (status, _) = call(
        &state,
        Method::POST,
        "/api/passkeys",
        Some(&theirs),
        Some(again.create(&challenge)),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM passkeys")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[sqlx::test]
async fn wrong_answers_count_like_wrong_passwords(db: PgPool) {
    let ada = ada(&db).await;
    let state = with_passkeys(&db);
    let cookie = common::session(&db, ada).await;
    let mut device = Device::new();
    add(
        &state,
        &cookie,
        &mut device,
        json!({ "password": PASSWORD }),
    )
    .await;
    device.flags = 0x01;
    for _ in 0..login::PER_ADDRESS {
        let (status, _, _) = log_in(&state, &mut device).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    device.flags = 0x05;
    let (status, logged_in, _) = log_in(&state, &mut device).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert!(logged_in.is_none());
}

#[sqlx::test]
async fn ceremonies_have_a_limit(db: PgPool) {
    let state = with_passkeys(&db);
    let mut opened = 0;
    loop {
        let (status, _) = call(&state, Method::POST, "/api/login/passkey", None, None).await;
        if status != StatusCode::OK {
            assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
            break;
        }
        opened += 1;
        assert!(opened <= 20, "a sender has only so many open");
    }
    assert_eq!(state.passkeys.as_ref().unwrap().open_ceremonies(), 20);
}

#[sqlx::test]
async fn an_account_has_at_most_ten(db: PgPool) {
    let ada = ada(&db).await;
    let state = with_passkeys(&db);
    let cookie = common::session(&db, ada).await;
    let mut first = Device::new();
    add(&state, &cookie, &mut first, json!({ "password": PASSWORD })).await;
    for _ in 1..rueder::passkeys::MAX_PER_ACCOUNT {
        let shown = proof(&state, &cookie, &mut first).await;
        add(&state, &cookie, &mut Device::new(), shown).await;
    }
    let shown = proof(&state, &cookie, &mut first).await;
    let mut body = json!({ "name": "One more" });
    body["passkey"] = shown["passkey"].clone();
    let (status, _) = call(
        &state,
        Method::POST,
        "/api/passkeys/register",
        Some(&cookie),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn admins_log_in_with_a_passkey_as_with_a_second_factor(db: PgPool) {
    let admin = accounts::create(&db, "root", Some(PASSWORD), true)
        .await
        .unwrap()
        .unwrap();
    let state = with_passkeys(&db).with_admin_second_factor();
    let cookie = common::session(&db, admin).await;
    let (_, me) = call(&state, Method::GET, "/api/me", Some(&cookie), None).await;
    assert_eq!(me["is_admin"], false);
    let mut device = Device::new();
    add(
        &state,
        &cookie,
        &mut device,
        json!({ "password": PASSWORD }),
    )
    .await;
    let (status, logged_in, me) = log_in(&state, &mut device).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["is_admin"], true);
    let (_, me) = call(&state, Method::GET, "/api/me", logged_in.as_deref(), None).await;
    assert_eq!(
        (
            me["is_admin"].clone(),
            me["admin_needs_second_factor"].clone()
        ),
        (json!(true), json!(false))
    );
}

#[sqlx::test]
async fn a_disabled_account_logs_in_with_no_passkey(db: PgPool) {
    let ada = ada(&db).await;
    let state = with_passkeys(&db);
    let cookie = common::session(&db, ada).await;
    let mut device = Device::new();
    add(
        &state,
        &cookie,
        &mut device,
        json!({ "password": PASSWORD }),
    )
    .await;
    accounts::set_disabled(&db, "ada", true).await.unwrap();
    let (status, logged_in, _) = log_in(&state, &mut device).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(logged_in.is_none());
}

#[sqlx::test]
async fn without_a_password_only_a_young_session_adds_the_first(db: PgPool) {
    let id = accounts::create(&db, "oidc", None, false)
        .await
        .unwrap()
        .unwrap();
    let state = with_passkeys(&db);
    let cookie = common::session(&db, id).await;
    sqlx::query("UPDATE sessions SET created_at = now() - interval '11 minutes'")
        .execute(&db)
        .await
        .unwrap();
    let (status, said) = call(
        &state,
        Method::POST,
        "/api/passkeys/register",
        Some(&cookie),
        Some(json!({ "name": "Phone" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{said}");
    // Logged in again, it can.
    let cookie = common::session(&db, id).await;
    let mut device = Device::new();
    add(&state, &cookie, &mut device, json!({})).await;
    let (status, _, me) = log_in(&state, &mut device).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["has_password"], false);
}

#[sqlx::test]
async fn with_a_passkey_the_app_is_set_up_only_by_the_owner(db: PgPool) {
    let ada = ada(&db).await;
    let state = with_passkeys(&db);
    let cookie = common::session(&db, ada).await;
    let mut device = Device::new();
    let added = add(
        &state,
        &cookie,
        &mut device,
        json!({ "password": PASSWORD }),
    )
    .await;
    // A password that got out with a session sets up no app of its own.
    let setup = |body: Value| {
        let state = state.clone();
        let cookie = cookie.clone();
        async move {
            call(
                &state,
                Method::POST,
                "/api/second-factor/setup",
                Some(&cookie),
                Some(body),
            )
            .await
            .0
        }
    };
    assert_eq!(
        setup(json!({ "password": PASSWORD })).await,
        StatusCode::FORBIDDEN
    );
    // With a recovery code, or a passkey, it can.
    let code = added["recovery_codes"][0].as_str().unwrap();
    assert_eq!(
        setup(json!({ "password": PASSWORD, "code": code })).await,
        StatusCode::OK
    );
    let shown = proof(&state, &cookie, &mut device).await;
    assert_eq!(setup(shown).await, StatusCode::OK);
}

#[sqlx::test]
async fn the_command_line_takes_the_passkeys_too(db: PgPool) {
    let ada = ada(&db).await;
    let state = with_passkeys(&db);
    let cookie = common::session(&db, ada).await;
    let mut device = Device::new();
    add(
        &state,
        &cookie,
        &mut device,
        json!({ "password": PASSWORD }),
    )
    .await;
    assert!(second_factor::switch_off_named(&db, "ADA").await.unwrap());
    let (status, _, _) = log_in(&state, &mut device).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let status = second_factor_of(&state, &cookie).await;
    assert_eq!(
        (status["on"].clone(), status["passkeys"].clone()),
        (json!(false), json!(0))
    );
    // The password alone logs in again.
    let body = json!({ "username": "ada", "password": PASSWORD }).to_string();
    let (status, _, _) = raw(
        &state,
        Method::POST,
        "/api/login",
        None,
        Some(("application/json", body.into_bytes())),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}
