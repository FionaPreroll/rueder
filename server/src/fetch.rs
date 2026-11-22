//! Fetching feeds: one feed ([`fetch_feed`]), everything that is due
//! ([`fetch_due`]) and the loop the server runs ([`run`]).
//!
//! A feed is fetched with a conditional request, so a feed that did not change
//! costs its site next to nothing. What comes back goes through
//! [`crate::feeds`]; the feed's row records how the fetch went and when the next
//! one is due. A web page without a feed is asked of RSSifier instead, which
//! answers with a feed of it, or with a problem that says why not. A failure
//! is written to the feed, where the interface shows it,
//! and makes the next try come later each time. A site that says it is asked
//! too often (429) is asked less often for a while, and one that says when to
//! come back (`Retry-After`) is not asked before then.

use std::{
    collections::{HashMap, HashSet},
    ops::RangeInclusive,
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, Instant},
};

use anyhow::Context;
use reqwest::{StatusCode, Url};
use sqlx::PgPool;
use time::{OffsetDateTime, format_description::well_known::Rfc2822};
use tokio::{sync::Semaphore, task::JoinSet};

use crate::{
    cleaning::Rules,
    events::{Events, ServerEvent},
    feeds::{self, Stored},
    filters,
    http_client::{Agent, HttpClient, Request},
    monitor::{Fetched, Monitor, Round},
    rssifier::{self, Rssifier},
    sizes::within,
};

/// The shortest and the longest time between two fetches of a feed, in seconds.
pub const MIN_INTERVAL: i32 = 15 * 60;
pub const MAX_INTERVAL: i32 = 12 * 60 * 60;
/// How fast feeds are fetched: how many one round takes, how many are fetched
/// at the same time, how often a round starts, and how many of one host are
/// fetched at the same time. Together they cap how many
/// fetches an hour has, [`Pace::per_hour`]: 12,000 as rueder comes. A large
/// instance sets them with `FETCH_ROUND`, `FETCH_AT_ONCE`, `FETCH_EVERY`
/// (seconds) and `FETCH_PER_HOST`; the server checks them against [`ROUNDS`],
/// [`AT_ONCE`], [`EVERY`] and [`PER_HOST`] when it starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pace {
    pub round: i64,
    pub at_once: usize,
    pub every: Duration,
    pub per_host: usize,
}

/// The sizes of a round a server may be given.
pub const ROUNDS: RangeInclusive<i64> = 1..=10_000;
/// How many fetches at once a server may be given.
pub const AT_ONCE: RangeInclusive<usize> = 1..=256;
/// How many seconds between two rounds a server may be given.
pub const EVERY: RangeInclusive<u64> = 5..=3600;
/// How many fetches of one host at once a server may be given. A site that
/// hosts many feeds, such as a video platform, may take more than one.
pub const PER_HOST: RangeInclusive<usize> = 1..=32;

impl Default for Pace {
    fn default() -> Self {
        Self {
            round: 100,
            at_once: 8,
            every: Duration::from_secs(30),
            per_host: 1,
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
        let pace = Self::default();
        Ok(Self {
            round: within("FETCH_ROUND", var("FETCH_ROUND"), &ROUNDS)?.unwrap_or(pace.round),
            at_once: within("FETCH_AT_ONCE", var("FETCH_AT_ONCE"), &AT_ONCE)?
                .unwrap_or(pace.at_once),
            every: within("FETCH_EVERY", var("FETCH_EVERY"), &EVERY)?
                .map_or(pace.every, Duration::from_secs),
            per_host: within("FETCH_PER_HOST", var("FETCH_PER_HOST"), &PER_HOST)?
                .unwrap_or(pace.per_host),
        })
    }

    /// How many fetches an hour can have at most at this pace, if every round
    /// is over before the next one starts.
    pub fn per_hour(&self) -> u64 {
        let rounds = 3600.0 / self.every.as_secs_f64();
        (self.round as f64 * rounds).round() as u64
    }
}
/// How long a feed that is taken is kept from other servers, in case this one
/// dies; this one keeps it in [`Underway`] for as long as it takes.
const LEASE_S: f64 = 600.0;

/// The hosts whose feeds are being fetched now, of all rounds. Rounds do not
/// wait for each other: a round fetches the feeds it takes of one host after
/// the other, and while it does, the next rounds leave that host out and go
/// on with the others. So a host with many slow feeds holds up nothing but
/// itself.
pub struct Underway {
    at_once: Semaphore,
    hosts: Mutex<HashSet<String>>,
}

/// A host taken by a round: it is busy until this is dropped.
struct Busy {
    underway: Arc<Underway>,
    host: String,
}

impl Drop for Busy {
    fn drop(&mut self) {
        self.underway.hosts().remove(&self.host);
    }
}

impl Underway {
    pub fn new(pace: &Pace) -> Arc<Self> {
        Arc::new(Self {
            at_once: Semaphore::new(pace.at_once),
            hosts: Mutex::default(),
        })
    }

