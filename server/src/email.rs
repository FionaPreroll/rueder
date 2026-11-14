//! The address of an account (work package 5.4, A16, A17, E6).
//!
//! Whether there is one is the operator's choice, `rueder registration set
//! --email off|optional|required` ([`Mode`]); as rueder comes it is `off`, and
//! none of this shows. With `optional` or `required` registration asks for an
//! address, and an account can give, change or (where it is optional) remove
//! one in its settings.
//!
//! An address counts only once its link was opened: a token of 32 random bytes
//! like an invitation's, of which only the SHA-256 is stored, good for
//! [`LINK_HOURS`] and for one use. Of an account only the newest link counts.
//! With `required`, an account made by registration logs in only after that
//! ([`awaiting`](crate::accounts)); one that never does goes after
//! [`UNCONFIRMED_DAYS`], so that its name is free again. An account that was
//! there before is asked by the page when it logs in.
//!
//! A new address is confirmed the same way, and the one before gets a notice
//! once it is replaced. Mail goes in the language the account was made in.
//! The status page names no address, and a move to another rueder takes none
//! along. How many mails an account, an address and the server may cause is
//! counted in [`crate::rate`].

use std::time::Instant;

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use ts_rs::TS;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    AppError, AppState, accounts,
    events::ServerEvent,
    login::{Key, Sender},
    mail::{self, Language, Template},
    passkeys, second_factor,
    sessions::{self, Session},
};

/// How many hours a link is good for.
pub const LINK_HOURS: i32 = 24;
/// After how many days an account that never confirmed its address goes.
pub const UNCONFIRMED_DAYS: i32 = 7;
/// The longest token that is looked up at all.
const MAX_TOKEN: usize = 100;
/// What PostgreSQL says when a unique index refuses a row.
const UNIQUE_VIOLATION: &str = "23505";

/// The types this module sends and reads, for the frontend.
pub(crate) fn types(types: &mut crate::typescript::Types) {
    types
        .add::<Mode>()
        .add::<Language>()
        .add::<EmailState>()
        .add::<NewEmail>()
        .add::<EmailProof>()
        .add::<Pending>()
        .add::<ConfirmEmail>()
        .add::<Confirmed>()
        .add::<Resend>();
}

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(state_of, change, remove))
        .routes(routes!(confirm))
        .routes(routes!(resend))
}

/// Whether accounts have an address, as the operator set it.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema, sqlx::Type,
)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[ts(rename = "EmailMode")]
#[schema(as = EmailMode)]
pub enum Mode {
    /// No addresses: registration does not ask, and nothing shows.
    #[default]
    Off,
    /// Registration asks, and an account may have one.
    Optional,
    /// Every account has one: a new one logs in only once it is confirmed,
    /// and one from before is asked for it.
    Required,
}

impl std::str::FromStr for Mode {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "off" => Ok(Self::Off),
            "optional" => Ok(Self::Optional),
            "required" => Ok(Self::Required),
            _ => Err("`off`, `optional` or `required`".into()),
        }
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Off => "off",
            Self::Optional => "optional",
            Self::Required => "required",
        })
    }
}

