//! Fetching feeds: one feed ([`fetch_feed`]), everything that is due
//! ([`fetch_due`]) and the loop the server runs ([`run`]).
//!
//! A feed is fetched with a conditional request, so a feed that did not change
//! costs its site next to nothing. What comes back goes through
//! [`crate::feeds`]; the feed's row records how the fetch went and when the next
//! one is due. A failure is written to the feed, where the interface shows it,
//! and makes the next try come later each time.

use std::{
    collections::HashMap,
    ops::RangeInclusive,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Context;
use reqwest::{StatusCode, Url};
use sqlx::PgPool;
use time::OffsetDateTime;
use tokio::{sync::Semaphore, task::JoinSet};

use crate::{
    cleaning::Rules,
    events::{Change, Events},
    feeds::{self, Stored},
    filters,
    http_client::{Agent, HttpClient, Request},
    monitor::{Fetched, Monitor, Round},
};

/// The shortest and the longest time between two fetches of a feed, in seconds.
pub const MIN_INTERVAL: i32 = 15 * 60;
pub const MAX_INTERVAL: i32 = 12 * 60 * 60;
/// How fast feeds are fetched: how many one round takes, how many are fetched
/// at the same time, and how often a round starts. Together they cap how many
/// fetches an hour has, [`Pace::per_hour`]: 12,000 as rueder comes. A large
/// instance sets them with `FETCH_ROUND`, `FETCH_AT_ONCE` and `FETCH_EVERY`
/// (seconds); the server checks them against [`ROUNDS`], [`AT_ONCE`] and
/// [`EVERY`] when it starts. Feeds of one host are fetched one after the other
/// whatever the pace says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pace {
    pub round: i64,
    pub at_once: usize,
    pub every: Duration,
}

/// The sizes of a round a server may be given.
pub const ROUNDS: RangeInclusive<i64> = 1..=10_000;
/// How many fetches at once a server may be given.
pub const AT_ONCE: RangeInclusive<usize> = 1..=256;
/// How many seconds between two rounds a server may be given.
pub const EVERY: RangeInclusive<u64> = 5..=3600;

impl Default for Pace {
    fn default() -> Self {
        Self {
            round: 100,
            at_once: 8,
            every: Duration::from_secs(30),
        }
    }
}

impl Pace {
    /// The pace the environment sets, with the default for what it leaves out.
    pub fn from_env() -> anyhow::Result<Self> {
        Self::from_vars(|name| std::env::var(name).ok())
    }

    /// The pace `var` gives, with the default for what it leaves out. A value
    /// that is no number or out of bounds is an error, not quietly the default.
    pub fn from_vars(var: impl Fn(&str) -> Option<String>) -> anyhow::Result<Self> {
        fn within<T: std::str::FromStr + PartialOrd + std::fmt::Display>(
            name: &str,
            value: Option<String>,
            bounds: &RangeInclusive<T>,
        ) -> anyhow::Result<Option<T>> {
            let Some(value) = value else {
                return Ok(None);
            };
            let number: T = value
                .trim()
                .parse()
                .map_err(|_| anyhow::anyhow!("{name} is no number: {value:?}"))?;
            if !bounds.contains(&number) {
                anyhow::bail!(
                    "{name} must be from {} to {}, not {number}",
                    bounds.start(),
                    bounds.end()
                );
            }
            Ok(Some(number))
        }
        let pace = Self::default();
        Ok(Self {
            round: within("FETCH_ROUND", var("FETCH_ROUND"), &ROUNDS)?.unwrap_or(pace.round),
            at_once: within("FETCH_AT_ONCE", var("FETCH_AT_ONCE"), &AT_ONCE)?
                .unwrap_or(pace.at_once),
            every: within("FETCH_EVERY", var("FETCH_EVERY"), &EVERY)?
                .map_or(pace.every, Duration::from_secs),
        })
    }

