//! Accounts nobody uses (E13). Where the operator sets it with `rueder
//! retention set --inactive`, an account goes after so many months without
//! being used, after two mails to its confirmed address. As rueder comes,
//! every account stays.
//!
//! - **Used** is a login, a request with a session or an app logging in
//!   ([`used`]), written down at most once a day in `accounts.active_at`.
//!   Using an account again takes back the mails it had.
//! - **Two mails**, [`FIRST_DAYS`] and [`SECOND_DAYS`] before the account
//!   goes, each naming the day. It goes when its months are over, but never
//!   sooner than those days after each mail actually went: where the day's
//!   mail ran out, an account goes later, never with less warning.
//! - **Never** an account without a confirmed address, which no mail reaches;
//!   an admin, so that an instance does not lose the one it has; or a
//!   disabled one, about which the operator has decided already.
//! - **Only the running server** looks after it, once an hour ([`sweep`]),
//!   as the mails count against its limits (`rate::Rates::inactive_mail`).

use std::{sync::Arc, time::Instant};

use anyhow::Context;
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::{
    mail::{Language, Mailer, Template},
    rate::Rates,
    retention, sizes,
};

/// How many days before an account goes the first mail goes.
pub const FIRST_DAYS: i32 = 30;
/// How many days before it goes the second mail goes.
pub const SECOND_DAYS: i32 = 7;
/// How long after it was written down that an account was used it is written
/// down again, in seconds: not every request needs a write.
const TOUCH_AFTER: f64 = 24.0 * 60.0 * 60.0;
/// How many accounts one run mails or removes at most, of each.
const PORTION: i64 = 500;

/// Notes that the account was used, at most once a day, and takes back the
/// mails it had about going.
pub async fn used(db: &PgPool, account_id: i32) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE accounts
         SET active_at = now(), inactive_warned_at = NULL, inactive_reminded_at = NULL
         WHERE id = $1 AND active_at < now() - $2::float8 * interval '1 second'",
        account_id,
        TOUCH_AFTER,
    )
    .execute(db)
    .await
    .context("noting that an account was used")?;
    Ok(())
}

/// After how many months without being used an account goes; `None` keeps
/// every account, which is what rueder comes with.
pub async fn months(db: &PgPool) -> anyhow::Result<Option<i32>> {
    let months = sqlx::query_scalar!("SELECT inactive_months FROM retention")
        .fetch_one(db)
        .await?;
    Ok(months)
}

/// Sets after how many months without being used an account goes, within
/// [`retention::MONTHS`]; `None` keeps every account.
pub async fn set_months(db: &PgPool, months: Option<i32>) -> anyhow::Result<()> {
    if let Some(months) = months {
        let what = "the months of accounts nobody uses";
        sizes::in_bounds(what, months, &retention::MONTHS).map_err(anyhow::Error::msg)?;
    }
    sqlx::query!("UPDATE retention SET inactive_months = $1", months)
        .execute(db)
        .await?;
    Ok(())
}

/// How the accounts stand, for `rueder retention show`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Standing {
    /// Accounts that had the first mail and are not used since.
    pub warned: i64,
    /// Accounts not used for the months that no mail reaches, as they have no
    /// confirmed address: they stay.
    pub without_address: i64,
}

/// How the accounts stand at `now` with these months.
pub async fn standing(db: &PgPool, months: i32, now: OffsetDateTime) -> anyhow::Result<Standing> {
    let row = sqlx::query!(
        r#"SELECT count(*) FILTER (WHERE inactive_warned_at IS NOT NULL) AS "warned!",
                  count(*) FILTER (WHERE email IS NULL AND NOT is_admin AND disabled_at IS NULL
                                     AND active_at + make_interval(months => $2) <= $1)
                      AS "without_address!"
           FROM accounts"#,
        now,
        months,
    )
    .fetch_one(db)
    .await?;
    Ok(Standing {
        warned: row.warned,
        without_address: row.without_address,
    })
}

/// What a run did.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Swept {
    /// Accounts that got the first mail.
    pub warned: u64,
    /// Accounts that got the second.
    pub reminded: u64,
    /// Accounts that went.
    pub removed: u64,
    /// Mails wait for the next run: the day's share ran out, or the relay
    /// did not take one.
    pub waiting: bool,
}

impl Swept {
    pub fn is_empty(&self) -> bool {
        self.warned == 0 && self.reminded == 0 && self.removed == 0 && !self.waiting
    }
}

/// A mail about an account going, and to whom.
struct Notice {
    id: i32,
    username: String,
    email: String,
    language: Language,
    active_at: OffsetDateTime,
    goes: OffsetDateTime,
}

