//! How long the server keeps what it keeps for a time: set by the operator
//! with `rueder retention set`, shown with `rueder retention show`, and kept in
//! the database, so that a change counts at once, without a restart.
//!
//! So far it is the status of the server ([`crate::history`]). The articles
//! follow with work package 5.3.

use sqlx::PgPool;

/// The fewest and the most days anything is kept for.
pub const DAYS: std::ops::RangeInclusive<i32> = 1..=3650;

/// What the operator set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    /// How many days the counts of the status and the problems of the log are
    /// kept for.
    pub status_days: i32,
}

pub async fn settings(db: &PgPool) -> anyhow::Result<Settings> {
    let settings = sqlx::query_as!(Settings, "SELECT status_days FROM retention")
        .fetch_one(db)
        .await?;
    Ok(settings)
}

pub async fn set(db: &PgPool, settings: &Settings) -> anyhow::Result<()> {
    anyhow::ensure!(
        DAYS.contains(&settings.status_days),
        "the status is kept for {} to {} days",
        DAYS.start(),
        DAYS.end()
    );
    sqlx::query!(
        "UPDATE retention SET status_days = $1",
        settings.status_days
    )
    .execute(db)
    .await?;
    Ok(())
}
