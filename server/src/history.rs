//! What the server counts about itself, kept across restarts.
//!
//! The [`Monitor`] counts in memory, in parts of ten minutes. Every minute,
//! and once more when the server stops, the parts that were counted in since
//! the last time go to `status_slots`, and the list of problems of the log to
//! `status_problems` when it changed ([`save`]). On the start, the parts of the
//! last hour and the problems come back ([`load`]), so that the status page
//! goes on where it was. For more than the last hour, the status page counts
//! from the database ([`period`]). What is older than the operator set with
//! `rueder retention set --status-days` goes once an hour ([`sweep`]).
//!
//! The counts hold no address, no name and no text of anybody's: requests are
//! counted by their route, and the problems are what the server itself wrote to
//! its log, as on the status page.

use std::{collections::HashMap, time::Duration, time::Instant};

use futures_util::TryStreamExt;
use serde::{Serialize, de::DeserializeOwned};
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::{
    images::ImageProxy,
    monitor::{
        self, Fetches, Images, Lookups, MAX_ROUTES, Monitor, Problem, ProblemCount, Problems,
        Requests, SLOTS, Severity, Tally,
    },
    retention,
};

/// How often the counts are saved while the server runs.
pub const EVERY: Duration = Duration::from_secs(60);
/// The longest route that is saved. The routes the code defines are far
/// shorter; the table holds no longer one.
const MAX_ROUTE: usize = 300;

/// The kinds of counts, as the table names them.
const REQUESTS: &str = "requests";
const FETCHES: &str = "fetches";
const LOOKUPS: &str = "lookups";
const IMAGES: &str = "images";
const PROBLEMS: &str = "problems";

/// Rows for `status_slots`, one array for each column.
#[derive(Default)]
struct Rows {
    starts: Vec<OffsetDateTime>,
    kinds: Vec<&'static str>,
    routes: Vec<String>,
    counts: Vec<serde_json::Value>,
}

impl Rows {
    fn push(
        &mut self,
        number: u64,
        kind: &'static str,
        route: &str,
        counts: &impl Serialize,
    ) -> anyhow::Result<()> {
        self.starts.push(monitor::slot_start(number));
        self.kinds.push(kind);
        self.routes.push(route.to_owned());
        self.counts.push(serde_json::to_value(counts)?);
        Ok(())
    }

    fn all<T: Serialize>(&mut self, kind: &'static str, parts: &[(u64, T)]) -> anyhow::Result<()> {
        for (number, counts) in parts {
            self.push(*number, kind, "", counts)?;
        }
        Ok(())
    }
}

