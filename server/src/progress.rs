//! Lines in the log that say how far a long run has come: the migrations at
//! the start, the import of the old database and cleaning again. One every
//! [`EVERY`] at most, so that whoever watches the log sees what is being done
//! without being flooded, and none for a run that is over before.

use std::time::{Duration, Instant};

use anyhow::Context;
use sqlx::PgPool;

/// How often a long run says how far it has come.
pub const EVERY: Duration = Duration::from_secs(30);

/// How far a run through a known number of things has come.
pub struct Progress {
    what: &'static str,
    total: u64,
    done: u64,
    started: Instant,
    said: Instant,
}

impl Progress {
    /// A run through `total` things; `what` says what is done with them.
    pub fn new(what: &'static str, total: u64) -> Self {
        let now = Instant::now();
        Self {
            what,
            total,
            done: 0,
            started: now,
            said: now,
        }
    }

    /// Counts `n` more as done, and says how far the run is if it last did
    /// [`EVERY`] ago.
    pub fn add(&mut self, n: u64) {
        self.done += n;
        if self.said.elapsed() >= EVERY {
            self.said = Instant::now();
            tracing::info!(
                "{}",
                line(self.what, self.done, self.total, self.started.elapsed())
            );
        }
    }
}

/// "articles brought: 120000 of 700000 (17 %) after 95 s, about 460 s left"
fn line(what: &str, done: u64, total: u64, elapsed: Duration) -> String {
    let percent = (done * 100).checked_div(total).unwrap_or(100);
    let mut line = format!(
        "{what}: {done} of {total} ({percent} %) after {} s",
        elapsed.as_secs()
    );
    if done > 0 && done < total {
        let left = elapsed.as_secs_f64() * (total - done) as f64 / done as f64;
        line += &format!(", about {left:.0} s left");
    }
    line
}

/// Runs the migrations. One that rewrites a large table can take minutes, so
/// while they run, the log says every [`EVERY`] which one it is and how far:
/// for an index being built, what PostgreSQL tells of it, and otherwise how
/// much the database has grown, as rewriting a table writes it anew.
pub async fn migrate(db: &PgPool) -> anyhow::Result<()> {
    let grown_from = database_size(db).await?;
    let watching = tokio::spawn({
        let db = db.clone();
        let started = tokio::time::Instant::now();
        async move {
            let mut every = tokio::time::interval_at(started + EVERY, EVERY);
            loop {
                every.tick().await;
                match migrating(&db, grown_from).await {
                    Ok(how_far) => {
                        tracing::info!("migrating for {} s: {how_far}", started.elapsed().as_secs())
                    }
                    Err(err) => tracing::warn!("cannot tell how far the migrations are: {err:#}"),
                }
            }
        }
    });
    let ran = sqlx::migrate!().run(db).await;
    watching.abort();
    ran.context("running migrations")
}

async fn database_size(db: &PgPool) -> anyhow::Result<i64> {
    Ok(
        sqlx::query_scalar!(r#"SELECT pg_database_size(current_database()) AS "size!""#)
            .fetch_one(db)
            .await?,
    )
}

/// Which migration is running and how far it is, with `grown_from` the size
/// of the database before the migrations.
pub async fn migrating(db: &PgPool, grown_from: i64) -> anyhow::Result<String> {
    let now = sqlx::query!(
        r#"SELECT (SELECT max(version) FROM _sqlx_migrations) AS done,
                  pg_database_size(current_database()) AS "size!",
                  (SELECT p.phase || coalesce(', ' || (100 * p.blocks_done / nullif(p.blocks_total, 0)) || ' %', '')
                   FROM pg_stat_progress_create_index p
                   WHERE p.datname = current_database() LIMIT 1) AS index"#
    )
    .fetch_one(db)
    .await?;
    let migrator = sqlx::migrate!();
    let which = migrator
        .iter()
        .find(|migration| Some(migration.version) > now.done)
        .map_or_else(
            || "the last migration".to_owned(),
            |migration| {
                format!(
                    "migration {} ({})",
                    migration.version, migration.description
                )
            },
        );
    Ok(match now.index {
        // PostgreSQL's own words, e.g. "building index: scanning table, 40 %".
        Some(index) => format!("{which}: {index}"),
        None => format!(
            "{which}, the database has grown by {} MB",
            (now.size - grown_from) / (1024 * 1024)
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn says_how_far_and_how_long_still() {
        let line = |done, total| line("articles brought", done, total, Duration::from_secs(60));
        assert_eq!(line(0, 400), "articles brought: 0 of 400 (0 %) after 60 s");
        assert_eq!(
            line(100, 400),
            "articles brought: 100 of 400 (25 %) after 60 s, about 180 s left"
        );
        assert_eq!(
            line(400, 400),
            "articles brought: 400 of 400 (100 %) after 60 s"
        );
        // Nothing to do, or more done than there seemed to be.
        assert_eq!(line(0, 0), "articles brought: 0 of 0 (100 %) after 60 s");
        assert_eq!(
            line(410, 400),
            "articles brought: 410 of 400 (102 %) after 60 s"
        );
    }
}
