//! What a reader does with their own account, and an admin with accounts:
//! the list of the own sessions, a new password, deleting the own account,
//! and making accounts.

use std::time::Instant;

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ts_rs::TS;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    AppError, AppState, accounts,
    login::{CODE_NEEDED, Key},
    passkeys, second_factor,
    sessions::{self, Session},
};

/// The types this module sends and reads, for the frontend.
pub(crate) fn types(types: &mut crate::typescript::Types) {
    types
        .add::<Device>()
        .add::<NewPassword>()
        .add::<Farewell>()
        .add::<Listed>()
        .add::<NewAccount>()
        .add::<Created>();
}

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(own_sessions, end_others))
        .routes(routes!(end_one))
        .routes(routes!(change_password))
        .routes(routes!(remove_account))
        .routes(routes!(all_accounts, add_account))
}

/// One place a reader is logged in.
///
/// `GET /api/sessions` answers with a list of them; `DELETE /api/sessions/{id}`
/// ends one, `DELETE /api/sessions` all but the one that asks.
#[derive(Serialize, TS, ToSchema)]
struct Device {
    /// A name for the session that is not its token and leads to none.
    id: String,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    #[schema(value_type = String)]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    #[schema(value_type = String)]
    last_seen_at: OffsetDateTime,
    /// The session that asks.
    current: bool,
}

/// The reader's sessions, the one last used first.
#[utoipa::path(
    get,
    path = "/sessions",
    tag = "Account",
    security(("session" = [])),
    responses((status = OK, body = Vec<Device>), crate::openapi::NeedsSession),
)]
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
#[utoipa::path(
    delete,
    path = "/sessions/{id}",
    tag = "Account",
    params(("id" = String, Path, description = "The session, as the list names it.")),
    security(("session" = [])),
    responses((status = NO_CONTENT, description = "Done."), crate::openapi::NeedsSession, (status = NOT_FOUND, description = "No such session of the account.")),
)]
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
#[utoipa::path(
    delete,
    path = "/sessions",
    tag = "Account",
    security(("session" = [])),
    responses((status = NO_CONTENT, description = "Done."), crate::openapi::NeedsSession),
)]
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

/// What `PUT /api/password` reads.
#[derive(Deserialize, TS, ToSchema)]
struct NewPassword {
    /// The password so far. An account that has none needs none.
    #[serde(default)]
    #[ts(as = "Option<String>", optional)]
    #[schema(value_type = Option<String>)]
    current: String,
    new: String,
}

/// Gives the reader a new password. That ends every session of the account;
/// the browser that asks gets a new one.
#[utoipa::path(
    put,
    path = "/password",
    tag = "Account",
    request_body = NewPassword,
    security(("session" = [])),
    responses((status = NO_CONTENT, description = "Changed; the answer sets the new session's cookie."), crate::openapi::NeedsSession, (status = BAD_REQUEST, description = "A new password that is too short or too long."), (status = FORBIDDEN, description = "Password, code or passkey of the account are wrong."), (status = TOO_MANY_REQUESTS, description = "Too many wrong tries; see `Retry-After`.")),
)]
async fn change_password(
    State(state): State<AppState>,
    session: Session,
    Json(change): Json<NewPassword>,
) -> Result<Response, AppError> {
    // A stolen session must not be a way to try passwords without end.
    let keys = [Key::name(&session.username)];
    // Counted as wrong until it turns out right, as at login.
    if let Err(wait) = state.logins.attempt(&keys, Instant::now()) {
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
        let message = "the current password is wrong";
        return Err(AppError::Status(StatusCode::FORBIDDEN, message.into()));
    }
    state.logins.succeeded(&keys);
    match accounts::set_password(&state.db, &session.username, &change.new).await? {
        Err(invalid) => Err(AppError::BadRequest(invalid.to_string())),
        Ok(false) => Err(AppError::NotFound),
        Ok(true) => {
            let token =
                sessions::start(&state.db, session.account_id, session.second_factor).await?;
            let cookie = [(header::SET_COOKIE, sessions::cookie(&token))];
            Ok((StatusCode::NO_CONTENT, cookie).into_response())
        }
    }
}

/// What `DELETE /api/account` reads: proof that the reader means it.
#[derive(Deserialize, TS, ToSchema)]
struct Farewell {
    /// The account's password. An account that has none sends its name.
    #[serde(default)]
    #[ts(as = "Option<String>", optional)]
    #[schema(value_type = Option<String>)]
    password: String,
    #[serde(default)]
    #[ts(as = "Option<String>", optional)]
    #[schema(value_type = Option<String>)]
    name: String,
    /// A code of the app or a recovery code, where the second factor is on.
    #[serde(default)]
    #[ts(as = "Option<String>", optional)]
    #[schema(value_type = Option<String>)]
    code: String,
    /// A passkey of the account, instead of all the above.
    #[serde(default)]
    #[ts(optional)]
    passkey: Option<passkeys::Answer>,
}