    /// How many fetches an hour can have at most at this pace, if every round
    /// is over before the next one starts.
    pub fn per_hour(&self) -> u64 {
        let rounds = 3600.0 / self.every.as_secs_f64();
        (self.round as f64 * rounds).round() as u64
    }
}
/// How long a round's feeds are kept from other rounds, in case this one dies.
const LEASE_S: f64 = 600.0;
/// What rueder would like to get.
const ACCEPT: &str = "application/atom+xml, application/rss+xml, application/feed+json, \
                      application/xml;q=0.9, text/xml;q=0.9, */*;q=0.5";
/// The longest error text kept at a feed, in characters.
const MAX_MESSAGE: usize = 300;

/// How a fetch went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The site says the feed is as it was.
    NotModified,
    Fetched(Stored),
    /// The reason, as it stands at the feed now.
    Failed(String),
}

/// How often a feed is fetched, by how much it publishes: the rule of the old
/// rueder. `None` in a feed's first day, when there is nothing to go by yet.
pub fn interval(articles: i32, age: time::Duration) -> Option<i32> {
    let days = (age.as_seconds_f64() / 86400.0).round();
    if age < time::Duration::days(1) || days < 1.0 {
        return None;
    }
    let per_day = (f64::from(articles) / days).round();
    Some(match per_day {
        _ if articles == 0 => MAX_INTERVAL,
        15.0.. => MIN_INTERVAL,
        8.0.. => 2 * MIN_INTERVAL,
        5.0.. => 3 * MIN_INTERVAL,
        _ => 4 * MIN_INTERVAL,
    })
}

/// A few seconds that differ from feed to feed, so feeds with the same interval
/// do not all come due in the same moment.
fn spread(feed_id: i32) -> f64 {
    f64::from(feed_id.rem_euclid(120))
}

/// Fetches one feed and writes down how it went. An error from here means the
/// database failed; a feed that cannot be fetched is an [`Outcome::Failed`].
pub async fn fetch_feed(
    db: &PgPool,
    client: &HttpClient,
    rules: &Arc<Rules>,
    feed_id: i32,
) -> anyhow::Result<Outcome> {
    let feed = sqlx::query!(
        "SELECT feed_url, fetch_agent, etag, last_modified FROM feeds WHERE id = $1",
        feed_id
    )
    .fetch_optional(db)
    .await?
    .with_context(|| format!("no feed {feed_id}"))?;

    let request = Request {
        agent: if feed.fetch_agent == "browser" {
            Agent::Browser
        } else {
            Agent::Rueder
        },
        accept: Some(ACCEPT),
        etag: feed.etag.as_deref(),
        last_modified: feed.last_modified.as_deref(),
        ..Request::new(&feed.feed_url)
    };
    let response = match client.get(request).await {
        Ok(response) => response,
        Err(err) => return failed(db, feed_id, err.to_string()).await,
    };
    let now = OffsetDateTime::now_utc();
    if response.status == StatusCode::NOT_MODIFIED {
        sqlx::query!(
            "UPDATE feeds
             SET fetched_at = $2, last_success_at = $2, error_message = NULL,
                 next_fetch_at = $2::timestamptz + (fetch_interval_s + $3::float8) * interval '1 second'
             WHERE id = $1",
            feed_id,
            now,
            spread(feed_id),
        )
        .execute(db)
        .await?;
        return Ok(Outcome::NotModified);
    }
    if !response.status.is_success() {
        return failed(
            db,
            feed_id,
            format!("the site answers with {}", response.status),
        )
        .await;
    }
    // Reading and cleaning a large feed takes a while, so it runs where it
    // keeps nothing else waiting.
    let reading = {
        let (rules, body, url) = (rules.clone(), response.body.clone(), response.url.clone());
        tokio::task::spawn_blocking(move || feeds::parse(&body, url.as_str(), now, &rules))
    };
    let parsed = match reading.await {
        Ok(Ok(parsed)) => parsed,
        Ok(Err(err)) => return failed(db, feed_id, format!("{err:#}")).await,
        Err(_) => return failed(db, feed_id, "the feed could not be read".into()).await,
    };

    let mut tx = db.begin().await?;
    let stored = feeds::store(&mut tx, feed_id, &parsed.articles).await?;
    // The readers' filter rules see new articles before the readers do.
    if let Some(first) = stored.first_new {
        filters::apply_new(&mut tx, feed_id, first).await?;
    }
    // What a reader who moved here had read elsewhere is read here too.
    crate::reading::settle_read_before(&mut tx, feed_id).await?;
    let state = sqlx::query!(
        "SELECT last_seq, created_at, fetch_interval_s FROM feeds WHERE id = $1",
        feed_id
    )
    .fetch_one(&mut *tx)
    .await?;
    // A failure made the interval longer; the first success ends that.
    let interval = interval(state.last_seq, now - state.created_at)
        .unwrap_or(state.fetch_interval_s.min(4 * MIN_INTERVAL));
    sqlx::query!(
        "UPDATE feeds
         SET title = coalesce($2, title), site_url = coalesce($3, site_url),
             icon_url = coalesce($4, icon_url), etag = $5, last_modified = $6,
             fetched_at = $7, last_success_at = $7, error_message = NULL,
             fetch_interval_s = $8, next_fetch_at = $7::timestamptz + ($8::integer + $9::float8) * interval '1 second'
         WHERE id = $1",
        feed_id,
        parsed.title,
        parsed.site_url,
        parsed.icon_url,
        response.header("etag"),
        response.header("last-modified"),
        now,
        interval,
        spread(feed_id),
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Outcome::Fetched(stored))
}

