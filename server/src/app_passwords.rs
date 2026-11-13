//! App passwords: how apps of other makers log in to the Google Reader API
//! (see [`crate::greader`]).
//!
//! - **Making one** (`POST /api/app-passwords`) takes the account's password
//!   and, where the second factor is on, one of its codes, counted like a
//!   login. The answer shows the app password this once; the database keeps
//!   only its SHA-256. An account has at most [`MAX_PER_ACCOUNT`].
//! - **The list** (`GET /api/app-passwords`) has each one's name, when it was
//!   made and when an app last used it. `DELETE /api/app-passwords/{id}`
//!   revokes one, and with it the addresses of its images.
//! - **Apps** log in with the account's name and an app password, never with
//!   the account's password: so the second factor holds, an account that
//!   logs in through OpenID Connect can use apps, and a lost device costs
//!   only its own app password. A new password of the account and disabling
//!   it end all of them, deleting it takes them along.
//!
//! An app password is 24 letters and digits of base32, 120 bits, shown in
//! groups of four. Apps send it as it is typed; dashes, blanks and case do not
//! count.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use time::OffsetDateTime;
use ts_rs::TS;

use crate::{
    AppError, AppState,
    login::{CODE_NEEDED, Key},
    second_factor,
    sessions::Session,
    tree,
};

/// How many app passwords an account can have.
pub const MAX_PER_ACCOUNT: i64 = 20;
/// How many letters an app password has: 5 bits each, 120 in all.
pub const SECRET_LEN: usize = 24;
/// The letters of base32, in lower case: easy to type on a phone.
const ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";
/// How much later an app password is used again before that is written down,
/// in seconds.
const TOUCH_AFTER: f64 = 3600.0;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/app-passwords", get(list).post(create))
        .route("/app-passwords/{id}", delete(revoke))
}

/// The types of this module for the frontend.
pub(crate) fn types(types: &mut crate::typescript::Types) {
    types.add::<AppPasswords>().add::<New>().add::<Created>();
}

/// An app password as the list shows it: never the password itself.
#[derive(Serialize, TS)]
#[ts(rename = "AppPassword")]
pub struct Listed {
    id: i32,
    name: String,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    created_at: OffsetDateTime,
    /// When an app last logged in with it, to the hour; null until one did.
    #[serde(with = "time::serde::rfc3339::option")]
    #[ts(type = "string | null")]
    used_at: Option<OffsetDateTime>,
}

/// What `GET /api/app-passwords` answers.
#[derive(Serialize, TS)]
struct AppPasswords {
    app_passwords: Vec<Listed>,
    /// How many the account can have.
    max: i64,
}

/// What `POST /api/app-passwords` reads: a name for it, and the account's
/// password and, where the second factor is on, one of its codes.
#[derive(Deserialize, TS)]
#[ts(rename = "NewAppPassword")]
struct New {
    name: String,
    #[serde(default)]
    #[ts(as = "Option<String>", optional)]
    password: String,
    #[serde(default)]
    #[ts(as = "Option<String>", optional)]
    code: String,
}

/// What `POST /api/app-passwords` answers: the app password, this once.
#[derive(Serialize, TS)]
#[ts(rename = "CreatedAppPassword")]
struct Created {
    #[serde(flatten)]
    listed: Listed,
    /// In groups of four letters, as it is shown to be typed.
    secret: String,
}

/// An app password as it is compared: without dashes and blanks, in lower case.
fn normal(secret: &str) -> String {
    secret
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .flat_map(char::to_lowercase)
        .collect()
}

/// The token an app gets for an app password: the app password as it is
/// compared, without dashes, which apps read up to an `=`.
pub fn token(secret: &str) -> String {
    normal(secret)
}

/// The hash an app password is kept and found by, if it can be one at all.
pub(crate) fn hash_of(secret: &str) -> Option<Vec<u8>> {
    let secret = normal(secret);
    let fits = secret.len() == SECRET_LEN && secret.bytes().all(|b| ALPHABET.contains(&b));
    fits.then(|| Sha256::digest(secret.as_bytes()).to_vec())
}

/// A new app password, in groups of four.
fn new_secret() -> anyhow::Result<String> {
    let mut bytes = [0u8; SECRET_LEN];
    getrandom::fill(&mut bytes).map_err(|err| anyhow::anyhow!("no random numbers: {err}"))?;
    // 256 is a multiple of 32, so every letter is as likely as any other.
    let letters: Vec<char> = bytes
        .iter()
        .map(|byte| char::from(ALPHABET[usize::from(byte & 31)]))
        .collect();
    let groups: Vec<String> = letters
        .chunks(4)
        .map(|group| group.iter().collect())
        .collect();
    Ok(groups.join("-"))
}

/// An app that logged in: the account as a session for the API's endpoints,
/// never an admin's, and which app password it was.
#[derive(Debug, Clone)]
pub struct AppLogin {
    pub id: i32,
    pub session: Session,
    /// The app password was never used before: an app just got set up.
    pub first_use: bool,
}

