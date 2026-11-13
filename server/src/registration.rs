//! Accounts that make themselves: open registration and invitations.
//!
//! The operator says on the command line (`rueder registration set`) whether
//! anybody may register and how many; it is kept in the database, so a change
//! counts at once. Until the operator opens something, nothing is open.
//!
//! There are two ways in, and two pools of places that do not touch:
//!
//! - **The general pool.** Accounts the operator made, accounts from open
//!   registration and accounts from readers' invitations count against
//!   `max_accounts`; the latter two also against `per_day`. A reader may
//!   invite as many accounts in all as their quota says.
//! - **The admins' pool.** An admin can always make invitations, whatever the
//!   quota and whether or not registration is open. The accounts they bring
//!   count against `admin_invite_accounts` only.
//!
//! An invitation is a link with a random token, like a session's; the database
//! keeps its SHA-256. It is good for a number of accounts and for a number of
//! days, and whoever made it can take it back.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use ts_rs::TS;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    AppError, AppState, accounts, legal,
    login::Me,
    sessions::{self, Session},
};

/// For how many accounts one link can be at most.
pub const MAX_USES: i32 = 1000;
/// How many links an account can have at a time, used up or not.
pub const MAX_LINKS: i64 = 200;
/// The longest token a request may name; the real ones are 43 characters.
const MAX_TOKEN: usize = 100;

/// The types this module sends and reads, for the frontend.
pub(crate) fn types(types: &mut crate::typescript::Types) {
    types
        .add::<AskedInvite>()
        .add::<Registering>()
        .add::<NewRegistration>()
        .add::<Invite>()
        .add::<Invites>()
        .add::<NewInvite>()
        .add::<MadeInvite>();
}

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(registering, register))
        .routes(routes!(invites, invite))
        .routes(routes!(revoke))
}

/// What the operator set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// Anybody can register, without an invitation.
    pub open: bool,
    /// The most accounts the general pool holds; `None` is no limit.
    pub max_accounts: Option<i32>,
    /// The most accounts registration and readers' invitations make in 24 hours.
    pub per_day: Option<i32>,
    /// How many accounts a reader may invite in all, unless their account says otherwise.
    pub invites_per_account: i32,
    /// How many days an invitation is good for.
    pub invite_days: i32,
    /// The most accounts admins' invitations make; `None` is no limit.
    pub admin_invite_accounts: Option<i32>,
}

pub async fn settings(db: &PgPool) -> anyhow::Result<Settings> {
    let mut conn = db.acquire().await?;
    read_settings(&mut conn, false).await
}

/// The settings; with `lock`, nobody else registers until the transaction ends.
async fn read_settings(conn: &mut PgConnection, lock: bool) -> anyhow::Result<Settings> {
    let settings = if lock {
        sqlx::query_as!(
            Settings,
            "SELECT open, max_accounts, per_day, invites_per_account, invite_days,
                    admin_invite_accounts
             FROM registration FOR UPDATE"
        )
        .fetch_one(conn)
        .await?
    } else {
        sqlx::query_as!(
            Settings,
            "SELECT open, max_accounts, per_day, invites_per_account, invite_days,
                    admin_invite_accounts
             FROM registration"
        )
        .fetch_one(conn)
        .await?
    };
    Ok(settings)
}

pub async fn set_settings(db: &PgPool, settings: &Settings) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE registration
         SET open = $1, max_accounts = $2, per_day = $3, invites_per_account = $4,
             invite_days = $5, admin_invite_accounts = $6",
        settings.open,
        settings.max_accounts,
        settings.per_day,
        settings.invites_per_account,
        settings.invite_days,
        settings.admin_invite_accounts,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Gives an account a quota of invitations of its own, or with `None` the one
/// everybody has. False if there is no account of that name.
pub async fn set_quota(db: &PgPool, name: &str, quota: Option<i32>) -> anyhow::Result<bool> {
    let changed = sqlx::query!(
        "UPDATE accounts SET invite_quota = $2 WHERE lower(username) = lower($1)",
        name.trim(),
        quota,
    )
    .execute(db)
    .await?;
    Ok(changed.rows_affected() > 0)
}

/// How many accounts there are, by the pool they count in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// In the general pool.
    pub general: i64,
    /// Of these, made by registration or a reader's invitation in the last 24 hours.
    pub today: i64,
    /// Made by admins' invitations.
    pub by_admins: i64,
}