/// Writes a failure to the feed and doubles the time to the next try, up to
/// the longest interval.
async fn failed(db: &PgPool, feed_id: i32, message: String) -> anyhow::Result<Outcome> {
    let message: String = message.chars().take(MAX_MESSAGE).collect();
    sqlx::query!(
        "UPDATE feeds
         SET fetched_at = now(), last_error_at = now(), error_message = $2,
             fetch_interval_s = least(fetch_interval_s * 2, $3),
             next_fetch_at = now() + (least(fetch_interval_s * 2, $3) + $4::float8) * interval '1 second'
         WHERE id = $1",
        feed_id,
        message,
        MAX_INTERVAL,
        spread(feed_id),
    )
    .execute(db)
    .await?;
    Ok(Outcome::Failed(message))
}

/// Fetches the feeds that are due and have somebody reading them, and returns
/// how many that were: at most a round's worth of the pace, so many at a time
/// as it says, and never two from the same host at once. Each fetch and the
/// round count for the status page.
pub async fn fetch_due(
    db: &PgPool,
    client: &HttpClient,
    rules: &Arc<Rules>,
    events: &Events,
    monitor: &Arc<Monitor>,
    pace: &Pace,
) -> anyhow::Result<usize> {
    let (started, began) = (Instant::now(), OffsetDateTime::now_utc());
    // Taking a feed moves its next fetch away, so a second server or a round
    // that overlaps does not take it too. The fetch sets the real time.
    let due = sqlx::query!(
        r#"UPDATE feeds SET next_fetch_at = now() + $2::float8 * interval '1 second'
         WHERE id IN (
             SELECT f.id FROM feeds f
             WHERE f.next_fetch_at <= now() AND f.blocked_at IS NULL
               AND EXISTS (SELECT 1 FROM subscriptions s WHERE s.feed_id = f.id)
             ORDER BY f.next_fetch_at
             LIMIT $1
             FOR UPDATE SKIP LOCKED
         )
         RETURNING id, feed_url, error_message IS NOT NULL AS "failing!""#,
        pace.round,
        LEASE_S,
    )
    .fetch_all(db)
    .await?;
    let count = due.len();

    let mut hosts: HashMap<String, Vec<(i32, bool)>> = HashMap::new();
    for feed in due {
        let host = Url::parse(&feed.feed_url)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned));
        hosts
            .entry(host.unwrap_or_default())
            .or_default()
            .push((feed.id, feed.failing));
    }
    let at_once = Arc::new(Semaphore::new(pace.at_once));
    let mut fetches = JoinSet::new();
    for feeds in hosts.into_values() {
        let (db, client, at_once) = (db.clone(), client.clone(), at_once.clone());
        let (rules, events, monitor) = (rules.clone(), events.clone(), monitor.clone());
        fetches.spawn(async move {
            for (feed_id, was_failing) in feeds {
                let Ok(_permit) = at_once.acquire().await else {
                    return;
                };
                let fetching = Instant::now();
                let result = fetch_feed(&db, &client, &rules, feed_id).await;
                let fetched = match &result {
                    Ok(Outcome::Fetched(stored)) => Fetched::Fetched {
                        new: u64::try_from(stored.new).unwrap_or(0),
                    },
                    Ok(Outcome::NotModified) => Fetched::Unchanged,
                    Ok(Outcome::Failed(_)) => Fetched::Failed,
                    Err(_) => Fetched::Broken,
                };
                let now = Instant::now();
                monitor.fetch(fetched, now - fetching, now);
                let outcome = match result {
                    Ok(outcome) => outcome,
                    Err(err) => {
                        tracing::error!(feed_id, "fetching: {err:#}");
                        continue;
                    }
                };
                match &outcome {
                    Outcome::Failed(why) => tracing::info!(feed_id, "fetch failed: {why}"),
                    outcome => tracing::debug!(feed_id, ?outcome, "fetched"),
                }
                for change in changes(feed_id, was_failing, outcome) {
                    if let Err(err) = events.publish_to_readers(&db, feed_id, change).await {
                        tracing::error!(feed_id, "telling readers: {err:#}");
                    }
                }
            }
        });
    }
    while fetches.join_next().await.is_some() {}
    monitor.round(Round {
        at: began,
        feeds: count,
        took: started.elapsed(),
    });
    Ok(count)
}