/// What the operator set.
pub async fn mode(conn: &mut PgConnection) -> anyhow::Result<Mode> {
    Ok(
        sqlx::query_scalar!(r#"SELECT email AS "email: Mode" FROM registration"#)
            .fetch_one(conn)
            .await?,
    )
}

/// The language a page names, for the mail: German or English.
pub fn language(text: Option<&str>) -> Language {
    match text.map(|text| text.trim().to_lowercase()) {
        Some(text) if text == "de" || text.starts_with("de-") => Language::De,
        _ => Language::En,
    }
}

/// The address of an account and the one waiting for its link, as the
/// account's own page sees them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS, ToSchema)]
pub struct EmailState {
    pub mode: Mode,
    /// The confirmed address, if the account has one.
    pub email: Option<String>,
    /// An address whose link was sent and not yet opened.
    pub pending: Option<String>,
}

/// The state of an account's address.
pub async fn state(conn: &mut PgConnection, account_id: i32) -> anyhow::Result<EmailState> {
    let mode = mode(&mut *conn).await?;
    let row = sqlx::query!(
        "SELECT a.email,
                (SELECT l.email FROM email_links l
                 WHERE l.account_id = a.id AND l.expires_at > now()
                 ORDER BY l.created_at DESC LIMIT 1) AS pending
         FROM accounts a WHERE a.id = $1",
        account_id
    )
    .fetch_one(&mut *conn)
    .await?;
    Ok(EmailState {
        mode,
        email: row.email,
        pending: row.pending,
    })
}

/// Makes a link that confirms `address` for the account, in place of any it
/// had, and mails it. Counted against the limits for mail. If the mail cannot
/// be handed to the relay, the answer is 503 and the link is gone again.
pub(crate) async fn send_link(
    state: &AppState,
    account_id: i32,
    name: &str,
    language: Language,
    address: &str,
) -> Result<(), AppError> {
    let Some(mailer) = state.mail.as_ref() else {
        return Err(no_mail());
    };
    state.rates.mail(account_id, address, Instant::now())?;
    let mut random = [0u8; 32];
    getrandom::fill(&mut random).map_err(|err| anyhow::anyhow!("no random numbers: {err}"))?;
    let token = URL_SAFE_NO_PAD.encode(random);
    let hash = sessions::token_hash(&token);
    let mut tx = state.db.begin().await?;
    // One at a time per account, so that of two asked at once only the
    // newer link is left.
    sqlx::query!(
        "SELECT id FROM accounts WHERE id = $1 FOR UPDATE",
        account_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    sqlx::query!("DELETE FROM email_links WHERE account_id = $1", account_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!(
        "INSERT INTO email_links (token_hash, account_id, email, expires_at)
         VALUES ($1, $2, $3, now() + $4::int4 * interval '1 hour')",
        hash,
        account_id,
        address,
        LINK_HOURS,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    let link = mailer.link("confirm", &token);
    let hours = LINK_HOURS.to_string();
    let site = mailer.site();
    let values = [
        ("name", name),
        ("site", site.as_str()),
        ("link", link.as_str()),
        ("hours", hours.as_str()),
    ];
    if let Err(err) = mailer
        .send(address, language, Template::Confirm, &values)
        .await
    {
        tracing::warn!("{}", without(&format!("{err:#}"), address));
        sqlx::query!("DELETE FROM email_links WHERE token_hash = $1", hash)
            .execute(&state.db)
            .await?;
        return Err(not_sent());
    }
    Ok(())
}

/// What a log says of a mail that failed, without the address it was for:
/// the relay's answer may repeat it.
pub(crate) fn without(text: &str, address: &str) -> String {
    format!(
        "a mail could not be handed to the relay: {}",
        text.replace(address, &mail::masked(address))
    )
}

fn no_mail() -> AppError {
    AppError::Status(
        StatusCode::SERVICE_UNAVAILABLE,
        "this server sends no mail".into(),
    )
}

fn not_sent() -> AppError {
    AppError::Status(
        StatusCode::SERVICE_UNAVAILABLE,
        "the mail could not be sent, try again later".into(),
    )
}

/// An address as it is given, or 400.
fn checked(text: &str) -> Result<String, AppError> {
    mail::address(text)
        .ok_or_else(|| AppError::BadRequest("that is no address mail can go to".into()))
}

/// The address of the account that is logged in, and the one waiting for its
/// link.
#[utoipa::path(
    get,
    path = "/email",
    operation_id = "email_state",
    tag = "Account",
    security(("session" = [])),
    responses((status = OK, body = EmailState), crate::openapi::NeedsSession),
)]
async fn state_of(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<EmailState>, AppError> {
    let mut conn = state.db.acquire().await?;
    Ok(Json(self::state(&mut conn, session.account_id).await?))
}

/// What `PUT /api/email` reads: the new address, and the account's password
/// and, where the second factor is on, one of its codes; or a passkey instead
/// of both. A stolen session must not be able to send the account's mail
/// elsewhere.
#[derive(Deserialize, TS, ToSchema)]
pub struct NewEmail {
    email: String,
    /// The language of the page, `en` or `de`: the account's mail is written
    /// in it from now on, unless a page made the account, whose language
    /// stays.
    #[serde(default)]
    #[ts(optional)]
    language: Option<String>,
    #[serde(flatten)]
    proof: EmailProof,
}

/// How the reader shows that they are the account's owner.
#[derive(Deserialize, TS, ToSchema)]
pub struct EmailProof {
    #[serde(default)]
    #[ts(as = "Option<String>", optional)]
    #[schema(value_type = Option<String>)]
    password: String,
    #[serde(default)]
    #[ts(as = "Option<String>", optional)]
    #[schema(value_type = Option<String>)]
    code: String,
    #[serde(default)]
    #[ts(optional)]
    passkey: Option<passkeys::Answer>,
}

/// An address that waits for its link.
#[derive(Serialize, TS, ToSchema)]
pub struct Pending {
    pending: String,
}

/// Gives the account a new address: it counts once the link mailed to it is
/// opened, and until then the one before stays.
#[utoipa::path(
    put,
    path = "/email",
    operation_id = "change_email",
    tag = "Account",
    request_body = NewEmail,
    security(("session" = [])),
    responses(
        (status = ACCEPTED, description = "The link is on its way to the address.", body = Pending),
        crate::openapi::NeedsSession,
        (status = BAD_REQUEST, description = "No address mail can go to."),
        (status = FORBIDDEN, description = "A wrong password or code, or `second factor needed`."),
        (status = CONFLICT, description = "Addresses are off on this server, or this is the address already."),
        (status = TOO_MANY_REQUESTS, description = "Too many wrong tries or too many mails; see `Retry-After`."),
        (status = SERVICE_UNAVAILABLE, description = "The mail could not be sent."),
    ),
)]
async fn change(
    State(state): State<AppState>,
    session: Session,
    headers: HeaderMap,
    Json(new): Json<NewEmail>,
) -> Result<(StatusCode, Json<Pending>), AppError> {
    let mut conn = state.db.acquire().await?;
    let now = self::state(&mut conn, session.account_id).await?;
    // Not held while the password is checked, which takes a while.
    drop(conn);
    if now.mode == Mode::Off {
        return Err(off());
    }
    let address = checked(&new.email)?;
    if now
        .email
        .as_deref()
        .is_some_and(|email| email.eq_ignore_ascii_case(&address))
    {
        let message = "that is the address already";
        return Err(AppError::Status(StatusCode::CONFLICT, message.into()));
    }
    let proof = passkeys::proof(
        &new.proof.password,
        &new.proof.code,
        new.proof.passkey.as_ref(),
    );
    second_factor::prove(&state, &session, &headers, proof).await?;
    // An account no page made takes the language of the page it first gives
    // an address in; any other keeps the one it was made in.
    let language = sqlx::query_scalar!(
        r#"UPDATE accounts SET language = COALESCE(language, $2) WHERE id = $1
           RETURNING language AS "language!: Language""#,
        session.account_id,
        language(new.language.as_deref()) as Language,
    )
    .fetch_one(&state.db)
    .await?;
    send_link(
        &state,
        session.account_id,
        &session.username,
        language,
        &address,
    )
    .await?;
    Ok((StatusCode::ACCEPTED, Json(Pending { pending: address })))
}

fn off() -> AppError {
    AppError::Status(
        StatusCode::CONFLICT,
        "this server keeps no addresses".into(),
    )
}

/// Takes the address from the account, and any link that waits; only where an
/// address is optional.
#[utoipa::path(
    delete,
    path = "/email",
    operation_id = "remove_email",
    tag = "Account",
    request_body = EmailProof,
    security(("session" = [])),
    responses(
        (status = NO_CONTENT, description = "The account has no address any more."),
        crate::openapi::NeedsSession,
        (status = FORBIDDEN, description = "A wrong password or code, or `second factor needed`."),
        (status = CONFLICT, description = "Addresses are off, or every account needs one."),
        (status = TOO_MANY_REQUESTS, description = "Too many wrong tries; see `Retry-After`."),
    ),
)]
async fn remove(
    State(state): State<AppState>,
    session: Session,
    headers: HeaderMap,
    Json(proof): Json<EmailProof>,
) -> Result<StatusCode, AppError> {
    let mut conn = state.db.acquire().await?;
    match mode(&mut conn).await? {
        Mode::Off => return Err(off()),
        Mode::Required => {
            let message = "every account here needs an address";
            return Err(AppError::Status(StatusCode::CONFLICT, message.into()));
        }
        Mode::Optional => {}
    }
    drop(conn);
    let proof = passkeys::proof(&proof.password, &proof.code, proof.passkey.as_ref());
    second_factor::prove(&state, &session, &headers, proof).await?;
    let mut tx = state.db.begin().await?;
    sqlx::query!(
        "DELETE FROM email_links WHERE account_id = $1",
        session.account_id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE accounts SET email = NULL WHERE id = $1",
        session.account_id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// What `POST /api/email/confirm` reads: the token from the link.
#[derive(Deserialize, TS, ToSchema)]
pub struct ConfirmEmail {
    token: String,
}

/// What a confirmed link did.
#[derive(Debug, Serialize, TS, ToSchema)]
pub struct Confirmed {
    /// The account it was for.
    pub username: String,
    /// Its address now.
    pub email: String,
    /// The account was waiting for its address and can log in now.
    pub activated: bool,
}

/// Opens a link from a mail: the address in it is the account's from now on.
/// For anybody who has the link, with or without a session; it does not log
/// in. An unknown, used or old link gets the same answer.
#[utoipa::path(
    post,
    path = "/email/confirm",
    operation_id = "confirm_email",
    tag = "Account",
    request_body = ConfirmEmail,
    responses(
        (status = OK, body = Confirmed),
        (status = NOT_FOUND, description = "No such link, or not any more: used, replaced by a newer one, or older than a day."),
        (status = CONFLICT, description = "Another account has the address meanwhile (`taken`)."),
    ),
)]
async fn confirm(
    State(state): State<AppState>,
    Json(asked): Json<ConfirmEmail>,
) -> Result<Json<Confirmed>, AppError> {
    let token = asked.token.trim();
    if token.is_empty() || token.len() > MAX_TOKEN {
        return Err(AppError::NotFound);
    }
    let hash = sessions::token_hash(token);
    let mut tx = state.db.begin().await?;
    let Some(link) = sqlx::query!(
        r#"SELECT l.account_id, l.email, a.username, a.email AS before,
                  COALESCE(a.language, 'en') AS "language!: Language", a.awaiting_email
           FROM email_links l JOIN accounts a ON a.id = l.account_id
           WHERE l.token_hash = $1 AND l.expires_at > now()
           FOR UPDATE OF a"#,
        hash
    )
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Err(AppError::NotFound);
    };
    sqlx::query!(
        "DELETE FROM email_links WHERE account_id = $1",
        link.account_id
    )
    .execute(&mut *tx)
    .await?;
    let set = sqlx::query!(
        "UPDATE accounts SET email = $2, awaiting_email = false WHERE id = $1",
        link.account_id,
        link.email,
    )
    .execute(&mut *tx)
    .await;
    match set {
        Ok(_) => {}
        Err(err)
            if err
                .as_database_error()
                .and_then(|err| err.code())
                .is_some_and(|code| code == UNIQUE_VIOLATION) =>
        {
            return Err(AppError::Status(StatusCode::CONFLICT, "taken".into()));
        }
        Err(err) => return Err(err.into()),
    }
    tx.commit().await?;

    // The address before hears of it, unless it was the same written otherwise.
    if let (Some(before), Some(mailer)) = (link.before.as_deref(), state.mail.as_ref())
        && !before.eq_ignore_ascii_case(&link.email)
    {
        let site = mailer.site();
        let address = mail::masked(&link.email);
        let values = [
            ("name", link.username.as_str()),
            ("site", site.as_str()),
            ("address", address.as_str()),
        ];
        let counted = state.rates.mail(link.account_id, before, Instant::now());
        if counted.is_ok()
            && let Err(err) = mailer
                .send(before, link.language, Template::Changed, &values)
                .await
        {
            tracing::warn!("{}", without(&format!("{err:#}"), before));
        }
    }
    // Its open tabs show the new address.
    let event = ServerEvent::Session { by: None };
    state.events.publish(vec![link.account_id], event);
    Ok(Json(Confirmed {
        username: link.username,
        email: link.email,
        activated: link.awaiting_email,
    }))
}