pub async fn counts(conn: &mut PgConnection) -> anyhow::Result<Counts> {
    let counts = sqlx::query_as!(
        Counts,
        r#"SELECT count(*) FILTER (WHERE origin <> 'admin_invite') AS "general!",
                  count(*) FILTER (WHERE origin IN ('registration', 'invite')
                                   AND created_at > now() - interval '24 hours') AS "today!",
                  count(*) FILTER (WHERE origin = 'admin_invite') AS "by_admins!"
           FROM accounts"#
    )
    .fetch_one(conn)
    .await?;
    Ok(counts)
}

/// Why nobody can register right now, or not this way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// Registration is not open, and there was no invitation.
    Closed,
    /// The invitation is unknown, used up, run out or taken back.
    Invite,
    /// The pool the account would count in is full.
    Full,
    /// Enough accounts were made in the last 24 hours.
    Today,
}

impl Refused {
    /// The word the answer carries, for the page to say it in its language.
    fn word(self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::Invite => "invite",
            Self::Full => "full",
            Self::Today => "today",
        }
    }
}

impl From<Refused> for AppError {
    fn from(refused: Refused) -> Self {
        AppError::Status(StatusCode::FORBIDDEN, refused.word().into())
    }
}

/// An invitation that can still be used, as far as registering needs it.
struct Usable {
    id: i32,
    by_admin: bool,
}

/// The invitation of this token if it is good for one more account. The link
/// of a disabled account takes nobody in. With `lock`, nobody else uses it
/// until the transaction ends.
async fn usable(
    conn: &mut PgConnection,
    token: &str,
    lock: bool,
) -> anyhow::Result<Option<Usable>> {
    if token.len() > MAX_TOKEN {
        return Ok(None);
    }
    let hash = sessions::token_hash(token);
    let invite = if lock {
        sqlx::query_as!(
            Usable,
            "SELECT id, by_admin FROM invites
             WHERE token_hash = $1 AND NOT revoked AND expires_at > now() AND uses < max_uses
               AND account_id IN (SELECT id FROM accounts WHERE disabled_at IS NULL)
             FOR UPDATE",
            hash,
        )
        .fetch_optional(conn)
        .await?
    } else {
        sqlx::query_as!(
            Usable,
            "SELECT id, by_admin FROM invites
             WHERE token_hash = $1 AND NOT revoked AND expires_at > now() AND uses < max_uses
               AND account_id IN (SELECT id FROM accounts WHERE disabled_at IS NULL)",
            hash,
        )
        .fetch_optional(conn)
        .await?
    };
    Ok(invite)
}

/// Where a new account would come from, and so which pool it counts in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Way {
    Open,
    Invite { id: i32, by_admin: bool },
}

impl Way {
    fn origin(self) -> &'static str {
        match self {
            Self::Open => "registration",
            Self::Invite {
                by_admin: false, ..
            } => "invite",
            Self::Invite { by_admin: true, .. } => "admin_invite",
        }
    }
}

/// Whether one more account may be made, with this invitation or without one.
async fn way_in(
    conn: &mut PgConnection,
    token: Option<&str>,
    lock: bool,
) -> anyhow::Result<Result<Way, Refused>> {
    let settings = read_settings(conn, lock).await?;
    let way = match token {
        None if settings.open => Way::Open,
        None => return Ok(Err(Refused::Closed)),
        Some(token) => match usable(conn, token, lock).await? {
            Some(invite) => Way::Invite {
                id: invite.id,
                by_admin: invite.by_admin,
            },
            None => return Ok(Err(Refused::Invite)),
        },
    };
    let counts = counts(conn).await?;
    let over = |count: i64, limit: Option<i32>| limit.is_some_and(|limit| count >= limit.into());
    let refused = match way {
        Way::Invite { by_admin: true, .. } => {
            over(counts.by_admins, settings.admin_invite_accounts).then_some(Refused::Full)
        }
        _ if over(counts.general, settings.max_accounts) => Some(Refused::Full),
        _ if over(counts.today, settings.per_day) => Some(Refused::Today),
        _ => None,
    };
    Ok(refused.map_or(Ok(way), Err))
}

/// What became of a try to register.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The number of the new account.
    Made(i32),
    Refused(Refused),
    Invalid(accounts::Invalid),
}

