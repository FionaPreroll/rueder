//! What a reader does with their own account, and an admin with accounts:
//! the list of the own sessions, a new password, and making accounts.

use std::time::Instant;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{delete, get, put},
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{
    AppError, AppState, accounts,
    login::Key,
    sessions::{self, Session},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/sessions", get(own_sessions).delete(end_others))
        .route("/sessions/{id}", delete(end_one))
        .route("/password", put(change_password))
        .route("/accounts", get(all_accounts).post(add_account))
}

/// One place a reader is logged in.
#[derive(Serialize)]
struct Device {
    /// A name for the session that is not its token and leads to none.
    id: String,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    last_seen_at: OffsetDateTime,
    /// The session that asks.
    current: bool,
}

/// The reader's sessions, the one last used first.
async fn own_sessions(
    State(state): State<AppState>,
    session: Session,
    headers: HeaderMap,
) -> Result<Json<Vec<Device>>, AppError> {
    let current = sessions::token(&headers).map(sessions::token_hash);
    let devices = sqlx::query_as!(
        Device,
        r#"SELECT encode(substring(token_hash FROM 1 FOR 8), 'hex') AS "id!", created_at,
                  last_seen_at, token_hash = $2 AS "current!"
           FROM sessions WHERE account_id = $1 AND expires_at > now()
           ORDER BY last_seen_at DESC, created_at DESC"#,
        session.account_id,
        current.as_deref().unwrap_or_default(),
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(devices))
}

/// Ends one of the reader's sessions, by the name the list gave it.
async fn end_one(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let ended = sqlx::query!(
        "DELETE FROM sessions
         WHERE account_id = $1 AND encode(substring(token_hash FROM 1 FOR 8), 'hex') = $2",
        session.account_id,
        id,
    )
    .execute(&state.db)
    .await?;
    if ended.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Ends all sessions of the reader but the one that asks.
async fn end_others(
    State(state): State<AppState>,
    session: Session,
    headers: HeaderMap,
) -> Result<StatusCode, AppError> {
    let current = sessions::token(&headers).map(sessions::token_hash);
    sqlx::query!(
        "DELETE FROM sessions WHERE account_id = $1 AND token_hash <> $2",
        session.account_id,
        current.as_deref().unwrap_or_default(),
    )
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct NewPassword {
    /// The password so far. An account that has none needs none.
    #[serde(default)]
    current: String,
    new: String,
}

/// Gives the reader a new password. That ends every session of the account;
/// the browser that asks gets a new one.
async fn change_password(
    State(state): State<AppState>,
    session: Session,
    Json(change): Json<NewPassword>,
) -> Result<Response, AppError> {
    // A stolen session must not be a way to try passwords without end.
    let (key, now) = (Key::name(&session.username), Instant::now());
    if let Some(wait) = state.logins.blocked(&key, now) {
        let message = "too many wrong tries, try again later";
        return Err(AppError::TooMany(wait, message));
    }
    let hash = sqlx::query_scalar!(
        "SELECT password_hash FROM accounts WHERE id = $1",
        session.account_id
    )
    .fetch_one(&state.db)
    .await?;
    if hash.is_some() && !accounts::verify(&change.current, hash).await? {
        state.logins.failed(key, now);
        let message = "the current password is wrong";
        return Err(AppError::Status(StatusCode::FORBIDDEN, message.into()));
    }
    match accounts::set_password(&state.db, &session.username, &change.new).await? {
        Err(invalid) => Err(AppError::BadRequest(invalid.to_string())),
        Ok(false) => Err(AppError::NotFound),
        Ok(true) => {
            let token = sessions::start(&state.db, session.account_id).await?;
            let cookie = [(header::SET_COOKIE, sessions::cookie(&token))];
            Ok((StatusCode::NO_CONTENT, cookie).into_response())
        }
    }
}

fn admin(session: &Session) -> Result<(), AppError> {
    if session.is_admin {
        return Ok(());
    }
    let message = "only an admin can do this";
    Err(AppError::Status(StatusCode::FORBIDDEN, message.into()))
}

/// An account as an admin sees it.
#[derive(Serialize)]
struct Listed {
    id: i32,
    username: String,
    is_admin: bool,
    has_password: bool,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

/// All accounts, for an admin. Nothing of what they read.
async fn all_accounts(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<Vec<Listed>>, AppError> {
    admin(&session)?;
    let accounts = accounts::list(&state.db).await?.into_iter();
    let listed = accounts.map(|account| Listed {
        id: account.id,
        username: account.username,
        is_admin: account.is_admin,
        has_password: account.has_password,
        created_at: account.created_at,
    });
    Ok(Json(listed.collect()))
}

#[derive(Deserialize)]
struct NewAccount {
    username: String,
    /// The first password, which the admin hands over and the reader changes.
    password: String,
    #[serde(default)]
    is_admin: bool,
}

#[derive(Serialize)]
struct Created {
    id: i32,
}

/// Makes an account, as an admin.
async fn add_account(
    State(state): State<AppState>,
    session: Session,
    Json(new): Json<NewAccount>,
) -> Result<(StatusCode, Json<Created>), AppError> {
    admin(&session)?;
    let made = accounts::create(&state.db, &new.username, Some(&new.password), new.is_admin);
    match made.await? {
        Ok(id) => Ok((StatusCode::CREATED, Json(Created { id }))),
        Err(invalid) => Err(AppError::BadRequest(invalid.to_string())),
    }
}
