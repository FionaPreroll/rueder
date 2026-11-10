//! Fetching feeds: one feed ([`fetch_feed`]), everything that is due
//! ([`fetch_due`]) and the loop the server runs ([`run`]).
//!
//! A feed is fetched with a conditional request, so a feed that did not change
//! costs its site next to nothing. What comes back goes through
//! [`crate::feeds`]; the feed's row records how the fetch went and when the next
//! one is due. A failure is written to the feed, where the interface shows it,
//! and makes the next try come later each time.

use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::Context;
use reqwest::{StatusCode, Url};
use sqlx::PgPool;
use time::OffsetDateTime;
use tokio::{sync::Semaphore, task::JoinSet};

use crate::{
    cleaning::Rules,
    feeds::{self, Stored},
    http_client::{Agent, HttpClient, Request},
};

/// The shortest and the longest time between two fetches of a feed, in seconds.
pub const MIN_INTERVAL: i32 = 15 * 60;
pub const MAX_INTERVAL: i32 = 12 * 60 * 60;
/// How many feeds are fetched at the same time.
const AT_ONCE: usize = 8;
/// How many feeds one round takes.
const ROUND: i64 = 100;
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
/// how many that were. At most [`AT_ONCE`] at a time, and never two from the
/// same host at once.
pub async fn fetch_due(
    db: &PgPool,
    client: &HttpClient,
    rules: &Arc<Rules>,
) -> anyhow::Result<usize> {
    // Taking a feed moves its next fetch away, so a second server or a round
    // that overlaps does not take it too. The fetch sets the real time.
    let due = sqlx::query!(
        "UPDATE feeds SET next_fetch_at = now() + $2::float8 * interval '1 second'
         WHERE id IN (
             SELECT f.id FROM feeds f
             WHERE f.next_fetch_at <= now()
               AND EXISTS (SELECT 1 FROM subscriptions s WHERE s.feed_id = f.id)
             ORDER BY f.next_fetch_at
             LIMIT $1
             FOR UPDATE SKIP LOCKED
         )
         RETURNING id, feed_url",
        ROUND,
        LEASE_S,
    )
    .fetch_all(db)
    .await?;
    let count = due.len();

    let mut hosts: HashMap<String, Vec<i32>> = HashMap::new();
    for feed in due {
        let host = Url::parse(&feed.feed_url)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned));
        hosts
            .entry(host.unwrap_or_default())
            .or_default()
            .push(feed.id);
    }
    let at_once = Arc::new(Semaphore::new(AT_ONCE));
    let mut fetches = JoinSet::new();
    for feeds in hosts.into_values() {
        let (db, client, at_once) = (db.clone(), client.clone(), at_once.clone());
        let rules = rules.clone();
        fetches.spawn(async move {
            for feed_id in feeds {
                let Ok(_permit) = at_once.acquire().await else {
                    return;
                };
                match fetch_feed(&db, &client, &rules, feed_id).await {
                    Ok(Outcome::Failed(why)) => tracing::info!(feed_id, "fetch failed: {why}"),
                    Ok(outcome) => tracing::debug!(feed_id, ?outcome, "fetched"),
                    Err(err) => tracing::error!(feed_id, "fetching: {err:#}"),
                }
            }
        });
    }
    while fetches.join_next().await.is_some() {}
    Ok(count)
}

/// Fetches what is due, again and again, for as long as the server runs.
pub async fn run(db: PgPool, client: HttpClient, rules: Arc<Rules>) {
    let mut every = tokio::time::interval(Duration::from_secs(30));
    loop {
        every.tick().await;
        if let Err(err) = fetch_due(&db, &client, &rules).await {
            tracing::error!("fetching feeds: {err:#}");
        }
    }
}
