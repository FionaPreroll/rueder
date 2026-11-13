//! Passkeys: logging in without a name or a password, with a key the reader's
//! device keeps and unlocks with its PIN or biometrics (WebAuthn). A passkey is
//! a whole login and a second factor at once: the device stands for what the
//! reader has, its PIN or fingerprint for what they know or are. The checking
//! is `webauthn-rs`'s; this module keeps what goes around it.
//!
//! - **Only with `PUBLIC_URL`.** A passkey belongs to one site, named by the
//!   address browsers reach rueder at. The server never takes it from a
//!   request, so a passkey only ever counts for the site it was made for;
//!   without the setting, there are no passkeys.
//! - **In Android apps** that the site names (`PASSKEY_ANDROID_APPS`, without
//!   it rueder's own): Android gives an app the site's passkeys only where
//!   `/.well-known/assetlinks.json` says so, and the answer names the app's
//!   key as its origin, which counts for exactly these apps (see
//!   [`crate::asset_links`]).
//! - **Adding one** (`POST /api/passkeys/register`, then `POST
//!   /api/passkeys`) takes what changes that need it take: the password and,
//!   where the second factor is on, a code, or a passkey of the account (see
//!   [`second_factor::prove`]). An account without a password and without a
//!   second factor adds its first one only in a session that began within
//!   [`FRESH`], so that a stolen session cannot leave a key of its own behind.
//!   The first passkey turns the second factor on, and if it was off, the
//!   answer has the ten recovery codes, shown this once.
//! - **Logging in** (`POST /api/login/passkey`, then `.../finish`) needs no
//!   name: the device says whose key it is. The session counts as one with a
//!   second factor.
//! - **Showing it is the owner** (`POST /api/passkeys/proof`): a passkey of the
//!   account, just now, stands for password and code wherever a change asks
//!   for them.
//! - **Removing one** (`DELETE /api/passkeys/{id}`) takes the same proof. The
//!   last one goes with the second factor where there is no authenticator app.
//!
//! Between start and finish, the server keeps what it asked the device
//! (challenge, allowed keys) in memory for [`CEREMONY_TIME`], once: a
//! ceremony is taken out when it is finished, whether that worked or not.
//! Only so much is kept, in all and per sender. Wrong answers count like wrong
//! passwords.

