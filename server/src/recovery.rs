//! "Forgot password" (work package 5.5, A17): a link mailed to the account's
//! confirmed address sets a new password.
//!
//! It is there where accounts have addresses (`rueder registration set
//! --email optional|required`) and the server sends mail; otherwise the
//! endpoints answer 404 and the login page offers nothing, and a forgotten
//! password stays the operator's job (`rueder account password`).
//!
//! `POST /api/password/forgot` takes a name or an address and gives every
//! asker the same answer, at once: whether there is such an account, and
//! whether a mail goes out, is worked out afterwards, in the background, so
//! that neither the answer nor its time tells. Asking is counted like a wrong
//! password at login, per name, sender and network, in a counter of its own,
//! so that whoever asks for somebody else's name does not lock their login.
//! The limits on mail count silently on top.
//!
//! The link is `/?reset=<token>` on `PUBLIC_URL`: 32 random bytes, of which
//! only the SHA-256 is stored, good for [`LINK_MINUTES`] and once, and only
//! while the account still has the address it went to. Of an account only the
//! newest counts. `POST /api/password/reset` sets the new password, which ends
//! every session and app password of the account like any new password; it
//! does not log in, so the login asks for a second factor where the account
//! has one. The address then hears that the password changed.
//!
//! Only accounts that have a password get a link: one that logs in through
//! OpenID Connect alone gets no new way in.

use std::time::Instant;

use axum::{Json, extract::State, http::StatusCode};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;
use sqlx::PgPool;
use ts_rs::TS;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    AppError, AppState, accounts,
    email::{self, Mode},
    events::ServerEvent,
    login::{Key, Sender},
    mail::{self, Language, Template},
    sessions,
};

/// How many minutes a link is good for.
pub const LINK_MINUTES: i32 = 30;
/// The longest token that is looked up at all.
const MAX_TOKEN: usize = 100;

/// The types this module reads, for the frontend.
pub(crate) fn types(types: &mut crate::typescript::Types) {
    types.add::<ForgotPassword>().add::<ResetPassword>();
}

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(forgot))
        .routes(routes!(reset))
}

/// Whether this server offers the way: it keeps addresses and sends mail.
pub async fn offered(state: &AppState) -> anyhow::Result<bool> {
    if state.mail.is_none() {
        return Ok(false);
    }
    let mut conn = state.db.acquire().await?;
    Ok(email::mode(&mut conn).await? != Mode::Off)
}

/// What `POST /api/password/forgot` reads: the account's name or its address.
#[derive(Deserialize, TS, ToSchema)]
pub struct ForgotPassword {
    name: String,
}

/// Mails a link that sets a new password to the confirmed address of the
/// account with this name or this address, if there is one. The answer is the
/// same whether there is or not, and comes before the mail goes.
#[utoipa::path(
    post,
    path = "/password/forgot",
    operation_id = "forgot_password",
    tag = "Session",
    request_body = ForgotPassword,
    responses(
        (status = ACCEPTED, description = "Asked. If there is such an account with a confirmed address, a link is on its way to it; the answer says neither."),
        (status = BAD_REQUEST, description = "No name or address given, or a longer one than there can be."),
        (status = NOT_FOUND, description = "This server keeps no addresses or sends no mail."),
        (status = TOO_MANY_REQUESTS, description = "Asked too often for this name or from this sender; see `Retry-After`."),
    ),
)]
async fn forgot(
    State(state): State<AppState>,
    Sender(ip): Sender,
    Json(asked): Json<ForgotPassword>,
) -> Result<StatusCode, AppError> {
    if !offered(&state).await? {
        return Err(AppError::NotFound);
    }
    let who = asked.name.trim();
    if who.is_empty() || who.chars().count() > mail::MAX_ADDRESS {
        return Err(AppError::BadRequest("give the name or the address".into()));
    }
    // Counted for whoever is asked for, known or not, and never taken back.
    if let Err(wait) = state.resets.attempt(&Key::of(who, ip), Instant::now()) {
        let message = "asked too often, try again later";
        return Err(AppError::TooMany(wait, message));
    }
    let (state, who) = (state.clone(), who.to_owned());
    tokio::spawn(async move {
        if let Err(err) = send_link(&state, &who).await {
            tracing::warn!("a link to set a new password was not sent: {err:#}");
        }
    });
    Ok(StatusCode::ACCEPTED)
}

