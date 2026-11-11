//! Sessions: who a request comes from.
//!
//! A login gives the browser a random token in a cookie that scripts cannot
//! read. The database keeps only the token's SHA-256, so whoever reads the
//! database cannot log in with what is there. A session ends when it is not
//! used for [`IDLE`], on logout, and when the account gets a new password.
//!
//! A handler that takes a [`Session`] answers only to logged-in readers, and
//! has the account's id to bind its queries to.

use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, StatusCode, header, request::Parts},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use crate::{AppError, AppState};

/// The cookie's name. The prefix makes browsers take it only over HTTPS, only
/// from this host and only for all of its paths.
pub const COOKIE: &str = "__Host-rueder";
/// How long a session lasts without being used, in seconds.
pub const IDLE: i64 = 30 * 24 * 60 * 60;
/// How long the browser keeps the cookie. The server decides when a session
/// is over; this only has to be longer.
const COOKIE_AGE: i64 = 400 * 24 * 60 * 60;
/// How much later a session is used again before that is written down, in
/// seconds: not every request needs a write.
const TOUCH_AFTER: f64 = 3600.0;
/// How many sessions an account can have at once: browsers and devices that
/// are logged in.
pub const MAX_PER_ACCOUNT: i64 = 100;
/// 32 random bytes in base64 without padding.
const TOKEN_LEN: usize = 43;

/// The logged-in reader a request comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub account_id: i32,
    pub username: String,
    pub is_admin: bool,
}

pub(crate) fn token_hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

/// Starts a session for the account and returns the token for the cookie.
pub async fn start(db: &PgPool, account_id: i32) -> anyhow::Result<String> {
    let mut random = [0u8; 32];
    getrandom::fill(&mut random).map_err(|err| anyhow::anyhow!("no random numbers: {err}"))?;
    let token = URL_SAFE_NO_PAD.encode(random);
    sqlx::query!(
        "INSERT INTO sessions (token_hash, account_id, expires_at)
         VALUES ($1, $2, now() + $3::float8 * interval '1 second')",
        token_hash(&token),
        account_id,
        IDLE as f64,
    )
    .execute(db)
    .await?;
    // Beyond what an account can have, the ones used longest ago end.
    sqlx::query!(
        "DELETE FROM sessions WHERE token_hash IN (
             SELECT token_hash FROM sessions WHERE account_id = $1
             ORDER BY last_seen_at DESC, created_at DESC OFFSET $2)",
        account_id,
        MAX_PER_ACCOUNT,
    )
    .execute(db)
    .await?;
    Ok(token)
}

/// The session a token stands for, if it is one and has not run out. Using a
/// session keeps it alive.
pub async fn find(db: &PgPool, token: &str) -> anyhow::Result<Option<Session>> {
    if token.len() != TOKEN_LEN {
        return Ok(None);
    }
    let hash = token_hash(token);
    let found = sqlx::query!(
        r#"SELECT a.id, a.username, a.is_admin,
                  s.last_seen_at < now() - $2::float8 * interval '1 second' AS "stale!"
           FROM sessions s JOIN accounts a ON a.id = s.account_id
           WHERE s.token_hash = $1 AND s.expires_at > now()"#,
        hash,
        TOUCH_AFTER,
    )
    .fetch_optional(db)
    .await?;
    let Some(found) = found else {
        return Ok(None);
    };
    if found.stale {
        sqlx::query!(
            "UPDATE sessions
             SET last_seen_at = now(), expires_at = now() + $2::float8 * interval '1 second'
             WHERE token_hash = $1",
            hash,
            IDLE as f64,
        )
        .execute(db)
        .await?;
    }
    Ok(Some(Session {
        account_id: found.id,
        username: found.username,
        is_admin: found.is_admin,
    }))
}

/// Ends the session of this token.
pub async fn end(db: &PgPool, token: &str) -> anyhow::Result<()> {
    sqlx::query!(
        "DELETE FROM sessions WHERE token_hash = $1",
        token_hash(token)
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Removes the sessions that have run out and returns how many that were.
pub async fn sweep(db: &PgPool) -> anyhow::Result<u64> {
    let gone = sqlx::query!("DELETE FROM sessions WHERE expires_at <= now()")
        .execute(db)
        .await?;
    Ok(gone.rows_affected())
}

/// The `Set-Cookie` value that gives the browser the token.
pub fn cookie(token: &str) -> String {
    format!("{COOKIE}={token}; Path=/; Max-Age={COOKIE_AGE}; HttpOnly; Secure; SameSite=Lax")
}

/// The `Set-Cookie` value that takes the cookie away again.
pub fn no_cookie() -> String {
    format!("{COOKIE}=; Path=/; Max-Age=0; HttpOnly; Secure; SameSite=Lax")
}

/// The token a request brings along, if any.
pub fn token(headers: &HeaderMap) -> Option<&str> {
    cookie_value(headers, COOKIE)
}

/// The value of the cookie of this name among the ones a request brings.
pub fn cookie_value<'a>(headers: &'a HeaderMap, cookie: &str) -> Option<&'a str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find_map(|(name, value)| (name == cookie).then_some(value))
}

impl FromRequestParts<AppState> for Session {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        // What wraps the API has looked the session up already for requests
        // that change something (see `events::announce`).
        if let Some(session) = parts.extensions.get::<Self>() {
            return Ok(session.clone());
        }
        let session = match token(&parts.headers) {
            Some(token) => find(&state.db, token).await?,
            None => None,
        };
        session.ok_or_else(|| AppError::Status(StatusCode::UNAUTHORIZED, "not logged in".into()))
    }
}