use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Mutex, PoisonError},
    time::{Duration, Instant},
};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::{delete, get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use time::OffsetDateTime;
use ts_rs::TS;
use webauthn_rs::prelude::{
    AuthenticationResult, CreationChallengeResponse, DiscoverableAuthentication, Passkey,
    PasskeyAuthentication, PasskeyRegistration, PublicKeyCredential, RegisterPublicKeyCredential,
    RequestChallengeResponse, Url, Uuid, Webauthn, WebauthnBuilder,
};

use crate::{
    AppError, AppState,
    asset_links::AndroidApp,
    login::{self, Key, Sender},
    second_factor::{self, Proof},
    sessions::{self, Session},
    tree,
};

/// How many passkeys an account can have.
pub const MAX_PER_ACCOUNT: i64 = 10;
/// How long a ceremony waits for the device: as long as browsers give the
/// reader by default, to find a key, type a PIN or use a phone.
pub const CEREMONY_TIME: Duration = Duration::from_secs(300);
/// How many ceremonies are kept at most, of all senders.
const MAX_CEREMONIES: usize = 10_000;
/// How many ceremonies one sender can have open at once.
const MAX_PER_SENDER: usize = 20;
/// The longest ceremony name that is looked at; those the server makes are 22
/// characters long.
const MAX_CEREMONY_ID: usize = 64;
/// How young a session has to be for an account without a password and
/// without a second factor to add its first passkey.
pub const FRESH: Duration = Duration::from_secs(10 * 60);

/// What the server knows about passkeys: the site they belong to, and the
/// ceremonies under way.
pub struct Passkeys {
    webauthn: Webauthn,
    /// The Android apps that may use them too, see [`crate::asset_links`].
    android_apps: Vec<AndroidApp>,
    ceremonies: Mutex<HashMap<String, Ceremony>>,
}

/// One ceremony under way, waiting for the device's answer.
struct Ceremony {
    kind: Kind,
    until: Instant,
    /// Who asked for it, for the limit per sender.
    sender: IpAddr,
}

enum Kind {
    /// Adding a passkey to an account, from one of its sessions.
    Register {
        account_id: i32,
        session: Vec<u8>,
        name: String,
        state: PasskeyRegistration,
    },
    /// Logging in, whoever the device says it is.
    Login { state: DiscoverableAuthentication },
    /// Showing, in a session, that the account's owner is there.
    Proof {
        account_id: i32,
        session: Vec<u8>,
        state: PasskeyAuthentication,
    },
}

impl Passkeys {
    /// The passkeys of the site at `public_url`, such as
    /// `https://reader.example`: HTTPS, or HTTP for `localhost` only, which
    /// browsers allow for trying things out. A passkey belongs to the host;
    /// the address has no path.
    pub fn new(public_url: &str) -> anyhow::Result<Self> {
        Self::for_apps(public_url, Vec::new())
    }

    /// The passkeys of the site at `public_url`, which these Android apps may
    /// use as well: an answer that comes from one of them names it as its
    /// origin rather than the site (see [`crate::asset_links`]).
    pub fn for_apps(public_url: &str, android_apps: Vec<AndroidApp>) -> anyhow::Result<Self> {
        let url = Url::parse(public_url.trim())
            .map_err(|err| anyhow::anyhow!("PUBLIC_URL is no address: {err}"))?;
        let Some(host) = url.domain() else {
            anyhow::bail!("PUBLIC_URL needs a host name, not an IP address");
        };
        let local = host == "localhost";
        if url.scheme() != "https" && !(url.scheme() == "http" && local) {
            anyhow::bail!("PUBLIC_URL must be https (http only for localhost)");
        }
        if !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() != "/"
        {
            anyhow::bail!(
                "PUBLIC_URL is only scheme, host and port, such as https://reader.example"
            );
        }
        let host = host.to_owned();
        let mut builder = WebauthnBuilder::new(&host, &url)
            .map_err(|err| anyhow::anyhow!("PUBLIC_URL: {err}"))?;
        // An app's origin is opaque, and webauthn-rs takes it only as it is.
        for app in &android_apps {
            let origin = Url::parse(&app.origin())
                .map_err(|err| anyhow::anyhow!("PASSKEY_ANDROID_APPS: {err}"))?;
            builder = builder.append_allowed_origin(&origin);
        }
        let webauthn = builder
            .rp_name("rueder")
            .timeout(CEREMONY_TIME)
            .build()
            .map_err(|err| anyhow::anyhow!("PUBLIC_URL: {err}"))?;
        Ok(Self {
            webauthn,
            android_apps,
            ceremonies: Mutex::default(),
        })
    }

    /// The Android apps that may use the passkeys.
    pub fn android_apps(&self) -> &[AndroidApp] {
        &self.android_apps
    }

    /// Keeps a ceremony and returns its name, or refuses when too many are
    /// open, in all or of this sender.
    fn open(&self, kind: Kind, sender: IpAddr, now: Instant) -> Result<String, AppError> {
        let sender = login::network(sender);
        let mut ceremonies = self
            .ceremonies
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        ceremonies.retain(|_, ceremony| ceremony.until > now);
        let of_sender = ceremonies
            .values()
            .filter(|ceremony| ceremony.sender == sender)
            .count();
        if ceremonies.len() >= MAX_CEREMONIES || of_sender >= MAX_PER_SENDER {
            let message = "too many passkey ceremonies, try again later";
            return Err(AppError::TooMany(CEREMONY_TIME, message));
        }
        let id = URL_SAFE_NO_PAD.encode(random::<16>()?);
        ceremonies.insert(
            id.clone(),
            Ceremony {
                kind,
                until: now + CEREMONY_TIME,
                sender,
            },
        );
        Ok(id)
    }

    /// Takes a ceremony out, if it is there and has not run out: each counts
    /// once, whatever the answer.
    fn take(&self, id: &str, now: Instant) -> Option<Kind> {
        if id.len() > MAX_CEREMONY_ID {
            return None;
        }
        let mut ceremonies = self
            .ceremonies
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let ceremony = ceremonies.remove(id)?;
        (ceremony.until > now).then_some(ceremony.kind)
    }

    /// How many ceremonies are open, for the tests.
    pub fn open_ceremonies(&self) -> usize {
        self.ceremonies
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }
}

fn random<const N: usize>() -> anyhow::Result<[u8; N]> {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).map_err(|err| anyhow::anyhow!("no random numbers: {err}"))?;
    Ok(bytes)
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/passkeys", get(list).post(finish_registration))
        .route("/passkeys/register", post(start_registration))
        .route("/passkeys/proof", post(start_proof))
        .route("/passkeys/{id}", delete(remove))
}