/// What `POST /api/email/resend` reads: the name and password of an account
/// that waits for its address.
#[derive(Deserialize, TS, ToSchema)]
pub struct Resend {
    username: String,
    password: String,
}

/// Sends the link again to an account that registered and has not confirmed
/// its address yet, which cannot log in to ask otherwise. Wrong passwords
/// count as at login.
#[utoipa::path(
    post,
    path = "/email/resend",
    operation_id = "resend_email_link",
    tag = "Account",
    request_body = Resend,
    responses(
        (status = ACCEPTED, description = "The link is on its way again.", body = Pending),
        (status = UNAUTHORIZED, description = "A wrong name or password (the same answer for both)."),
        (status = CONFLICT, description = "The account waits for no address."),
        (status = TOO_MANY_REQUESTS, description = "Too many wrong tries or too many mails; see `Retry-After`."),
        (status = SERVICE_UNAVAILABLE, description = "The mail could not be sent."),
    ),
)]
async fn resend(
    State(state): State<AppState>,
    Sender(ip): Sender,
    Json(asked): Json<Resend>,
) -> Result<(StatusCode, Json<Pending>), AppError> {
    let keys = Key::of(&asked.username, ip);
    if let Err(wait) = state.logins.attempt(&keys, Instant::now()) {
        let message = "too many wrong tries, try again later";
        return Err(AppError::TooMany(wait, message));
    }
    let Some(account) = accounts::login(&state.db, &asked.username, &asked.password).await? else {
        return Err(AppError::Status(
            StatusCode::UNAUTHORIZED,
            "wrong name or password".into(),
        ));
    };
    state.logins.succeeded(&keys);
    let waiting = sqlx::query!(
        r#"SELECT a.awaiting_email, COALESCE(a.language, 'en') AS "language!: Language",
                  (SELECT l.email FROM email_links l WHERE l.account_id = a.id
                   ORDER BY l.created_at DESC LIMIT 1) AS address
           FROM accounts a WHERE a.id = $1"#,
        account.id
    )
    .fetch_one(&state.db)
    .await?;
    let (true, Some(address)) = (waiting.awaiting_email, waiting.address) else {
        let message = "this account waits for no address";
        return Err(AppError::Status(StatusCode::CONFLICT, message.into()));
    };
    send_link(
        &state,
        account.id,
        &account.username,
        waiting.language,
        &address,
    )
    .await?;
    Ok((StatusCode::ACCEPTED, Json(Pending { pending: address })))
}