/// What the readers of a feed hear about a fetch of it: new articles, new
/// versions, and the feed starting or ceasing to fail. `was_failing` says how
/// the feed stood before.
pub fn changes(feed_id: i32, was_failing: bool, outcome: Outcome) -> Vec<Change> {
    let mut changes = Vec::new();
    let works_again = || Change::Feed {
        feed_id,
        error: None,
    };
    match outcome {
        Outcome::Failed(error) => {
            // Said once, when it starts; the reason is at the feed after that.
            if !was_failing {
                changes.push(Change::Feed {
                    feed_id,
                    error: Some(error),
                });
            }
        }
        Outcome::NotModified => changes.extend(was_failing.then(works_again)),
        Outcome::Fetched(stored) => {
            changes.extend(was_failing.then(works_again));
            if let Some(first_new) = stored.first_new
                && stored.new > 0
            {
                let last_seq = first_new.saturating_add(stored.new as i32 - 1);
                changes.push(Change::Articles {
                    feed_id,
                    last_seq,
                    new: stored.new,
                });
            }
            if stored.updated > 0 {
                changes.push(Change::Updated {
                    feed_id,
                    count: stored.updated,
                });
            }
        }
    }
    changes
}

/// Fetches what is due, again and again, for as long as the server runs.
pub async fn run(
    db: PgPool,
    client: HttpClient,
    rules: Arc<Rules>,
    events: Events,
    monitor: Arc<Monitor>,
    pace: Pace,
) {
    monitor.fetching_runs(pace);
    let mut every = tokio::time::interval(pace.every);
    // A round that takes longer than the pace allows makes the next one wait,
    // instead of starting several at once to catch up.
    every.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        every.tick().await;
        if let Err(err) = fetch_due(&db, &client, &rules, &events, &monitor, &pace).await {
            tracing::error!("fetching feeds: {err:#}");
        }
    }
}