    fn hosts(&self) -> std::sync::MutexGuard<'_, HashSet<String>> {
        self.hosts.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The hosts that are busy.
    pub fn busy(&self) -> Vec<String> {
        self.hosts().iter().cloned().collect()
    }

    /// Takes the host for a round, or `None` if another round has it.
    fn take(self: &Arc<Self>, host: String) -> Option<Busy> {
        self.hosts().insert(host.clone()).then(|| Busy {
            underway: self.clone(),
            host,
        })
    }
}

/// The host a feed counts for: its name as the address has it, in small
/// letters, as `round` finds it in the database too.
fn host(feed_url: &str) -> String {
    Url::parse(feed_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_lowercase))
        .unwrap_or_default()
}

/// What rueder would like to get.
const ACCEPT: &str = "application/atom+xml, application/rss+xml, application/feed+json, \
                      application/xml;q=0.9, text/xml;q=0.9, */*;q=0.5";
/// The longest error text kept at a feed, in characters.
const MAX_MESSAGE: usize = 300;
/// How long a feed keeps its longer interval after its site said it is asked
/// too often (429): a success within that time does not shorten it.
pub const LIMITED_FOR: time::Duration = time::Duration::days(1);

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

/// Fetches one feed and writes down how it went: from its site, or from
/// RSSifier for a web page. An error from here means the database failed; a
/// feed that cannot be fetched is an [`Outcome::Failed`].
pub async fn fetch_feed(
    db: &PgPool,
    client: &HttpClient,
    rssifier: Option<&Rssifier>,
    rules: &Arc<Rules>,
    feed_id: i32,
) -> anyhow::Result<Outcome> {
    let feed = sqlx::query!(
        r#"SELECT feed_url, fetch_agent, etag, last_modified, source = 'page' AS "page!"
           FROM feeds WHERE id = $1"#,
        feed_id
    )
    .fetch_optional(db)
    .await?
    .with_context(|| format!("no feed {feed_id}"))?;

    let request = Request {
        agent: if feed.fetch_agent == "browser" {
            Agent::Browser
        } else {
            Agent::Own
        },
        accept: Some(ACCEPT),
        etag: feed.etag.as_deref(),
        last_modified: feed.last_modified.as_deref(),
        ..Request::new(&feed.feed_url)
    };
    let response = match (feed.page, rssifier) {
        (false, _) => client.get(request).await,
        (true, Some(rssifier)) => rssifier.feed(&feed.feed_url, feed.etag.as_deref()).await,
        (true, None) => {
            let message = "this server reads no web pages without a feed".into();
            return failed(db, feed_id, message, Back::default()).await;
        }
    };
    let response = match response {
        Ok(response) => response,
        Err(err) => return failed(db, feed_id, err.to_string(), Back::default()).await,
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
        // A site that is asked too often or down for a while may say when to
        // come back.
        let limited = response.status == StatusCode::TOO_MANY_REQUESTS;
        let back = Back {
            after: (limited || response.status == StatusCode::SERVICE_UNAVAILABLE)
                .then(|| response.header("retry-after"))
                .flatten()
                .and_then(|value| retry_after(value, now))
                .unwrap_or(0),
            limited,
        };
        let message = if feed.page {
            rssifier::problem(&response)
        } else {
            format!("the site answers with {}", response.status)
        };
        return failed(db, feed_id, message, back).await;
    }
    // What RSSifier answers is the feed of the page.
    let url = if feed.page {
        Url::parse(&feed.feed_url).unwrap_or_else(|_| response.url.clone())
    } else {
        response.url.clone()
    };
    // Reading and cleaning a large feed takes a while, so it runs where it
    // keeps nothing else waiting.
    let reading = {
        let (rules, body) = (rules.clone(), response.body.clone());
        tokio::task::spawn_blocking(move || feeds::parse(&body, url.as_str(), now, &rules))
    };
    let parsed = match reading.await {
        Ok(Ok(parsed)) => parsed,
        Ok(Err(err)) => return failed(db, feed_id, format!("{err:#}"), Back::default()).await,
        Err(_) => {
            let message = "the feed could not be read".into();
            return failed(db, feed_id, message, Back::default()).await;
        }
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
        "SELECT last_seq, created_at, fetch_interval_s, limited_at FROM feeds WHERE id = $1",
        feed_id
    )
    .fetch_one(&mut *tx)
    .await?;
    // A failure made the interval longer; the first success ends that, unless
    // the site said not long ago that it is asked too often.
    let mut interval = interval(state.last_seq, now - state.created_at)
        .unwrap_or(state.fetch_interval_s.min(4 * MIN_INTERVAL));
    if state.limited_at.is_some_and(|at| now - at < LIMITED_FOR) {
        interval = interval.max(state.fetch_interval_s);
    }
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

/// What a site that failed said about coming back.
#[derive(Default)]
struct Back {
    /// Not before this many seconds from now.
    after: i32,
    /// It is asked too often, which keeps the longer interval a while.
    limited: bool,
}

/// When a site says to come back, as seconds or a date in `Retry-After`: in
/// seconds from now, at most the longest interval.
fn retry_after(value: &str, now: OffsetDateTime) -> Option<i32> {
    let value = value.trim();
    let seconds = match value.parse::<i64>() {
        Ok(seconds) => seconds,
        Err(_) => (OffsetDateTime::parse(value, &Rfc2822).ok()? - now).whole_seconds(),
    };
    i32::try_from(seconds.clamp(0, i64::from(MAX_INTERVAL))).ok()
}

/// Writes a failure to the feed and doubles the time to the next try, up to
/// the longest interval, or waits as long as the site asked.
async fn failed(db: &PgPool, feed_id: i32, message: String, back: Back) -> anyhow::Result<Outcome> {
    let message: String = message.chars().take(MAX_MESSAGE).collect();
    sqlx::query!(
        "UPDATE feeds
         SET fetched_at = now(), last_error_at = now(), error_message = $2,
             limited_at = CASE WHEN $6 THEN now() ELSE limited_at END,
             fetch_interval_s = least(fetch_interval_s * 2, $3),
             next_fetch_at = now() + (greatest(least(fetch_interval_s * 2, $3), $5) + $4::float8) * interval '1 second'
         WHERE id = $1",
        feed_id,
        message,
        MAX_INTERVAL,
        spread(feed_id),
        back.after,
        back.limited,
    )
    .execute(db)
    .await?;
    Ok(Outcome::Failed(message))
}

/// One round on its own: what [`round`] does with nothing else under way, for
/// the command line and the tests.
pub async fn fetch_due(
    db: &PgPool,
    client: &HttpClient,
    rssifier: Option<&Arc<Rssifier>>,
    rules: &Arc<Rules>,
    events: &Events,
    monitor: &Arc<Monitor>,
    pace: &Pace,
) -> anyhow::Result<usize> {
    let underway = Underway::new(pace);
    round(
        db, client, rssifier, rules, events, monitor, pace, &underway,
    )
    .await
}

/// Takes the feeds that are due and have somebody reading them, at most a
/// round's worth of the pace and none of a host that another round is still
/// fetching, fetches them, so many at a time as the pace says and no more of
/// one host at once than `per_host`, and returns how many it took once they
/// are fetched. Each fetch and the round count for the status page. A disabled
/// account reads nothing, so a feed only such accounts subscribe to rests
/// until one of them is enabled or somebody else subscribes.
#[allow(clippy::too_many_arguments, reason = "what a round needs")]
pub async fn round(
    db: &PgPool,
    client: &HttpClient,
    rssifier: Option<&Arc<Rssifier>>,
    rules: &Arc<Rules>,
    events: &Events,
    monitor: &Arc<Monitor>,
    pace: &Pace,
    underway: &Arc<Underway>,
) -> anyhow::Result<usize> {
    let (started, began) = (Instant::now(), OffsetDateTime::now_utc());
    // Taking a feed moves its next fetch away, so a second server does not
    // take it too. The fetch sets the real time.
    let due = sqlx::query!(
        r#"UPDATE feeds SET next_fetch_at = now() + $2::float8 * interval '1 second'
         WHERE id IN (
             SELECT f.id FROM feeds f
             WHERE f.next_fetch_at <= now() AND f.blocked_at IS NULL
               AND NOT coalesce(lower(substring(f.feed_url FROM '^[^:]*://(?:[^/?#]*@)?([^/?#:]*)')), '') = ANY($3)
               AND EXISTS (SELECT 1 FROM subscriptions s JOIN accounts a ON a.id = s.account_id
                           WHERE s.feed_id = f.id AND a.disabled_at IS NULL)
             ORDER BY f.next_fetch_at
             LIMIT $1
             FOR UPDATE SKIP LOCKED
         )
         RETURNING id, feed_url, error_message IS NOT NULL AS "failing!""#,
        pace.round,
        LEASE_S,
        &underway.busy(),
    )
    .fetch_all(db)
    .await?;
    let count = due.len();

    let mut hosts: HashMap<String, Vec<(i32, bool)>> = HashMap::new();
    for feed in due {
        hosts
            .entry(host(&feed.feed_url))
            .or_default()
            .push((feed.id, feed.failing));
    }
    let mut fetches = JoinSet::new();
    for (host, feeds) in hosts {
        // A host that another round took meanwhile, or that the database
        // wrote otherwise than the address reads: its feeds wait for the next
        // round.
        let Some(busy) = underway.take(host) else {
            sqlx::query!(
                "UPDATE feeds SET next_fetch_at = now() WHERE id = ANY($1)",
                &feeds.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            )
            .execute(db)
            .await?;
            continue;
        };
        let busy = Arc::new(busy);
        // So many at once of the host as the pace allows, each one after
        // the other.
        let mut lines: Vec<Vec<(i32, bool)>> = vec![Vec::new(); pace.per_host.min(feeds.len())];
        for (n, feed) in feeds.into_iter().enumerate() {
            let at = n % lines.len();
            lines[at].push(feed);
        }
        for line in lines {
            let (db, client, underway, busy) =
                (db.clone(), client.clone(), underway.clone(), busy.clone());
            let rssifier = rssifier.cloned();
            let (rules, events, monitor) = (rules.clone(), events.clone(), monitor.clone());
            fetches.spawn(async move {
                for (feed_id, was_failing) in line {
                    fetch_one(
                        &db,
                        &client,
                        rssifier.as_deref(),
                        &rules,
                        &events,
                        &monitor,
                        &underway,
                        feed_id,
                        was_failing,
                    )
                    .await;
                }
                drop(busy);
            });
        }
    }
    while fetches.join_next().await.is_some() {}
    monitor.round(Round {
        at: began,
        feeds: count,
        took: started.elapsed(),
    });
    Ok(count)
}

/// Fetches one feed of a round when there is room, and tells its readers.
#[allow(clippy::too_many_arguments, reason = "what a round hands each fetch")]
async fn fetch_one(
    db: &PgPool,
    client: &HttpClient,
    rssifier: Option<&Rssifier>,
    rules: &Arc<Rules>,
    events: &Events,
    monitor: &Arc<Monitor>,
    underway: &Underway,
    feed_id: i32,
    was_failing: bool,
) {
    let Ok(_permit) = underway.at_once.acquire().await else {
        return;
    };
    let fetching = Instant::now();
    let result = fetch_feed(db, client, rssifier, rules, feed_id).await;
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
            return;
        }
    };
    match &outcome {
        Outcome::Failed(why) => tracing::info!(feed_id, "fetch failed: {why}"),
        outcome => tracing::debug!(feed_id, ?outcome, "fetched"),
    }
    for change in changes(feed_id, was_failing, outcome) {
        if let Err(err) = events.publish_to_readers(db, feed_id, change).await {
            tracing::error!(feed_id, "telling readers: {err:#}");
        }
    }
}