/// Whether the account waits for its address and so may not log in.
pub async fn awaiting(db: &PgPool, account_id: i32) -> anyhow::Result<bool> {
    Ok(sqlx::query_scalar!(
        "SELECT awaiting_email FROM accounts WHERE id = $1",
        account_id
    )
    .fetch_one(db)
    .await?)
}

/// Once an hour: links that ran out go, and accounts that never confirmed
/// their address after [`UNCONFIRMED_DAYS`]. The link of an account that
/// still waits stays until then, so that it can be sent again.
pub async fn sweep(db: &PgPool) -> anyhow::Result<u64> {
    let links = sqlx::query!(
        "DELETE FROM email_links l USING accounts a
         WHERE a.id = l.account_id AND l.expires_at <= now() AND NOT a.awaiting_email"
    )
    .execute(db)
    .await?
    .rows_affected();
    let accounts = sqlx::query!(
        "DELETE FROM accounts
         WHERE awaiting_email AND created_at < now() - $1::int4 * interval '1 day'",
        UNCONFIRMED_DAYS,
    )
    .execute(db)
    .await?
    .rows_affected();
    Ok(links + accounts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_and_languages_are_read() {
        for mode in [Mode::Off, Mode::Optional, Mode::Required] {
            assert_eq!(mode.to_string().parse::<Mode>(), Ok(mode));
        }
        assert!("sometimes".parse::<Mode>().is_err());
        assert_eq!(language(Some("de")), Language::De);
        assert_eq!(language(Some("de-AT")), Language::De);
        assert_eq!(language(Some("en")), Language::En);
        assert_eq!(language(Some("fr")), Language::En);
        assert_eq!(language(None), Language::En);
    }

    #[test]
    fn a_log_does_not_name_the_address() {
        let said = without("550 <nina@example.org>: no such user", "nina@example.org");
        assert!(!said.contains("nina@example.org"), "{said}");
        assert!(said.contains("n***@example.org"), "{said}");
    }
}