/// Makes an account for somebody who registers, with an invitation or, where
/// registration is open, without one.
pub async fn make(
    db: &PgPool,
    name: &str,
    password: &str,
    token: Option<&str>,
) -> anyhow::Result<Outcome> {
    // Asked before the password is hashed, which is work: whoever cannot get
    // in does not make the server do it.
    let mut conn = db.acquire().await?;
    if let Err(refused) = way_in(&mut conn, token, false).await? {
        return Ok(Outcome::Refused(refused));
    }
    drop(conn);
    let name = match accounts::check_name(name) {
        Ok(name) => name,
        Err(invalid) => return Ok(Outcome::Invalid(invalid)),
    };
    if let Err(invalid) = accounts::check_password(password) {
        return Ok(Outcome::Invalid(invalid));
    }
    // Nor for a name that is taken: that says nothing the answer would not.
    // Taken meanwhile, it is refused below all the same.
    if accounts::id_of(db, name).await?.is_some() {
        return Ok(Outcome::Invalid(accounts::Invalid::NameTaken));
    }
    let hash = accounts::hash(password).await?;

    // One registration at a time: counting and adding belong together.
    let mut tx = db.begin().await?;
    let way = match way_in(&mut tx, token, true).await? {
        Ok(way) => way,
        Err(refused) => return Ok(Outcome::Refused(refused)),
    };
    let invite_id = match way {
        Way::Open => None,
        Way::Invite { id, .. } => Some(id),
    };
    let made = sqlx::query_scalar!(
        "INSERT INTO accounts (username, password_hash, origin, invite_id)
         VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING RETURNING id",
        name,
        hash,
        way.origin(),
        invite_id,
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(id) = made else {
        return Ok(Outcome::Invalid(accounts::Invalid::NameTaken));
    };
    if let Some(invite_id) = invite_id {
        sqlx::query!(
            "UPDATE invites SET uses = uses + 1 WHERE id = $1",
            invite_id
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(Outcome::Made(id))
}

/// What the page for registering needs to know before anybody typed a name.
///
/// `GET /api/register` answers with it, to anybody; with `?invite=<token>` it
/// also says whether that invitation can be used.
#[derive(Serialize, TS, ToSchema)]
struct Registering {
    /// Anybody can register, and there is room.
    open: bool,
    /// Whether the invitation that was asked about is good: known, not taken
    /// back, not run out, not used up. Left out if none was asked about.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    invite: Option<bool>,
}

/// The query of `GET /api/register`.
#[derive(Deserialize, TS, ToSchema, IntoParams)]
#[ts(optional_fields)]
#[into_params(parameter_in = Query)]
struct AskedInvite {
    /// The token of an invitation to ask about.
    invite: Option<String>,
}

/// Whether anybody can register now, and whether an invitation link is good,
/// for the page that offers it; for anybody, with or without a session.
#[utoipa::path(
    get,
    path = "/register",
    tag = "Registration",
    params(AskedInvite),
    responses((status = OK, body = Registering)),
)]
async fn registering(
    State(state): State<AppState>,
    Query(asked): Query<AskedInvite>,
) -> Result<Json<Registering>, AppError> {
    let mut conn = state.db.acquire().await?;
    let open = way_in(&mut conn, None, false).await?.is_ok();
    // Whether the link itself is good. Whether there is room for the account
    // it would make is said when somebody tries, as that can change by then.
    let invite = match asked.invite.as_deref() {
        Some(token) => Some(usable(&mut conn, token, false).await?.is_some()),
        None => None,
    };
    Ok(Json(Registering { open, invite }))
}

/// Whether anybody can register without an invitation right now, for the
/// login page.
pub(crate) async fn is_open(db: &PgPool) -> anyhow::Result<bool> {
    let mut conn = db.acquire().await?;
    Ok(way_in(&mut conn, None, false).await?.is_ok())
}

/// What `POST /api/register` reads. It answers with `Me` and sets the
/// session's cookie, as a login does.
///
/// A refusal is 403 with one word: `terms` (the terms of use the server names
/// were not agreed to), `closed` (no invitation, and registration is not open),
/// `invite` (the invitation cannot be used), `full` (no room for another
/// account) or `today` (enough accounts for one day). A name or a
/// password that cannot be one is 400 with the reason, a name that is taken
/// 409.
#[derive(Deserialize, TS, ToSchema)]
struct NewRegistration {
    username: String,
    password: String,
    /// The token of an invitation.
    #[serde(default)]
    #[ts(optional)]
    invite: Option<String>,
    /// The version of the terms of use agreed to, where the server names one.
    #[serde(default)]
    #[ts(optional)]
    accepted_terms: Option<String>,
}

/// Makes an account: through open registration, or with an invitation; the new
/// account is logged in at once.
#[utoipa::path(
    post,
    path = "/register",
    tag = "Registration",
    request_body = NewRegistration,
    responses((status = CREATED, description = "Made and logged in; the answer sets the cookie `__Host-rueder`.", body = crate::login::Me), (status = BAD_REQUEST, description = "A name or password that is not allowed."), (status = FORBIDDEN, description = "Registration is closed, the invitation is not good (any more), the server or the day is full, or the terms were not agreed to (`terms`)."), (status = CONFLICT, description = "The name is taken."), (status = TOO_MANY_REQUESTS, description = "Too many registrations from this sender; see `Retry-After`.")),
)]
async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(new): Json<NewRegistration>,
) -> Result<Response, AppError> {
    // Before anything else: who did not agree gets no account, whatever else.
    let terms = state.legal.terms_version.as_deref();
    if terms.is_some() && new.accepted_terms.as_deref() != terms {
        return Err(AppError::Status(StatusCode::FORBIDDEN, "terms".into()));
    }
    let token = new.invite.as_deref().filter(|token| !token.is_empty());
    let id = match make(&state.db, &new.username, &new.password, token).await? {
        Outcome::Made(id) => id,
        Outcome::Refused(refused) => return Err(refused.into()),
        Outcome::Invalid(accounts::Invalid::NameTaken) => {
            let message = accounts::Invalid::NameTaken.to_string();
            return Err(AppError::Status(StatusCode::CONFLICT, message));
        }
        Outcome::Invalid(invalid) => return Err(AppError::BadRequest(invalid.to_string())),
    };
    // Whoever was logged in in this browser is not any more.
    if let Some(old) = sessions::token(&headers) {
        sessions::end(&state.db, old).await?;
    }
    if let Some(version) = terms {
        legal::agreed(&state.db, id, version).await?;
    }
    let token = sessions::start(&state.db, id, false).await?;
    let me = Me::new(id, new.username.trim().to_owned(), false, true).offering(&state);
    let cookie = [(header::SET_COOKIE, sessions::cookie(&token))];
    Ok((StatusCode::CREATED, cookie, Json(me)).into_response())
}