/// The types of this module for the frontend.
pub(crate) fn types(types: &mut crate::typescript::Types) {
    types
        .add::<List>()
        .add::<Listed>()
        .add::<NewPasskey>()
        .add::<Challenge>()
        .add::<Registered>()
        .add::<Answer>()
        .add::<Registration>()
        .add::<RemovePasskey>();
}

/// A passkey as the list shows it.
#[derive(Serialize, TS)]
#[ts(rename = "Passkey")]
pub struct Listed {
    id: i32,
    name: String,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    created_at: OffsetDateTime,
    /// When it last logged in or showed it was the owner; null until then.
    #[serde(with = "time::serde::rfc3339::option")]
    #[ts(type = "string | null")]
    used_at: Option<OffsetDateTime>,
}

/// What `GET /api/passkeys` answers.
#[derive(Serialize, TS)]
#[ts(rename = "Passkeys")]
struct List {
    passkeys: Vec<Listed>,
    /// How many the account can have.
    max: i64,
}

/// What `POST /api/passkeys/register` reads: a name for the passkey, and what
/// shows that the owner is there (see [`second_factor::prove`]).
#[derive(Deserialize, TS)]
pub struct NewPasskey {
    name: String,
    #[serde(default)]
    #[ts(as = "Option<String>", optional)]
    password: String,
    #[serde(default)]
    #[ts(as = "Option<String>", optional)]
    code: String,
    #[serde(default)]
    #[ts(optional)]
    passkey: Option<Answer>,
}

/// What a ceremony asks the device: the options for
/// `navigator.credentials.create()` or `.get()`, in WebAuthn's JSON, with the
/// name of the ceremony to send the answer back with.
#[derive(Serialize, TS)]
#[ts(rename = "PasskeyChallenge")]
pub struct Challenge {
    ceremony: String,
    #[ts(type = "unknown")]
    options: Value,
}

/// The device's answer to a ceremony that asked for a passkey, which logs in
/// or shows that the owner is there.
#[derive(Deserialize, TS)]
#[ts(rename = "PasskeyAnswer")]
pub struct Answer {
    ceremony: String,
    #[ts(type = "unknown")]
    credential: Box<PublicKeyCredential>,
}

/// The device's answer to a ceremony that made a new passkey.
///
/// `POST /api/passkeys` reads it.
#[derive(Deserialize, TS)]
#[ts(rename = "PasskeyRegistration")]
pub struct Registration {
    ceremony: String,
    #[ts(type = "unknown")]
    credential: Box<RegisterPublicKeyCredential>,
}

/// What `POST /api/passkeys` answers: the new passkey, and the recovery codes
/// where it turned the second factor on.
#[derive(Serialize, TS)]
#[ts(rename = "RegisteredPasskey")]
pub struct Registered {
    passkey: Listed,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    recovery_codes: Option<Vec<String>>,
}

