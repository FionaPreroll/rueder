//! The second factor: codes of an authenticator app after RFC 6238 (TOTP), and
//! ten recovery codes for when the app is gone. Passkeys count as a second
//! factor too (see [`crate::passkeys`]): an account has it on when it has the
//! app or a passkey, and then a password alone opens nothing.
//!
//! - **Setting it up** takes the password (`POST /api/second-factor/setup`),
//!   and where passkeys have the second factor on already, a recovery code
//!   or a passkey,
//!   which answers with a new secret, as text and as a QR code, and keeps it
//!   aside. A code from the app confirms it (`POST /api/second-factor/confirm`);
//!   only then does it count, and the answer has the recovery codes, shown
//!   this once. The session that confirmed counts as one with the second
//!   factor from then on.
//! - **Logging in** takes the code with name and password (`login::login`).
//!   Without one the answer says that one is needed; a wrong one counts like
//!   a wrong password. A code counts once: the time step of the last one taken
//!   is kept. A recovery code goes once it was used.
//! - **Switching the app off** takes the password and a code, from the app or
//!   a recovery code, or a passkey (`DELETE /api/second-factor`), so that a
//!   password that got out together with a session is not enough. The
//!   recovery codes stay while there are passkeys. An admin on the command line
//!   (`rueder account second-factor-off`) takes all of it, passkeys included,
//!   for somebody who lost everything.
//! - **Admins** need it where the operator says so (`ADMIN_SECOND_FACTOR=required`):
//!   then the right of an admin counts only in a session that began with it.
//!
//! Recovery codes are kept only as their SHA-256; the secret as it is, since
//! every code is worked out from it.

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool};
use ts_rs::TS;

use crate::{
    AppError, AppState, accounts,
    login::{CODE_NEEDED, Key},
    passkeys,
    sessions::{self, Session},
};

/// How many digits a code has.
pub const DIGITS: u32 = 6;
/// How long a code counts, in seconds.
pub const STEP: u64 = 30;
/// How many steps before and after the present one still count, for clocks
/// that are a little off.
const DRIFT: i64 = 1;
/// The length of a secret in bytes: 160 bits, as RFC 4226 recommends.
const SECRET_LEN: usize = 20;
/// How many recovery codes an account gets.
pub const RECOVERY_CODES: usize = 10;
/// The letters a recovery code is made of: those of base32 without 0, 1, 8
/// and the letters that look like them, so it can be read off paper.
const CODE_LETTERS: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ234567";
/// How many letters a recovery code has, as two groups of five.
const CODE_LEN: usize = 10;
/// The longest code that is looked at: anything longer is wrong anyway.
const MAX_CODE: usize = 32;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/second-factor", get(status).delete(switch_off))
        .route("/second-factor/setup", post(set_up))
        .route("/second-factor/confirm", post(confirm))
}

/// The types this module sends and reads, for the frontend.
pub(crate) fn types(types: &mut crate::typescript::Types) {
    types
        .add::<SecondFactor>()
        .add::<Password>()
        .add::<SwitchOff>()
        .add::<Setup>()
        .add::<Code>()
        .add::<RecoveryCodes>();
}

/// Whether an account has the second factor, by the app or by passkeys, and
/// how many recovery codes it has left.
///
/// `GET /api/second-factor` answers with it.
#[derive(Serialize, TS)]
pub struct SecondFactor {
    /// Whether the second factor is on: the app or a passkey.
    on: bool,
    /// Whether the authenticator app is set up.
    app: bool,
    /// How many passkeys the account has.
    passkeys: i64,
    recovery_codes_left: i64,
    /// Whether this server lets admins be admins only with it.
    required_for_admins: bool,
}