/// What the server does once an hour: [`sweep`] now, and say in the log what
/// it did.
pub async fn hourly(db: PgPool, mailer: Arc<Mailer>, rates: Arc<Rates>) {
    match sweep(&db, &mailer, &rates, OffsetDateTime::now_utc()).await {
        Ok(swept) if !swept.is_empty() => tracing::info!(
            warned = swept.warned,
            reminded = swept.reminded,
            removed = swept.removed,
            waiting = swept.waiting,
            "accounts nobody uses"
        ),
        Ok(_) => {}
        Err(err) => tracing::error!("looking after accounts nobody uses: {err:#}"),
    }
}

/// Mails the accounts nobody used for the months the operator set, and
/// removes those whose time is over, as it stands at `now`.
pub async fn sweep(
    db: &PgPool,
    mailer: &Mailer,
    rates: &Rates,
    now: OffsetDateTime,
) -> anyhow::Result<Swept> {
    let mut swept = Swept::default();
    let Some(months) = months(db).await? else {
        return Ok(swept);
    };

    // Who goes first: removing frees no mail, so it comes before the mails,
    // which may run out.
    let removed = sqlx::query!(
        "DELETE FROM accounts WHERE id IN (
             SELECT id FROM accounts
             WHERE inactive_reminded_at IS NOT NULL
               AND inactive_may_go(accounts)
               AND greatest(active_at + make_interval(months => $2),
                            inactive_warned_at + make_interval(days => $3),
                            inactive_reminded_at + make_interval(days => $4)) <= $1
             LIMIT $5)",
        now,
        months,
        FIRST_DAYS,
        SECOND_DAYS,
        PORTION,
    )
    .execute(db)
    .await
    .context("removing accounts nobody uses")?;
    swept.removed = removed.rows_affected();

    let first = sqlx::query_as!(
        Notice,
        r#"SELECT id, username, email AS "email!",
                  coalesce(language, 'en') AS "language!: Language", active_at,
                  greatest(active_at + make_interval(months => $2),
                           $1::timestamptz + make_interval(days => $3)) AS "goes!"
           FROM accounts
           WHERE inactive_warned_at IS NULL
             AND inactive_may_go(accounts)
             AND active_at + make_interval(months => $2) - make_interval(days => $3) <= $1
           ORDER BY active_at, id
           LIMIT $4"#,
        now,
        months,
        FIRST_DAYS,
        PORTION,
    )
    .fetch_all(db)
    .await?;
    for notice in &first {
        if !mail(mailer, rates, notice, Template::Inactive, months).await {
            swept.waiting = true;
            break;
        }
        // Not if the account was used meanwhile.
        sqlx::query!(
            "UPDATE accounts SET inactive_warned_at = $2
             WHERE id = $1 AND active_at = $3 AND inactive_warned_at IS NULL",
            notice.id,
            now,
            notice.active_at,
        )
        .execute(db)
        .await?;
        swept.warned += 1;
    }
    if swept.waiting {
        return Ok(swept);
    }

    let second = sqlx::query_as!(
        Notice,
        r#"SELECT id, username, email AS "email!",
                  coalesce(language, 'en') AS "language!: Language", active_at,
                  greatest(active_at + make_interval(months => $2),
                           inactive_warned_at + make_interval(days => $3),
                           $1::timestamptz + make_interval(days => $4)) AS "goes!"
           FROM accounts
           WHERE inactive_warned_at IS NOT NULL AND inactive_reminded_at IS NULL
             AND inactive_may_go(accounts)
             AND greatest(active_at + make_interval(months => $2),
                          inactive_warned_at + make_interval(days => $3))
                 - make_interval(days => $4) <= $1
           ORDER BY active_at, id
           LIMIT $5"#,
        now,
        months,
        FIRST_DAYS,
        SECOND_DAYS,
        PORTION,
    )
    .fetch_all(db)
    .await?;
    for notice in &second {
        if !mail(mailer, rates, notice, Template::InactiveSoon, months).await {
            swept.waiting = true;
            break;
        }
        sqlx::query!(
            "UPDATE accounts SET inactive_reminded_at = $2
             WHERE id = $1 AND active_at = $3 AND inactive_reminded_at IS NULL",
            notice.id,
            now,
            notice.active_at,
        )
        .execute(db)
        .await?;
        swept.reminded += 1;
    }
    Ok(swept)
}

/// Sends the mail, if the day's share has room and the relay takes it.
async fn mail(
    mailer: &Mailer,
    rates: &Rates,
    notice: &Notice,
    template: Template,
    months: i32,
) -> bool {
    if rates
        .inactive_mail(notice.id, &notice.email, Instant::now())
        .is_err()
    {
        return false;
    }
    let (site, link) = (mailer.site(), mailer.home());
    let (date, months) = (notice.goes.date().to_string(), months.to_string());
    let values = [
        ("name", notice.username.as_str()),
        ("site", site.as_str()),
        ("link", link.as_str()),
        ("date", date.as_str()),
        ("months", months.as_str()),
    ];
    match mailer
        .send(&notice.email, notice.language, template, &values)
        .await
    {
        Ok(()) => true,
        Err(err) => {
            tracing::warn!("a mail about an account nobody uses did not go: {err:#}");
            false
        }
    }
}