/// An invitation as the one who made it sees it. The link itself is shown
/// once, when it is made.
#[derive(Serialize, TS, ToSchema)]
struct Invite {
    id: i32,
    /// For how many accounts it is, and how many it brought.
    max_uses: i32,
    uses: i32,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    #[schema(value_type = String)]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    #[schema(value_type = String)]
    expires_at: OffsetDateTime,
    revoked: bool,
    /// It can still be used: not taken back, not run out, not used up.
    usable: bool,
}

/// The reader's invitations and how many more they can make.
///
/// `GET /api/invites` answers with it.
#[derive(Serialize, TS, ToSchema)]
struct Invites {
    /// The newest first.
    invites: Vec<Invite>,
    /// For how many more accounts the reader can make invitations. Left out
    /// for an admin, who has no quota.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    left: Option<i32>,
    /// How many days a new invitation is good for.
    days: i32,
}

/// For how many more accounts a reader can make invitations: their quota less
/// the accounts their invitations brought and can still bring.
async fn left(
    conn: &mut PgConnection,
    account_id: i32,
    settings: &Settings,
) -> anyhow::Result<i32> {
    let row = sqlx::query!(
        r#"SELECT a.invite_quota,
                  coalesce(sum(CASE WHEN i.revoked OR i.expires_at <= now() THEN i.uses
                                    ELSE i.max_uses END), 0) AS "taken!"
           FROM accounts a
           LEFT JOIN invites i ON i.account_id = a.id AND NOT i.by_admin
           WHERE a.id = $1
           GROUP BY a.id"#,
        account_id
    )
    .fetch_one(conn)
    .await?;
    let quota = i64::from(row.invite_quota.unwrap_or(settings.invites_per_account));
    Ok((quota - row.taken).clamp(0, i32::MAX.into()) as i32)
}