/// What `DELETE /api/passkeys/{id}` reads: what shows that the owner is there.
#[derive(Deserialize, TS)]
pub struct RemovePasskey {
    #[serde(default)]
    #[ts(as = "Option<String>", optional)]
    password: String,
    #[serde(default)]
    #[ts(as = "Option<String>", optional)]
    code: String,
    #[serde(default)]
    #[ts(optional)]
    passkey: Option<Answer>,
}

/// What a request to show the owner is there says, as a [`Proof`].
pub(crate) fn proof<'a>(
    password: &'a str,
    code: &'a str,
    passkey: Option<&'a Answer>,
) -> Proof<'a> {
    match passkey {
        Some(answer) => Proof::Passkey(answer),
        None => Proof::Password { password, code },
    }
}

fn passkeys(state: &AppState) -> Result<&Passkeys, AppError> {
    state.passkeys.as_deref().ok_or(AppError::NotFound)
}

/// The account's passkeys as webauthn-rs reads them, with their ids.
async fn of_account(db: &PgPool, account_id: i32) -> anyhow::Result<Vec<(i32, Passkey)>> {
    let rows = sqlx::query!(
        "SELECT id, credential FROM passkeys WHERE account_id = $1 ORDER BY id",
        account_id
    )
    .fetch_all(db)
    .await?;
    rows.into_iter()
        .map(|row| Ok((row.id, serde_json::from_value(row.credential)?)))
        .collect()
}

/// Whether the account has a passkey.
pub async fn has_any(db: &PgPool, account_id: i32) -> anyhow::Result<bool> {
    Ok(sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM passkeys WHERE account_id = $1) AS "has!""#,
        account_id
    )
    .fetch_one(db)
    .await?)
}

/// Writes down that a passkey was used, with its counter and state as the
/// device reported them.
async fn used(
    db: &PgPool,
    id: i32,
    mut passkey: Passkey,
    result: &AuthenticationResult,
) -> anyhow::Result<()> {
    passkey.update_credential(result);
    sqlx::query!(
        "UPDATE passkeys SET credential = $2, used_at = now() WHERE id = $1",
        id,
        serde_json::to_value(&passkey)?
    )
    .execute(db)
    .await?;
    Ok(())
}

/// The options of a new passkey as browsers read them. webauthn-rs asks for a
/// passkey the device may keep without its name; rueder logs in without a
/// name, so the device has to keep it (a discoverable credential).
fn creation_options(ccr: &CreationChallengeResponse) -> anyhow::Result<Value> {
    let mut options = serde_json::to_value(ccr)?;
    let Some(selection) = options
        .pointer_mut("/publicKey/authenticatorSelection")
        .and_then(Value::as_object_mut)
    else {
        anyhow::bail!("webauthn-rs asked for no authenticator selection");
    };
    selection.insert("residentKey".into(), "required".into());
    selection.insert("requireResidentKey".into(), true.into());
    Ok(options)
}

/// The options of a login or a proof as browsers read them.
fn request_options(mut rcr: RequestChallengeResponse) -> anyhow::Result<Value> {
    // webauthn-rs makes a login without a name one for the browser's own list
    // of logins (conditional mediation); rueder asks when the reader presses
    // the button.
    rcr.mediation = None;
    Ok(serde_json::to_value(&rcr)?)
}

async fn list(State(state): State<AppState>, session: Session) -> Result<Json<List>, AppError> {
    passkeys(&state)?;
    let passkeys = sqlx::query_as!(
        Listed,
        "SELECT id, name, created_at, used_at FROM passkeys WHERE account_id = $1 ORDER BY id",
        session.account_id
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(List {
        passkeys,
        max: MAX_PER_ACCOUNT,
    }))
}