/// The app password a secret stands for, if it is one and its account is not
/// disabled. Using it is written down, at most once an hour.
pub async fn find(db: &PgPool, secret: &str) -> anyhow::Result<Option<AppLogin>> {
    let Some(hash) = hash_of(secret) else {
        return Ok(None);
    };
    let found = sqlx::query!(
        r#"SELECT p.id, p.used_at IS NULL AS "first!",
                  coalesce(p.used_at < now() - $2::float8 * interval '1 second', true)
                      AS "stale!",
                  a.id AS account_id, a.username, a.terms_version
           FROM app_passwords p JOIN accounts a ON a.id = p.account_id
           WHERE p.secret_hash = $1 AND a.disabled_at IS NULL"#,
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
            "UPDATE app_passwords SET used_at = now() WHERE id = $1",
            found.id
        )
        .execute(db)
        .await?;
    }
    Ok(Some(AppLogin {
        id: found.id,
        session: Session {
            account_id: found.account_id,
            username: found.username,
            is_admin: false,
            second_factor: false,
            admin_needs_second_factor: false,
            terms_version: found.terms_version,
        },
        first_use: found.first,
    }))
}

/// Whether the app password of this number still counts, for the addresses
/// of its images.
pub async fn counts(db: &PgPool, id: i32) -> anyhow::Result<bool> {
    let found = sqlx::query_scalar!(
        "SELECT p.id FROM app_passwords p JOIN accounts a ON a.id = p.account_id
         WHERE p.id = $1 AND a.disabled_at IS NULL",
        id
    )
    .fetch_optional(db)
    .await?;
    Ok(found.is_some())
}

async fn list(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<AppPasswords>, AppError> {
    if !state.app_api {
        return Err(AppError::NotFound);
    }
    let app_passwords = sqlx::query_as!(
        Listed,
        "SELECT id, name, created_at, used_at FROM app_passwords
         WHERE account_id = $1 ORDER BY created_at, id",
        session.account_id
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(AppPasswords {
        app_passwords,
        max: MAX_PER_ACCOUNT,
    }))
}

/// Makes an app password, with the account's password and, where the second
/// factor is on, one of its codes: a stolen session must not be enough for a
/// way in that outlasts it. Without a code the answer is 403 with `second
/// factor needed`; wrong ones count like wrong passwords at login.
async fn create(
    State(state): State<AppState>,
    session: Session,
    Json(new): Json<New>,
) -> Result<(StatusCode, Json<Created>), AppError> {
    if !state.app_api {
        return Err(AppError::NotFound);
    }
    let name = tree::title(&new.name)?;
    let keys = [Key::name(&session.username)];
    second_factor::tried_password(&state, &session, &keys, &new.password).await?;
    if second_factor::is_on(&state.db, session.account_id).await? {
        let code = new.code.trim();
        if code.is_empty() {
            // The password was right; the code is what is missing.
            state.logins.succeeded(&keys);
            return Err(AppError::Status(StatusCode::FORBIDDEN, CODE_NEEDED.into()));
        }
        // A wrong code stays counted, like a wrong password.
        if !second_factor::take(&state.db, session.account_id, code).await? {
            let message = "the code is wrong";
            return Err(AppError::Status(StatusCode::FORBIDDEN, message.into()));
        }
    }
    state.logins.succeeded(&keys);
    let mut tx = state.db.begin().await?;
    tree::lock(&mut tx, session.account_id).await?;
    let have = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM app_passwords WHERE account_id = $1"#,
        session.account_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if have >= MAX_PER_ACCOUNT {
        let message = format!("an account has at most {MAX_PER_ACCOUNT} app passwords");
        return Err(AppError::BadRequest(message));
    }
    let secret = new_secret()?;
    let Some(hash) = hash_of(&secret) else {
        return Err(anyhow::anyhow!("a new app password is no app password").into());
    };
    let listed = sqlx::query_as!(
        Listed,
        "INSERT INTO app_passwords (account_id, name, secret_hash) VALUES ($1, $2, $3)
         RETURNING id, name, created_at, used_at",
        session.account_id,
        name,
        hash,
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(Created { listed, secret })))
}

/// Revokes an app password: the app it was for is logged out, and the images
/// in what it fetched stop loading.
async fn revoke(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i32>,
) -> Result<StatusCode, AppError> {
    if !state.app_api {
        return Err(AppError::NotFound);
    }
    let gone = sqlx::query!(
        "DELETE FROM app_passwords WHERE account_id = $1 AND id = $2",
        session.account_id,
        id
    )
    .execute(&state.db)
    .await?;
    if gone.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_app_password_is_found_however_it_is_typed() {
        let secret = new_secret().unwrap();
        assert_eq!(secret.len(), SECRET_LEN + SECRET_LEN / 4 - 1);
        assert_eq!(secret.split('-').count(), SECRET_LEN / 4);
        let hash = hash_of(&secret).unwrap();
        assert_eq!(hash_of(&secret.to_uppercase()), Some(hash.clone()));
        assert_eq!(hash_of(&secret.replace('-', " ")), Some(hash.clone()));
        assert_eq!(
            hash_of(&format!(" {} ", secret.replace('-', ""))),
            Some(hash)
        );
        assert_ne!(new_secret().unwrap(), secret);
        // Too short, too long, or with letters base32 has not: no app password.
        assert_eq!(hash_of(&secret[..10]), None);
        assert_eq!(hash_of(&format!("{secret}a")), None);
        assert_eq!(hash_of(&"0".repeat(SECRET_LEN)), None);
        assert_eq!(hash_of(""), None);
    }
}