/// Saves what was counted since the last time. Each part goes as a whole, so
/// that a part that failed to go is saved with the next count in it.
pub async fn save(
    db: &PgPool,
    monitor: &Monitor,
    images: Option<&ImageProxy>,
) -> anyhow::Result<()> {
    let changed = monitor.take_changed();
    let mut rows = Rows::default();
    for (number, route, requests) in &changed.requests {
        if route.len() <= MAX_ROUTE {
            rows.push(*number, REQUESTS, route, requests)?;
        }
    }
    rows.all(FETCHES, &changed.fetches)?;
    rows.all(LOOKUPS, &changed.lookups)?;
    rows.all(PROBLEMS, &changed.problems)?;
    if let Some(images) = images {
        rows.all(IMAGES, &images.take_changed())?;
    }
    if !rows.starts.is_empty() {
        let kinds: Vec<String> = rows.kinds.iter().map(|&kind| kind.to_owned()).collect();
        sqlx::query!(
            r#"INSERT INTO status_slots (starts_at, kind, route, counts)
               SELECT * FROM UNNEST($1::timestamptz[], $2::text[], $3::text[], $4::jsonb[])
               ON CONFLICT (starts_at, kind, route) DO UPDATE SET counts = EXCLUDED.counts"#,
            &rows.starts,
            &kinds,
            &rows.routes,
            &rows.counts,
        )
        .execute(db)
        .await?;
    }

    if let Some(problems) = monitor.problems.take_changed() {
        let mut tx = db.begin().await?;
        sqlx::query!("DELETE FROM status_problems")
            .execute(&mut *tx)
            .await?;
        let positions: Vec<i32> = (0..).take(problems.len()).collect();
        let errors: Vec<bool> = problems
            .iter()
            .map(|problem| problem.severity == Severity::Error)
            .collect();
        let targets: Vec<String> = problems.iter().map(|p| p.target.clone()).collect();
        let texts: Vec<String> = problems.iter().map(|p| p.text.clone()).collect();
        let counts: Vec<i64> = problems
            .iter()
            .map(|p| i64::try_from(p.count).unwrap_or(i64::MAX))
            .collect();
        let firsts: Vec<OffsetDateTime> = problems.iter().map(|p| p.first).collect();
        let lasts: Vec<OffsetDateTime> = problems.iter().map(|p| p.last).collect();
        sqlx::query!(
            r#"INSERT INTO status_problems (position, error, target, text, count, first_at, last_at)
               SELECT * FROM UNNEST($1::int4[], $2::bool[], $3::text[], $4::text[], $5::int8[],
                                    $6::timestamptz[], $7::timestamptz[])"#,
            &positions,
            &errors,
            &targets,
            &texts,
            &counts,
            &firsts,
            &lasts,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
    }
    Ok(())
}

/// The counts of a row, or None where they are not what the server writes.
fn parsed<T: DeserializeOwned>(counts: &str) -> Option<T> {
    serde_json::from_str(counts).ok()
}

/// Puts back what was saved of the last hour, and the problems of the log.
/// For the start of the server, before it counts much.
pub async fn load(
    db: &PgPool,
    monitor: &Monitor,
    images: Option<&ImageProxy>,
) -> anyhow::Result<()> {
    let now = Instant::now();
    let first = monitor::slot(now).saturating_sub(SLOTS as u64 - 1);
    let rows = sqlx::query!(
        r#"SELECT starts_at, kind, route, counts::text AS "counts!"
           FROM status_slots WHERE starts_at >= $1"#,
        monitor::slot_start(first),
    )
    .fetch_all(db)
    .await?;
    for row in rows {
        let number = monitor::slot_at(row.starts_at);
        let counts = row.counts.as_str();
        match row.kind.as_str() {
            REQUESTS => {
                if let Some(requests) = parsed(counts) {
                    monitor.restore_requests(number, &row.route, &requests, now);
                }
            }
            FETCHES => {
                if let Some(fetches) = parsed(counts) {
                    monitor.restore_fetches(number, &fetches, now);
                }
            }
            LOOKUPS => {
                if let Some(lookups) = parsed(counts) {
                    monitor.restore_lookups(number, &lookups, now);
                }
            }
            PROBLEMS => {
                if let Some(problems) = parsed(counts) {
                    monitor.problems.restore_counts(number, &problems, now);
                }
            }
            IMAGES => {
                if let (Some(images), Some(served)) = (images, parsed(counts)) {
                    images.restore(number, &served, now);
                }
            }
            _ => {}
        }
    }

    let problems = sqlx::query!(
        "SELECT error, target, text, count, first_at, last_at
         FROM status_problems ORDER BY position LIMIT $1",
        monitor::MAX_PROBLEMS as i64,
    )
    .fetch_all(db)
    .await?;
    monitor.problems.restore(
        problems
            .into_iter()
            .map(|row| Problem {
                severity: if row.error {
                    Severity::Error
                } else {
                    Severity::Warning
                },
                target: row.target,
                text: row.text,
                count: u64::try_from(row.count).unwrap_or(1),
                first: row.first_at,
                last: row.last_at,
            })
            .collect(),
    );
    Ok(())
}

/// What was counted over a time longer than the hour in memory.
#[derive(Default, Debug)]
pub struct Period {
    pub requests: HashMap<String, Requests>,
    pub fetches: Fetches,
    pub lookups: Lookups,
    pub images: Images,
    pub problems: ProblemCount,
}

/// What was counted in the parts from the one with the number `first` on, as
/// saved. Save first to have what is in memory, too.
pub async fn period(db: &PgPool, first: u64) -> anyhow::Result<Period> {
    let mut period = Period::default();
    let mut rows = sqlx::query!(
        r#"SELECT kind, route, counts::text AS "counts!"
           FROM status_slots WHERE starts_at >= $1"#,
        monitor::slot_start(first),
    )
    .fetch(db);
    while let Some(row) = rows.try_next().await? {
        let counts = row.counts.as_str();
        match row.kind.as_str() {
            REQUESTS => {
                let Some(requests) = parsed::<Requests>(counts) else {
                    continue;
                };
                let known = period.requests.len();
                let route = if period.requests.contains_key(&row.route) || known < MAX_ROUTES {
                    row.route
                } else {
                    "other".to_owned()
                };
                period.requests.entry(route).or_default().add(&requests);
            }
            FETCHES => add(&mut period.fetches, counts),
            LOOKUPS => add(&mut period.lookups, counts),
            IMAGES => add(&mut period.images, counts),
            PROBLEMS => add(&mut period.problems, counts),
            _ => {}
        }
    }
    Ok(period)
}

fn add<T: Tally>(to: &mut T, counts: &str) {
    if let Some(counts) = parsed::<T>(counts) {
        to.add(&counts);
    }
}

/// Removes what is older than the operator keeps the status for, here and in
/// memory, and says how many parts went.
pub async fn sweep(db: &PgPool, problems: &Problems) -> anyhow::Result<u64> {
    let days = retention::settings(db).await?.status_days;
    let cutoff = OffsetDateTime::now_utc() - time::Duration::days(days.into());
    problems.forget_before(cutoff);
    let slots = sqlx::query!("DELETE FROM status_slots WHERE starts_at < $1", cutoff)
        .execute(db)
        .await?;
    sqlx::query!("DELETE FROM status_problems WHERE last_at < $1", cutoff)
        .execute(db)
        .await?;
    Ok(slots.rows_affected())
}