/// Whether the session of these headers began within [`FRESH`].
async fn is_fresh(db: &PgPool, headers: &HeaderMap) -> anyhow::Result<bool> {
    let Some(token) = sessions::token(headers) else {
        return Ok(false);
    };
    Ok(sqlx::query_scalar!(
        r#"SELECT created_at > now() - $2::float8 * interval '1 second' AS "fresh!"
           FROM sessions WHERE token_hash = $1"#,
        sessions::token_hash(token),
        FRESH.as_secs_f64()
    )
    .fetch_optional(db)
    .await?
    .unwrap_or(false))
}

async fn start_registration(
    State(state): State<AppState>,
    session: Session,
    Sender(ip): Sender,
    headers: HeaderMap,
    Json(new): Json<NewPasskey>,
) -> Result<Json<Challenge>, AppError> {
    let keys = passkeys(&state)?;
    let name = tree::title(&new.name)?.to_owned();
    let proof = proof(&new.password, &new.code, new.passkey.as_ref());
    let by_password = matches!(proof, Proof::Password { .. });
    second_factor::prove(&state, &session, &headers, proof).await?;
    // Without a password and a second factor, the session is all there is:
    // only a young one adds a key.
    if by_password {
        let found = sqlx::query!(
            r#"SELECT password_hash IS NULL AS "none!" FROM accounts WHERE id = $1"#,
            session.account_id
        )
        .fetch_one(&state.db)
        .await?;
        if found.none
            && !second_factor::is_on(&state.db, session.account_id).await?
            && !is_fresh(&state.db, &headers).await?
        {
            let message = "log in again first";
            return Err(AppError::Status(StatusCode::FORBIDDEN, message.into()));
        }
    }
    let have = of_account(&state.db, session.account_id).await?;
    if i64::try_from(have.len()).unwrap_or(i64::MAX) >= MAX_PER_ACCOUNT {
        let message = format!("an account has at most {MAX_PER_ACCOUNT} passkeys");
        return Err(AppError::BadRequest(message));
    }
    let user = sqlx::query_scalar!(
        r#"UPDATE accounts SET passkey_user = coalesce(passkey_user, $2)
           WHERE id = $1 RETURNING passkey_user AS "user!""#,
        session.account_id,
        Uuid::new_v4()
    )
    .fetch_one(&state.db)
    .await?;
    let exclude = have
        .iter()
        .map(|(_, passkey)| passkey.cred_id().clone())
        .collect();
    let (ccr, registration) = keys
        .webauthn
        .start_passkey_registration(user, &session.username, &session.username, Some(exclude))
        .map_err(|err| anyhow::anyhow!("starting a passkey: {err}"))?;
    let options = creation_options(&ccr)?;
    let Some(token) = sessions::token(&headers) else {
        return Err(AppError::Status(
            StatusCode::UNAUTHORIZED,
            "not logged in".into(),
        ));
    };
    let ceremony = keys.open(
        Kind::Register {
            account_id: session.account_id,
            session: sessions::token_hash(token),
            name,
            state: registration,
        },
        ip,
        Instant::now(),
    )?;
    Ok(Json(Challenge { ceremony, options }))
}