/// Deletes the reader's own account and everything that is its own (see
/// [`accounts::remove`]). Its sessions end, on this device and all others,
/// and so do its connections for live updates, as `events::changed` counts
/// this address as a session that ended. Where the second factor is on, it
/// takes a code as well, as at login: without one the answer is 403 with
/// `second factor needed`, and a wrong one counts like a wrong password. A
/// passkey of the account stands for all of it.
#[utoipa::path(
    delete,
    path = "/account",
    tag = "Account",
    request_body = Farewell,
    security(("session" = [])),
    responses((status = NO_CONTENT, description = "Deleted; the answer clears the cookie."), crate::openapi::NeedsSession, (status = BAD_REQUEST, description = "The name typed to confirm is not the account's."), (status = FORBIDDEN, description = "Password, code or passkey of the account are wrong."), (status = TOO_MANY_REQUESTS, description = "Too many wrong tries; see `Retry-After`.")),
)]
async fn remove_account(
    State(state): State<AppState>,
    session: Session,
    headers: HeaderMap,
    Json(farewell): Json<Farewell>,
) -> Result<Response, AppError> {
    if let Some(answer) = &farewell.passkey {
        passkeys::check_proof(&state, &session, &headers, answer).await?;
        return remove_proven(&state, &session).await;
    }
    // As for a new password: a stolen session must not be a way to try
    // passwords without end.
    let keys = [Key::name(&session.username)];
    if let Err(wait) = state.logins.attempt(&keys, Instant::now()) {
        let message = "too many wrong tries, try again later";
        return Err(AppError::TooMany(wait, message));
    }
    let hash = sqlx::query_scalar!(
        "SELECT password_hash FROM accounts WHERE id = $1",
        session.account_id
    )
    .fetch_one(&state.db)
    .await?;
    let proven = match hash {
        Some(_) => accounts::verify(&farewell.password, hash).await?,
        // Logged in through OpenID Connect only: typing the name shows that a
        // person means it, not a page that sends requests in their name.
        None => farewell.name.trim().to_lowercase() == session.username.to_lowercase(),
    };
    if !proven {
        let message = "the password is wrong";
        return Err(AppError::Status(StatusCode::FORBIDDEN, message.into()));
    }
    if second_factor::is_on(&state.db, session.account_id).await? {
        let code = farewell.code.trim();
        if code.is_empty() {
            // The password was right; the code is what is missing.
            state.logins.succeeded(&keys);
            return Err(AppError::Status(StatusCode::FORBIDDEN, CODE_NEEDED.into()));
        }
        // A wrong code stays counted, like a wrong password.
        if !second_factor::take(&state.db, session.account_id, code).await? {
            let message = "wrong code";
            return Err(AppError::Status(StatusCode::FORBIDDEN, message.into()));
        }
    }
    state.logins.succeeded(&keys);
    remove_proven(&state, &session).await
}

/// Deletes the session's account, once its owner showed they are there.
async fn remove_proven(state: &AppState, session: &Session) -> Result<Response, AppError> {
    if !accounts::remove(&state.db, session.account_id).await? {
        return Err(AppError::NotFound);
    }
    tracing::info!("an account deleted itself");
    let cookie = [(header::SET_COOKIE, sessions::no_cookie())];
    Ok((StatusCode::NO_CONTENT, cookie).into_response())
}

pub(crate) fn admin(session: &Session) -> Result<(), AppError> {
    if session.is_admin {
        return Ok(());
    }
    let message = "only an admin can do this";
    Err(AppError::Status(StatusCode::FORBIDDEN, message.into()))
}

/// An account as an admin sees it.
///
/// `GET /api/accounts` answers with a list of them, for admins only.
#[derive(Serialize, TS, ToSchema)]
#[ts(rename = "Account")]
#[schema(as = Account)]
struct Listed {
    id: i32,
    username: String,
    is_admin: bool,
    has_password: bool,
    /// Disabled by the operator, with `rueder account disable`.
    disabled: bool,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    #[schema(value_type = String)]
    created_at: OffsetDateTime,
}

/// All accounts, for an admin. Nothing of what they read.
#[utoipa::path(
    get,
    path = "/accounts",
    tag = "Admin",
    security(("session" = [])),
    responses((status = OK, body = Vec<Listed>), crate::openapi::NeedsSession, (status = FORBIDDEN, description = "The account is no admin, or an admin without the second factor where the operator asks for it.")),
)]
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
        disabled: account.disabled,
        created_at: account.created_at,
    });
    Ok(Json(listed.collect()))
}

/// What `POST /api/accounts` reads, for admins only. It answers with `Created`.
#[derive(Deserialize, TS, ToSchema)]
struct NewAccount {
    username: String,
    /// The first password, which the admin hands over and the reader changes.
    password: String,
    #[serde(default)]
    #[ts(as = "Option<bool>", optional)]
    #[schema(value_type = Option<bool>)]
    is_admin: bool,
}

/// The answer to making something: the number it got.
#[derive(Serialize, TS, ToSchema)]
struct Created {
    id: i32,
}

/// Makes an account, as an admin.
#[utoipa::path(
    post,
    path = "/accounts",
    tag = "Admin",
    request_body = NewAccount,
    security(("session" = [])),
    responses((status = CREATED, body = Created), crate::openapi::NeedsSession, (status = BAD_REQUEST, description = "A name that is taken or not allowed, or a password that is too short or too long."), (status = FORBIDDEN, description = "The account is no admin, or an admin without the second factor where the operator asks for it.")),
)]
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