/// The invitations the reader made, and how many more they may make.
#[utoipa::path(
    get,
    path = "/invites",
    tag = "Registration",
    security(("session" = [])),
    responses((status = OK, body = Invites), crate::openapi::NeedsSession),
)]
async fn invites(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<Invites>, AppError> {
    let mut conn = state.db.acquire().await?;
    let settings = read_settings(&mut conn, false).await?;
    let invites = sqlx::query_as!(
        Invite,
        r#"SELECT id, max_uses, uses, created_at, expires_at, revoked,
                  (NOT revoked AND expires_at > now() AND uses < max_uses) AS "usable!"
           FROM invites WHERE account_id = $1 ORDER BY created_at DESC, id DESC"#,
        session.account_id
    )
    .fetch_all(&mut *conn)
    .await?;
    let left = match session.is_admin {
        true => None,
        false => Some(left(&mut conn, session.account_id, &settings).await?),
    };
    Ok(Json(Invites {
        invites,
        left,
        days: settings.invite_days,
    }))
}

/// What `POST /api/invites` reads. It answers with `MadeInvite`.
#[derive(Deserialize, TS, ToSchema)]
struct NewInvite {
    /// For how many accounts the link is; one if left out.
    #[serde(default)]
    #[ts(optional)]
    uses: Option<i32>,
}

/// A new invitation, with its token, which is never shown again. The link is
/// the page's address with `?invite=<token>`.
#[derive(Serialize, TS, ToSchema)]
struct MadeInvite {
    id: i32,
    token: String,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    #[schema(value_type = String)]
    expires_at: OffsetDateTime,
}

/// Makes an invitation. An admin's is always made and counts in the admins'
/// pool; a reader's takes from their quota.
#[utoipa::path(
    post,
    path = "/invites",
    tag = "Registration",
    request_body = NewInvite,
    security(("session" = [])),
    responses((status = CREATED, description = "Made; the link is in the answer this once.", body = MadeInvite), crate::openapi::NeedsSession, (status = BAD_REQUEST, description = "More uses than allowed, or too many links."), (status = FORBIDDEN, description = "The reader has no invitations left.")),
)]
async fn invite(
    State(state): State<AppState>,
    session: Session,
    Json(new): Json<NewInvite>,
) -> Result<(StatusCode, Json<MadeInvite>), AppError> {
    let uses = new.uses.unwrap_or(1);
    if !(1..=MAX_USES).contains(&uses) {
        let message = format!("a link is for 1 to {MAX_USES} accounts");
        return Err(AppError::BadRequest(message));
    }
    let mut tx = state.db.begin().await?;
    // One at a time per account, so that two requests cannot both take the
    // last of a quota.
    sqlx::query!(
        "SELECT id FROM accounts WHERE id = $1 FOR UPDATE",
        session.account_id
    )
    .fetch_one(&mut *tx)
    .await?;
    let settings = read_settings(&mut tx, false).await?;
    if !session.is_admin && left(&mut tx, session.account_id, &settings).await? < uses {
        let message = "no invitations left";
        return Err(AppError::Status(StatusCode::FORBIDDEN, message.into()));
    }
    // Old links that can bring nobody any more make room for new ones.
    sqlx::query!(
        "DELETE FROM invites WHERE account_id = $1 AND uses = 0
           AND (revoked OR expires_at <= now())",
        session.account_id
    )
    .execute(&mut *tx)
    .await?;
    let have = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM invites WHERE account_id = $1"#,
        session.account_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if have >= MAX_LINKS {
        return Err(AppError::BadRequest("too many links".into()));
    }

    let mut random = [0u8; 32];
    getrandom::fill(&mut random).map_err(|err| anyhow::anyhow!("no random numbers: {err}"))?;
    let token = URL_SAFE_NO_PAD.encode(random);
    let made = sqlx::query!(
        "INSERT INTO invites (token_hash, account_id, by_admin, max_uses, expires_at)
         VALUES ($1, $2, $3, $4, now() + $5::int4 * interval '1 day')
         RETURNING id, expires_at",
        sessions::token_hash(&token),
        session.account_id,
        session.is_admin,
        uses,
        settings.invite_days,
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    let made = MadeInvite {
        id: made.id,
        token,
        expires_at: made.expires_at,
    };
    Ok((StatusCode::CREATED, Json(made)))
}

/// Takes an invitation back. What it brought so far stays.
#[utoipa::path(
    delete,
    path = "/invites/{id}",
    operation_id = "revoke_invite",
    tag = "Registration",
    params(("id" = i32, Path, description = "The invitation.")),
    security(("session" = [])),
    responses((status = NO_CONTENT, description = "Done."), crate::openapi::NeedsSession, (status = NOT_FOUND, description = "No such invitation of the account.")),
)]
async fn revoke(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i32>,
) -> Result<StatusCode, AppError> {
    let revoked = sqlx::query!(
        "UPDATE invites SET revoked = true WHERE account_id = $1 AND id = $2",
        session.account_id,
        id,
    )
    .execute(&state.db)
    .await?;
    if revoked.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