/// What setting up reads: the password of the account, which an account that
/// has none does without, and where the second factor is on already through
/// passkeys, a recovery code; or a passkey instead of both. Otherwise a
/// password that got out with a session could add an app of its own.
#[derive(Deserialize, TS)]
#[ts(rename = "SecondFactorPassword")]
pub struct Password {
    #[serde(default)]
    #[ts(as = "Option<String>", optional)]
    password: String,
    #[serde(default)]
    #[ts(as = "Option<String>", optional)]
    code: String,
    #[serde(default)]
    #[ts(optional)]
    passkey: Option<passkeys::Answer>,
}

/// What switching off reads: the password, as for setting up, and a code from
/// the app or a recovery code; or a passkey instead of both.
#[derive(Deserialize, TS)]
#[ts(rename = "SecondFactorSwitchOff")]
pub struct SwitchOff {
    #[serde(default)]
    #[ts(as = "Option<String>", optional)]
    password: String,
    #[serde(default)]
    #[ts(as = "Option<String>", optional)]
    code: String,
    #[serde(default)]
    #[ts(optional)]
    passkey: Option<passkeys::Answer>,
}

/// A new secret for the app, waiting for a code to confirm it.
///
/// `POST /api/second-factor/setup` answers with it.
#[derive(Serialize, TS)]
#[ts(rename = "SecondFactorSetup")]
pub struct Setup {
    /// The secret in base32, to type into an app by hand.
    secret: String,
    /// The `otpauth://` address apps read from the QR code.
    uri: String,
    /// The QR code of `uri`, as SVG.
    qr_svg: String,
}

/// A code from the app.
///
/// `POST /api/second-factor/confirm` reads it.
#[derive(Deserialize, TS)]
#[ts(rename = "SecondFactorCode")]
pub struct Code {
    code: String,
}

/// The recovery codes, shown once.
///
/// `POST /api/second-factor/confirm` answers with them.
#[derive(Serialize, TS)]
pub struct RecoveryCodes {
    codes: Vec<String>,
}

/// The code of a secret for a time step, after RFC 4226 (HOTP) with SHA-1.
pub fn code_at(secret: &[u8], step: u64) -> u32 {
    let Ok(mut mac) = <Hmac<Sha1> as KeyInit>::new_from_slice(secret) else {
        // HMAC takes a key of any length; this does not happen.
        return u32::MAX;
    };
    mac.update(&step.to_be_bytes());
    let hash = mac.finalize().into_bytes();
    let offset = usize::from(hash[hash.len() - 1] & 0x0f);
    let number = u32::from_be_bytes([
        hash[offset] & 0x7f,
        hash[offset + 1],
        hash[offset + 2],
        hash[offset + 3],
    ]);
    number % 10u32.pow(DIGITS)
}

/// The time step of a moment, in seconds since 1970.
pub fn step_of(unix: u64) -> u64 {
    unix / STEP
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// The step a code of six digits belongs to, if it is one of the steps around
/// `now` that are later than `after`.
pub fn matching_step(secret: &[u8], code: &str, now: u64, after: Option<i64>) -> Option<i64> {
    let code = code.trim();
    if code.len() != DIGITS as usize || !code.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let wanted: u32 = code.parse().ok()?;
    let now = i64::try_from(step_of(now)).ok()?;
    (now - DRIFT..=now + DRIFT)
        .filter(|step| after.is_none_or(|after| *step > after))
        .find(|step| u64::try_from(*step).is_ok_and(|step| code_at(secret, step) == wanted))
}

/// Base32 after RFC 4648, without padding, as apps read secrets.
pub fn base32(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut out = String::with_capacity(bytes.len().div_ceil(5) * 8);
    let (mut buffer, mut bits) = (0u32, 0u32);
    for &byte in bytes {
        buffer = (buffer << 8) | u32::from(byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(char::from(ALPHABET[((buffer >> bits) & 31) as usize]));
        }
    }
    if bits > 0 {
        out.push(char::from(ALPHABET[((buffer << (5 - bits)) & 31) as usize]));
    }
    out
}

/// A recovery code as it is compared: without dashes and spaces, in capitals.
fn normal(code: &str) -> String {
    code.chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .flat_map(char::to_uppercase)
        .collect()
}

fn code_hash(code: &str) -> Vec<u8> {
    Sha256::digest(normal(code).as_bytes()).to_vec()
}

fn random<const N: usize>() -> anyhow::Result<[u8; N]> {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).map_err(|err| anyhow::anyhow!("no random numbers: {err}"))?;
    Ok(bytes)
}