/// Finds the account and mails its link, if there is one to mail. Whatever
/// stops it says nothing to the asker, who has had the answer already.
async fn send_link(state: &AppState, who: &str) -> anyhow::Result<()> {
    let Some(mailer) = state.mail.as_ref() else {
        return Ok(());
    };
    // A name before an address, where somebody's name is another's address.
    let Some(account) = sqlx::query!(
        r#"SELECT id, username, email AS "email!",
                  COALESCE(language, 'en') AS "language!: Language"
           FROM accounts
           WHERE (lower(username) = lower($1) OR lower(email) = lower($1))
             AND email IS NOT NULL AND password_hash IS NOT NULL
             AND disabled_at IS NULL AND NOT awaiting_email
           ORDER BY lower(username) = lower($1) DESC
           LIMIT 1"#,
        who
    )
    .fetch_optional(&state.db)
    .await?
    else {
        return Ok(());
    };
    if state
        .rates
        .mail(account.id, &account.email, Instant::now())
        .is_err()
    {
        tracing::info!("a link to set a new password waits for the limits on mail");
        return Ok(());
    }
    let mut random = [0u8; 32];
    getrandom::fill(&mut random).map_err(|err| anyhow::anyhow!("no random numbers: {err}"))?;
    let token = URL_SAFE_NO_PAD.encode(random);
    let hash = sessions::token_hash(&token);
    let mut tx = state.db.begin().await?;
    // One at a time per account, so that of two asked at once only the
    // newer link is left.
    sqlx::query!(
        "SELECT id FROM accounts WHERE id = $1 FOR UPDATE",
        account.id
    )
    .fetch_optional(&mut *tx)
    .await?;
    sqlx::query!(
        "DELETE FROM password_resets WHERE account_id = $1",
        account.id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "INSERT INTO password_resets (token_hash, account_id, email, expires_at)
         VALUES ($1, $2, $3, now() + $4::int4 * interval '1 minute')",
        hash,
        account.id,
        account.email,
        LINK_MINUTES,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    let link = mailer.link("reset", &token);
    let (site, minutes) = (mailer.site(), LINK_MINUTES.to_string());
    let values = [
        ("name", account.username.as_str()),
        ("site", site.as_str()),
        ("link", link.as_str()),
        ("minutes", minutes.as_str()),
    ];
    let sent = mailer
        .send(&account.email, account.language, Template::Reset, &values)
        .await;
    if let Err(err) = sent {
        sqlx::query!("DELETE FROM password_resets WHERE token_hash = $1", hash)
            .execute(&state.db)
            .await?;
        anyhow::bail!("{}", email::without(&format!("{err:#}"), &account.email));
    }
    Ok(())
}

/// What `POST /api/password/reset` reads: the token from the link and the new
/// password.
#[derive(Deserialize, TS, ToSchema)]
pub struct ResetPassword {
    token: String,
    #[serde(rename = "new")]
    #[ts(rename = "new")]
    #[schema(rename = "new")]
    fresh: String,
}

/// Sets a new password with the link from a mail. Every session and app
/// password of the account ends, and so does the link; it does not log in.
/// An unknown, used, replaced or old link gets the same answer.
#[utoipa::path(
    post,
    path = "/password/reset",
    operation_id = "reset_password",
    tag = "Session",
    request_body = ResetPassword,
    responses(
        (status = NO_CONTENT, description = "The account has the new password; it logs in with it as always, with its second factor where it has one."),
        (status = BAD_REQUEST, description = "A password that cannot be one; the answer says why."),
        (status = NOT_FOUND, description = "No such link, or not any more: used, replaced by a newer one, older than half an hour, or the account has another address now. Also where this server keeps no addresses."),
    ),
)]
async fn reset(
    State(state): State<AppState>,
    Json(asked): Json<ResetPassword>,
) -> Result<StatusCode, AppError> {
    if !offered(&state).await? {
        return Err(AppError::NotFound);
    }
    let token = asked.token.trim();
    if token.is_empty() || token.len() > MAX_TOKEN {
        return Err(AppError::NotFound);
    }
    if let Err(invalid) = accounts::check_password(&asked.fresh) {
        return Err(AppError::BadRequest(invalid.to_string()));
    }
    let hash = sessions::token_hash(token);
    // Looked at before the password is hashed, which is work: a made-up link
    // does not make the server do it.
    if find(&state.db, &hash).await?.is_none() {
        return Err(AppError::NotFound);
    }
    let password_hash = accounts::hash(&asked.fresh).await?;
    let mut tx = state.db.begin().await?;
    // Again, and held: two tabs with the same link set one password.
    let Some(link) = sqlx::query!(
        r#"SELECT r.account_id, a.username, r.email,
                  COALESCE(a.language, 'en') AS "language!: Language"
           FROM password_resets r JOIN accounts a ON a.id = r.account_id
           WHERE r.token_hash = $1 AND r.expires_at > now() AND a.email = r.email
             AND a.password_hash IS NOT NULL AND a.disabled_at IS NULL
           FOR UPDATE OF r, a"#,
        hash
    )
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Err(AppError::NotFound);
    };
    accounts::replace_password(&mut tx, link.account_id, &password_hash).await?;
    tx.commit().await?;

    // Its open tabs find their session gone.
    let event = ServerEvent::Session { by: None };
    state.events.publish(vec![link.account_id], event);
    if let Some(mailer) = state.mail.as_ref() {
        let site = mailer.site();
        let values = [("name", link.username.as_str()), ("site", site.as_str())];
        let counted = state
            .rates
            .mail(link.account_id, &link.email, Instant::now());
        if counted.is_ok()
            && let Err(err) = mailer
                .send(
                    &link.email,
                    link.language,
                    Template::PasswordChanged,
                    &values,
                )
                .await
        {
            tracing::warn!("{}", email::without(&format!("{err:#}"), &link.email));
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

/// The account a link that is still good is for.
async fn find(db: &PgPool, hash: &[u8]) -> anyhow::Result<Option<i32>> {
    Ok(sqlx::query_scalar!(
        "SELECT r.account_id FROM password_resets r JOIN accounts a ON a.id = r.account_id
         WHERE r.token_hash = $1 AND r.expires_at > now() AND a.email = r.email",
        hash
    )
    .fetch_optional(db)
    .await?)
}

/// Once an hour: links that ran out go.
pub async fn sweep(db: &PgPool) -> anyhow::Result<u64> {
    Ok(
        sqlx::query!("DELETE FROM password_resets WHERE expires_at <= now()")
            .execute(db)
            .await?
            .rows_affected(),
    )
}