async fn finish_registration(
    State(state): State<AppState>,
    session: Session,
    headers: HeaderMap,
    Json(registration): Json<Registration>,
) -> Result<(StatusCode, Json<Registered>), AppError> {
    let keys = passkeys(&state)?;
    let token = sessions::token(&headers).map(sessions::token_hash);
    let Some(Kind::Register {
        account_id,
        session: started_in,
        name,
        state: started,
    }) = keys.take(&registration.ceremony, Instant::now())
    else {
        return Err(no_ceremony());
    };
    // Only the session that began it finishes it.
    if account_id != session.account_id || token.as_ref() != Some(&started_in) {
        return Err(no_ceremony());
    }
    let passkey = keys
        .webauthn
        .finish_passkey_registration(&registration.credential, &started)
        .map_err(|err| {
            tracing::debug!("a new passkey was refused: {err}");
            AppError::BadRequest("the passkey could not be checked".into())
        })?;
    let mut tx = state.db.begin().await?;
    tree::lock(&mut tx, session.account_id).await?;
    let before = sqlx::query!(
        r#"SELECT totp_secret IS NOT NULL AS "app!",
                  (SELECT count(*) FROM passkeys p WHERE p.account_id = a.id) AS "passkeys!"
           FROM accounts a WHERE a.id = $1"#,
        session.account_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if before.passkeys >= MAX_PER_ACCOUNT {
        let message = format!("an account has at most {MAX_PER_ACCOUNT} passkeys");
        return Err(AppError::BadRequest(message));
    }
    let listed = sqlx::query_as!(
        Listed,
        "INSERT INTO passkeys (account_id, name, credential_id, credential)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (credential_id) DO NOTHING
         RETURNING id, name, created_at, used_at",
        session.account_id,
        name,
        passkey.cred_id().as_slice(),
        serde_json::to_value(&passkey).map_err(anyhow::Error::from)?
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(listed) = listed else {
        let message = "this passkey is known already";
        return Err(AppError::Status(StatusCode::CONFLICT, message.into()));
    };
    // The first passkey turns the second factor on: without codes, a lost
    // device would lock the reader out.
    let recovery_codes = if before.app || before.passkeys > 0 {
        None
    } else {
        Some(second_factor::new_recovery_codes(&mut tx, session.account_id).await?)
    };
    // The session that showed it is the owner and made the passkey is one with
    // the second factor, as the one that sets up the app is.
    if let Some(token) = token {
        sqlx::query!(
            "UPDATE sessions SET second_factor = true WHERE token_hash = $1",
            token
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(Registered {
            passkey: listed,
            recovery_codes,
        }),
    ))
}

fn no_ceremony() -> AppError {
    let message = "the passkey ceremony is over, start again";
    AppError::Status(StatusCode::CONFLICT, message.into())
}

/// Starts showing, in a session, that the account's owner is there.
async fn start_proof(
    State(state): State<AppState>,
    session: Session,
    Sender(ip): Sender,
    headers: HeaderMap,
) -> Result<Json<Challenge>, AppError> {
    let keys = passkeys(&state)?;
    let have: Vec<Passkey> = of_account(&state.db, session.account_id)
        .await?
        .into_iter()
        .map(|(_, passkey)| passkey)
        .collect();
    if have.is_empty() {
        let message = "the account has no passkey";
        return Err(AppError::Status(StatusCode::CONFLICT, message.into()));
    }
    let (rcr, authentication) = keys
        .webauthn
        .start_passkey_authentication(&have)
        .map_err(|err| anyhow::anyhow!("starting a passkey proof: {err}"))?;
    let Some(token) = sessions::token(&headers) else {
        return Err(AppError::Status(
            StatusCode::UNAUTHORIZED,
            "not logged in".into(),
        ));
    };
    let ceremony = keys.open(
        Kind::Proof {
            account_id: session.account_id,
            session: sessions::token_hash(token),
            state: authentication,
        },
        ip,
        Instant::now(),
    )?;
    let options = request_options(rcr)?;
    Ok(Json(Challenge { ceremony, options }))
}

/// Checks a passkey's answer that the owner of the session's account is
/// there. It counts like a password: a stolen session cannot try without end.
pub(crate) async fn check_proof(
    state: &AppState,
    session: &Session,
    headers: &HeaderMap,
    answer: &Answer,
) -> Result<(), AppError> {
    let keys = passkeys(state)?;
    let names = [Key::name(&session.username)];
    if let Err(wait) = state.logins.attempt(&names, Instant::now()) {
        let message = "too many wrong tries, try again later";
        return Err(AppError::TooMany(wait, message));
    }
    let wrong = || AppError::Status(StatusCode::FORBIDDEN, "the passkey is wrong".into());
    let token = sessions::token(headers).map(sessions::token_hash);
    let Some(Kind::Proof {
        account_id,
        session: started_in,
        state: started,
    }) = keys.take(&answer.ceremony, Instant::now())
    else {
        return Err(wrong());
    };
    if account_id != session.account_id || token.as_ref() != Some(&started_in) {
        return Err(wrong());
    }
    let result = keys
        .webauthn
        .finish_passkey_authentication(&answer.credential, &started)
        .map_err(|err| {
            tracing::debug!("a passkey proof was refused: {err}");
            wrong()
        })?;
    let Some((id, passkey)) = of_account(&state.db, session.account_id)
        .await?
        .into_iter()
        .find(|(_, passkey)| passkey.cred_id() == result.cred_id())
    else {
        // Removed while the device was asked.
        return Err(wrong());
    };
    used(&state.db, id, passkey, &result).await?;
    state.logins.succeeded(&names);
    Ok(())
}

async fn remove(
    State(state): State<AppState>,
    session: Session,
    headers: HeaderMap,
    Path(id): Path<i32>,
    Json(remove): Json<RemovePasskey>,
) -> Result<StatusCode, AppError> {
    passkeys(&state)?;
    let proof = proof(&remove.password, &remove.code, remove.passkey.as_ref());
    second_factor::prove(&state, &session, &headers, proof).await?;
    let mut tx = state.db.begin().await?;
    tree::lock(&mut tx, session.account_id).await?;
    let gone = sqlx::query!(
        "DELETE FROM passkeys WHERE id = $1 AND account_id = $2",
        id,
        session.account_id
    )
    .execute(&mut *tx)
    .await?;
    if gone.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    second_factor::off_if_nothing_left(&mut tx, session.account_id).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Starts a login with a passkey, for anybody: the device says whose it is.
pub async fn start_login(
    State(state): State<AppState>,
    Sender(ip): Sender,
) -> Result<Json<Challenge>, AppError> {
    let keys = passkeys(&state)?;
    let (rcr, authentication) = keys
        .webauthn
        .start_discoverable_authentication()
        .map_err(|err| anyhow::anyhow!("starting a passkey login: {err}"))?;
    let ceremony = keys.open(
        Kind::Login {
            state: authentication,
        },
        ip,
        Instant::now(),
    )?;
    let options = request_options(rcr)?;
    Ok(Json(Challenge { ceremony, options }))
}

/// Finishes a login with a passkey: the answer of `POST /api/login`, with the
/// session's cookie, and the session counts as one with a second factor.
pub async fn finish_login(
    State(state): State<AppState>,
    Sender(ip): Sender,
    headers: HeaderMap,
    Json(answer): Json<Answer>,
) -> Result<Response, AppError> {
    let keys = passkeys(&state)?;
    // Every try counts as wrong until it turns out right, for the sender and
    // its network; whose key it is, nobody knows yet.
    let senders = [Key::address(ip), Key::network(ip)];
    if let Err(wait) = state.logins.attempt(&senders, Instant::now()) {
        let message = "too many wrong tries, try again later";
        return Err(AppError::TooMany(wait, message));
    }
    let wrong = || AppError::Status(StatusCode::UNAUTHORIZED, "the passkey is wrong".into());
    let Some(Kind::Login { state: started }) = keys.take(&answer.ceremony, Instant::now()) else {
        return Err(no_ceremony());
    };
    let Ok((user, credential_id)) = keys
        .webauthn
        .identify_discoverable_authentication(&answer.credential)
    else {
        return Err(wrong());
    };
    let found = sqlx::query!(
        r#"SELECT p.id, p.credential, a.id AS account_id, a.disabled_at IS NOT NULL AS "disabled!"
           FROM passkeys p JOIN accounts a ON a.id = p.account_id
           WHERE p.credential_id = $1 AND a.passkey_user = $2"#,
        credential_id,
        user
    )
    .fetch_optional(&state.db)
    .await?;
    let Some(found) = found else {
        return Err(wrong());
    };
    let passkey: Passkey = serde_json::from_value(found.credential).map_err(anyhow::Error::from)?;
    let result = keys
        .webauthn
        .finish_discoverable_authentication_v2(&answer.credential, started, &passkey)
        .map_err(|err| {
            tracing::debug!("a passkey login was refused: {err}");
            wrong()
        })?;
    state.logins.succeeded(&senders);
    used(&state.db, found.id, passkey, &result).await?;
    // Said only to whoever has the passkey.
    if found.disabled {
        return Err(AppError::Status(StatusCode::FORBIDDEN, "disabled".into()));
    }
    login::start_session(&state, &headers, found.account_id, true).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_site_is_https_or_localhost_and_nothing_else() {
        for good in [
            "https://reader.example",
            "https://reader.example/",
            "https://reader.example:8443",
            "http://localhost:3000",
        ] {
            assert!(Passkeys::new(good).is_ok(), "{good}");
        }
        for bad in [
            "",
            "reader.example",
            "http://reader.example",
            "https://192.0.2.7",
            "http://127.0.0.1:3000",
            "https://reader.example/rueder",
            "https://reader.example/?a=b",
            "https://reader.example/#top",
            "https://user:pass@reader.example",
            "ftp://reader.example",
        ] {
            assert!(Passkeys::new(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn ceremonies_count_once_run_out_and_have_limits() {
        let passkeys = Passkeys::new("https://reader.example").unwrap();
        let ip: IpAddr = "192.0.2.7".parse().unwrap();
        let now = Instant::now();
        let login = || {
            let (_, state) = passkeys
                .webauthn
                .start_discoverable_authentication()
                .unwrap();
            Kind::Login { state }
        };
        let id = passkeys.open(login(), ip, now).ok().unwrap();
        assert_eq!(id.len(), 22);
        assert!(passkeys.take(&id, now).is_some());
        assert!(passkeys.take(&id, now).is_none(), "once");
        let id = passkeys.open(login(), ip, now).ok().unwrap();
        assert!(passkeys.take(&id, now + CEREMONY_TIME).is_none(), "ran out");
        assert!(passkeys.take(&"x".repeat(1000), now).is_none());

        // One sender has only so many open; another still can, and those that
        // ran out make room.
        for _ in 0..MAX_PER_SENDER {
            passkeys.open(login(), ip, now).ok().unwrap();
        }
        assert!(passkeys.open(login(), ip, now).is_err());
        // The same /64 counts as the same sender.
        let neighbour: IpAddr = "2001:db8::1".parse().unwrap();
        for _ in 0..MAX_PER_SENDER {
            passkeys.open(login(), neighbour, now).ok().unwrap();
        }
        assert!(
            passkeys
                .open(login(), "2001:db8::2".parse().unwrap(), now)
                .is_err()
        );
        assert!(
            passkeys
                .open(login(), "192.0.2.8".parse().unwrap(), now)
                .is_ok()
        );
        assert!(passkeys.open(login(), ip, now + CEREMONY_TIME).is_ok());
    }

    #[test]
    fn a_new_passkey_is_one_the_device_keeps_with_its_owner() {
        let passkeys = Passkeys::new("https://reader.example").unwrap();
        let (ccr, _) = passkeys
            .webauthn
            .start_passkey_registration(Uuid::new_v4(), "ada", "ada", None)
            .unwrap();
        let options = creation_options(&ccr).unwrap();
        let selection = &options["publicKey"]["authenticatorSelection"];
        assert_eq!(selection["residentKey"], "required");
        assert_eq!(selection["requireResidentKey"], true);
        assert_eq!(selection["userVerification"], "required");
        assert_eq!(options["publicKey"]["rp"]["id"], "reader.example");
        assert_eq!(options["publicKey"]["attestation"], "none");

        let (rcr, _) = passkeys
            .webauthn
            .start_discoverable_authentication()
            .unwrap();
        let options = request_options(rcr).unwrap();
        assert!(options.get("mediation").is_none());
        assert_eq!(options["publicKey"]["userVerification"], "required");
        assert_eq!(options["publicKey"]["rpId"], "reader.example");
    }
}