/// Ten new recovery codes, like `K7QPM-2XRTA`.
fn new_codes() -> anyhow::Result<Vec<String>> {
    (0..RECOVERY_CODES)
        .map(|_| {
            let bytes = random::<CODE_LEN>()?;
            // 256 is no multiple of the letters, so the first ones come a
            // little more often; with ten letters of 29 that costs nothing.
            let letters: String = bytes
                .iter()
                .map(|byte| char::from(CODE_LETTERS[usize::from(*byte) % CODE_LETTERS.len()]))
                .collect();
            Ok(format!("{}-{}", &letters[..5], &letters[5..]))
        })
        .collect()
}

/// The QR code of an address, as SVG.
fn qr_svg(uri: &str) -> anyhow::Result<String> {
    let code = qrcode::QrCode::new(uri.as_bytes())?;
    Ok(code
        .render::<qrcode::render::svg::Color<'_>>()
        .min_dimensions(200, 200)
        .quiet_zone(true)
        .build())
}

/// Whether the account has the second factor on: the app or a passkey.
pub async fn is_on(db: &PgPool, account_id: i32) -> anyhow::Result<bool> {
    Ok(sqlx::query_scalar!(
        r#"SELECT totp_secret IS NOT NULL
                  OR EXISTS (SELECT 1 FROM passkeys p WHERE p.account_id = a.id) AS "on!"
           FROM accounts a WHERE a.id = $1"#,
        account_id
    )
    .fetch_optional(db)
    .await?
    .unwrap_or(false))
}

/// What shows, for a change that needs it, that the owner of the account is
/// at the keyboard and not only somebody with the session.
pub(crate) enum Proof<'a> {
    /// The password, which an account without one does without, and where
    /// the second factor is on, a code from the app or a recovery code.
    Password { password: &'a str, code: &'a str },
    /// A passkey of the account, just now: both factors at once.
    Passkey(&'a passkeys::Answer),
}

/// Checks that the owner of the session's account is there. Every try counts
/// like one at login: a stolen session must not be a way to try passwords or
/// codes without end. Without a code where one is needed, the answer is 403
/// with `second factor needed`.
pub(crate) async fn prove(
    state: &AppState,
    session: &Session,
    headers: &HeaderMap,
    proof: Proof<'_>,
) -> Result<(), AppError> {
    let (password, code) = match proof {
        Proof::Passkey(answer) => {
            return passkeys::check_proof(state, session, headers, answer).await;
        }
        Proof::Password { password, code } => (password, code),
    };
    let keys = [Key::name(&session.username)];
    tried_password(state, session, &keys, password).await?;
    if is_on(&state.db, session.account_id).await? {
        let code = code.trim();
        if code.is_empty() {
            // The password was right; the code is what is missing.
            state.logins.succeeded(&keys);
            return Err(AppError::Status(StatusCode::FORBIDDEN, CODE_NEEDED.into()));
        }
        // A wrong code stays counted, like a wrong password.
        if !take(&state.db, session.account_id, code).await? {
            let message = "the code is wrong";
            return Err(AppError::Status(StatusCode::FORBIDDEN, message.into()));
        }
    }
    state.logins.succeeded(&keys);
    Ok(())
}

/// Gives the account ten new recovery codes in place of any it had, and
/// returns them to be shown this once.
pub(crate) async fn new_recovery_codes(
    conn: &mut PgConnection,
    account_id: i32,
) -> anyhow::Result<Vec<String>> {
    let codes = new_codes()?;
    sqlx::query!(
        "DELETE FROM recovery_codes WHERE account_id = $1",
        account_id
    )
    .execute(&mut *conn)
    .await?;
    let hashes: Vec<Vec<u8>> = codes.iter().map(|code| code_hash(code)).collect();
    sqlx::query!(
        "INSERT INTO recovery_codes (account_id, code_hash) SELECT $1, unnest($2::bytea[])",
        account_id,
        &hashes
    )
    .execute(&mut *conn)
    .await?;
    Ok(codes)
}