/// What the readers of a feed hear about a fetch of it: new articles, new
/// versions, and the feed starting or ceasing to fail. `was_failing` says how
/// the feed stood before.
pub fn changes(feed_id: i32, was_failing: bool, outcome: Outcome) -> Vec<ServerEvent> {
    let mut changes = Vec::new();
    let works_again = || ServerEvent::Feed {
        feed_id,
        error: None,
    };
    match outcome {
        Outcome::Failed(error) => {
            // Said once, when it starts; the reason is at the feed after that.
            if !was_failing {
                changes.push(ServerEvent::Feed {
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
                changes.push(ServerEvent::Articles {
                    feed_id,
                    last_seq,
                    new: stored.new,
                });
            }
            if stored.updated > 0 {
                changes.push(ServerEvent::Updated {
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
    rssifier: Option<Arc<Rssifier>>,
    rules: Arc<Rules>,
    events: Events,
    monitor: Arc<Monitor>,
    pace: Pace,
) {
    monitor.fetching_runs(pace);
    let underway = Underway::new(&pace);
    let mut every = tokio::time::interval(pace.every);
    // A server that was busy starts one round, not several to catch up.
    every.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        every.tick().await;
        // Rounds overlap: one that waits for a slow host keeps no other from
        // taking what is due; `underway` keeps them apart.
        let (db, client, rules) = (db.clone(), client.clone(), rules.clone());
        let rssifier = rssifier.clone();
        let (events, monitor, underway) = (events.clone(), monitor.clone(), underway.clone());
        tokio::spawn(async move {
            let round = round(
                &db,
                &client,
                rssifier.as_ref(),
                &rules,
                &events,
                &monitor,
                &pace,
                &underway,
            );
            if let Err(err) = round.await {
                tracing::error!("fetching feeds: {err:#}");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn a_site_says_when_to_come_back_in_seconds_or_as_a_date() {
        let now = datetime!(2026-11-16 12:00 UTC);
        assert_eq!(retry_after("120", now), Some(120));
        assert_eq!(retry_after(" 3600 ", now), Some(3600));
        assert_eq!(
            retry_after("Mon, 16 Nov 2026 13:30:00 GMT", now),
            Some(5400)
        );
        // Never later than the longest interval, and a time gone by is now.
        assert_eq!(retry_after("1000000", now), Some(MAX_INTERVAL));
        assert_eq!(
            retry_after("Fri, 01 Jan 2027 00:00:00 GMT", now),
            Some(MAX_INTERVAL)
        );
        assert_eq!(retry_after("-5", now), Some(0));
        assert_eq!(retry_after("Sun, 15 Nov 2026 12:00:00 GMT", now), Some(0));
        // What is neither says nothing.
        for value in ["", "soon", "1.5", "16 Nov 2026", "99999999999999999999"] {
            assert_eq!(retry_after(value, now), None, "{value}");
        }
    }
}