/// Where the account has neither the app nor a passkey left, the second
/// factor is off: its recovery codes go, and its sessions no longer count as
/// ones with it.
pub(crate) async fn off_if_nothing_left(
    conn: &mut PgConnection,
    account_id: i32,
) -> anyhow::Result<()> {
    let left = sqlx::query_scalar!(
        r#"SELECT totp_secret IS NOT NULL
                  OR EXISTS (SELECT 1 FROM passkeys p WHERE p.account_id = a.id) AS "left!"
           FROM accounts a WHERE a.id = $1"#,
        account_id
    )
    .fetch_optional(&mut *conn)
    .await?
    .unwrap_or(false);
    if left {
        return Ok(());
    }
    sqlx::query!(
        "DELETE FROM recovery_codes WHERE account_id = $1",
        account_id
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "UPDATE sessions SET second_factor = false WHERE account_id = $1",
        account_id
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Takes a code at login, from the app or a recovery code. A code from the app
/// counts once; a recovery code goes once it was used. An account with
/// passkeys and no app has only recovery codes.
pub async fn take(db: &PgPool, account_id: i32, code: &str) -> anyhow::Result<bool> {
    if code.len() > MAX_CODE {
        return Ok(false);
    }
    let mut tx = db.begin().await?;
    let found = sqlx::query!(
        "SELECT totp_secret, totp_last_step FROM accounts WHERE id = $1 FOR UPDATE",
        account_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(found) = found else {
        return Ok(false);
    };
    // A code of the app, where there is one; a recovery code also where the
    // second factor is passkeys alone.
    let step = found
        .totp_secret
        .and_then(|secret| matching_step(&secret, code, now_unix(), found.totp_last_step));
    if let Some(step) = step {
        sqlx::query!(
            "UPDATE accounts SET totp_last_step = $2 WHERE id = $1",
            account_id,
            step
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        return Ok(true);
    }
    let used = sqlx::query!(
        "DELETE FROM recovery_codes WHERE account_id = $1 AND code_hash = $2",
        account_id,
        code_hash(code)
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(used.rows_affected() == 1)
}

/// Switches the authenticator app off for an account. Where it has no passkey
/// either, the second factor is off with it (see [`off_if_nothing_left`]).
/// False if there is no such account.
pub async fn switch_off_app(conn: &mut PgConnection, account_id: i32) -> anyhow::Result<bool> {
    let done = sqlx::query!(
        "UPDATE accounts SET totp_secret = NULL, totp_pending = NULL, totp_last_step = NULL
         WHERE id = $1",
        account_id
    )
    .execute(&mut *conn)
    .await?;
    off_if_nothing_left(conn, account_id).await?;
    Ok(done.rows_affected() == 1)
}

/// Switches the whole second factor off for an account: the app, its
/// passkeys and its recovery codes, and takes it from its sessions. False if
/// there is no such account.
pub async fn switch_off_for(conn: &mut PgConnection, account_id: i32) -> anyhow::Result<bool> {
    sqlx::query!("DELETE FROM passkeys WHERE account_id = $1", account_id)
        .execute(&mut *conn)
        .await?;
    switch_off_app(conn, account_id).await
}

/// For the command line: switches all of it off for the account of this
/// name, passkeys included.
pub async fn switch_off_named(db: &PgPool, name: &str) -> anyhow::Result<bool> {
    let mut tx = db.begin().await?;
    let id = sqlx::query_scalar!(
        "SELECT id FROM accounts WHERE lower(username) = lower($1)",
        name.trim()
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(id) = id else {
        return Ok(false);
    };
    switch_off_for(&mut tx, id).await?;
    tx.commit().await?;
    Ok(true)
}

async fn status(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<SecondFactor>, AppError> {
    let found = sqlx::query!(
        r#"SELECT a.totp_secret IS NOT NULL AS "app!",
                  (SELECT count(*) FROM passkeys p WHERE p.account_id = a.id) AS "passkeys!",
                  (SELECT count(*) FROM recovery_codes r WHERE r.account_id = a.id) AS "left!"
           FROM accounts a WHERE a.id = $1"#,
        session.account_id
    )
    .fetch_one(&state.db)
    .await?;
    Ok(Json(SecondFactor {
        on: found.app || found.passkeys > 0,
        app: found.app,
        passkeys: found.passkeys,
        recovery_codes_left: found.left,
        required_for_admins: state.admin_second_factor,
    }))
}

/// Counts a try against the limit of the account's name and checks the
/// password; the caller says when the whole of it succeeded.
pub(crate) async fn tried_password(
    state: &AppState,
    session: &Session,
    keys: &[Key],
    password: &str,
) -> Result<(), AppError> {
    if let Err(wait) = state.logins.attempt(keys, Instant::now()) {
        let message = "too many wrong tries, try again later";
        return Err(AppError::TooMany(wait, message));
    }
    let hash = sqlx::query_scalar!(
        "SELECT password_hash FROM accounts WHERE id = $1",
        session.account_id
    )
    .fetch_one(&state.db)
    .await?;
    if hash.is_some() && !accounts::verify(password, hash).await? {
        let message = "the password is wrong";
        return Err(AppError::Status(StatusCode::FORBIDDEN, message.into()));
    }
    Ok(())
}

async fn set_up(
    State(state): State<AppState>,
    session: Session,
    headers: HeaderMap,
    Json(password): Json<Password>,
) -> Result<Json<Setup>, AppError> {
    let proof = passkeys::proof(
        &password.password,
        &password.code,
        password.passkey.as_ref(),
    );
    prove(&state, &session, &headers, proof).await?;
    let secret = random::<SECRET_LEN>()?;
    sqlx::query!(
        "UPDATE accounts SET totp_pending = $2 WHERE id = $1",
        session.account_id,
        &secret[..]
    )
    .execute(&state.db)
    .await?;
    let secret = base32(&secret);
    let label: String =
        url::form_urlencoded::byte_serialize(format!("rueder:{}", session.username).as_bytes())
            .collect();
    let uri = format!(
        "otpauth://totp/{label}?secret={secret}&issuer=rueder&algorithm=SHA1&digits={DIGITS}&period={STEP}"
    );
    let qr_svg = qr_svg(&uri)?;
    Ok(Json(Setup {
        secret,
        uri,
        qr_svg,
    }))
}

async fn confirm(
    State(state): State<AppState>,
    session: Session,
    headers: HeaderMap,
    Json(code): Json<Code>,
) -> Result<Json<RecoveryCodes>, AppError> {
    // A wrong code counts like a wrong password.
    let keys = [Key::name(&session.username)];
    if let Err(wait) = state.logins.attempt(&keys, Instant::now()) {
        let message = "too many wrong tries, try again later";
        return Err(AppError::TooMany(wait, message));
    }
    let mut tx = state.db.begin().await?;
    let pending = sqlx::query_scalar!(
        "SELECT totp_pending FROM accounts WHERE id = $1 FOR UPDATE",
        session.account_id
    )
    .fetch_one(&mut *tx)
    .await?;
    let Some(secret) = pending else {
        let message = "set it up first";
        return Err(AppError::Status(StatusCode::CONFLICT, message.into()));
    };
    let Some(step) = matching_step(&secret, &code.code, now_unix(), None) else {
        let message = "the code is wrong";
        return Err(AppError::Status(StatusCode::FORBIDDEN, message.into()));
    };
    state.logins.succeeded(&keys);
    sqlx::query!(
        "UPDATE accounts
         SET totp_secret = totp_pending, totp_pending = NULL, totp_last_step = $2
         WHERE id = $1",
        session.account_id,
        step
    )
    .execute(&mut *tx)
    .await?;
    let codes = new_recovery_codes(&mut tx, session.account_id).await?;
    // The session that showed a code is one with the second factor.
    if let Some(token) = sessions::token(&headers) {
        sqlx::query!(
            "UPDATE sessions SET second_factor = true WHERE token_hash = $1",
            sessions::token_hash(token)
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(Json(RecoveryCodes { codes }))
}

async fn switch_off(
    State(state): State<AppState>,
    session: Session,
    headers: HeaderMap,
    Json(off): Json<SwitchOff>,
) -> Result<StatusCode, AppError> {
    // With the password, a code of the second factor itself, or a passkey;
    // a wrong or missing code stays counted, like a wrong password.
    let proof = passkeys::proof(&off.password, &off.code, off.passkey.as_ref());
    match prove(&state, &session, &headers, proof).await {
        Err(AppError::Status(StatusCode::FORBIDDEN, message)) if message == CODE_NEEDED => {
            let message = "the code is wrong";
            return Err(AppError::Status(StatusCode::FORBIDDEN, message.into()));
        }
        other => other?,
    }
    let mut tx = state.db.begin().await?;
    switch_off_app(&mut tx, session.account_id).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_those_of_rfc_6238() {
        // The SHA-1 cases of RFC 6238, appendix B, cut to six digits.
        let secret = b"12345678901234567890";
        for (unix, code) in [
            (59, 287_082),
            (1_111_111_109, 81_804),
            (1_111_111_111, 50_471),
            (1_234_567_890, 5_924),
            (2_000_000_000, 279_037),
            (20_000_000_000, 353_130),
        ] {
            assert_eq!(code_at(secret, step_of(unix)), code, "{unix}");
        }
    }

    #[test]
    fn a_code_counts_within_a_step_either_way_and_once() {
        let secret = b"12345678901234567890";
        let now = 1_111_111_111;
        let step = i64::try_from(step_of(now)).unwrap();
        assert_eq!(matching_step(secret, "050471", now, None), Some(step));
        assert_eq!(matching_step(secret, " 050471 ", now, None), Some(step));
        // A step before and after count, two do not.
        let before = format!("{:06}", code_at(secret, step_of(now) - 1));
        let long_before = format!("{:06}", code_at(secret, step_of(now) - 2));
        assert_eq!(matching_step(secret, &before, now, None), Some(step - 1));
        assert_eq!(matching_step(secret, &long_before, now, None), None);
        // Not again, nor one older than the last taken.
        assert_eq!(matching_step(secret, "050471", now, Some(step)), None);
        assert_eq!(matching_step(secret, &before, now, Some(step - 1)), None);
        for wrong in ["", "50471", "0504711", "05047a", "０５０４７１", "-50471"] {
            assert_eq!(matching_step(secret, wrong, now, None), None, "{wrong}");
        }
    }

    #[test]
    fn base32_is_that_of_rfc_4648() {
        for (bytes, text) in [
            (&b""[..], ""),
            (b"f", "MY"),
            (b"fo", "MZXQ"),
            (b"foo", "MZXW6"),
            (b"foob", "MZXW6YQ"),
            (b"fooba", "MZXW6YTB"),
            (b"foobar", "MZXW6YTBOI"),
        ] {
            assert_eq!(base32(bytes), text);
        }
        assert_eq!(base32(&[0u8; SECRET_LEN]).len(), 32);
    }

    #[test]
    fn recovery_codes_are_read_as_written_and_differ() {
        let codes = new_codes().unwrap();
        assert_eq!(codes.len(), RECOVERY_CODES);
        for code in &codes {
            assert_eq!(code.len(), CODE_LEN + 1);
            assert_eq!(&code[5..6], "-");
            assert_eq!(
                code_hash(code),
                code_hash(&code.to_lowercase().replace('-', " "))
            );
        }
        let mut unique = codes.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), RECOVERY_CODES);
    }
}
