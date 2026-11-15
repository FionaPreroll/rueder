//! Many made-up readers for the load tests of work package 5.30.
//!
//! `rueder bench readers` adds readers to a database of made-up data, as
//! `rueder bench generate` makes it, or to an empty one. Each reader
//! subscribes to a number of feeds that differs from reader to reader, 10 and
//! then an exponential share with a mean of [`MEAN_EXTRA`], at most
//! [`tree::MAX_FEEDS`]. Which feeds is drawn by popularity: feed `floor(n *
//! u²)` of the `n` feeds by id, so that the first are read by many and the
//! last by few, as the generator also makes the first feeds the largest. Most
//! of what a reader subscribes to is read, a few dozen articles at most are
//! not.
//!
//! With `--shared`, every new reader also subscribes to that one feed, which
//! the load test of the live updates lets publish while all readers listen.
//!
//! Every reader gets a session whose token is written to a file, one
//! `account_id token` a line, so that a load test needs no login, which would
//! hash a password for each. The tokens are random like any session's; the
//! file is the only place they are.

use std::{io::Write, path::Path};

use anyhow::{Context, bail};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sqlx::PgPool;

use crate::{sessions, tree};

/// The mean of the feeds a reader has on top of the first ten.
pub const MEAN_EXTRA: f64 = 40.0;

/// What was added.
#[derive(Debug, Clone, PartialEq)]
pub struct Added {
    pub readers: i64,
    pub subscriptions: i64,
    /// The most and the mean subscriptions of a new reader.
    pub most: i64,
    pub mean: f64,
    /// How many of the new readers the most popular feed has.
    pub top_feed_readers: i64,
}

impl std::fmt::Display for Added {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} readers with {} subscriptions, {:.1} on average and {} at most; \
             the most popular feed has {} of them",
            self.readers, self.subscriptions, self.mean, self.most, self.top_feed_readers
        )
    }
}

/// Adds `count` readers, their subscriptions and their sessions, and writes
/// the sessions' tokens to `tokens`. Refuses a database with any account that
/// made-up data did not make, so that it never runs against a real one.
pub async fn add(
    db: &PgPool,
    count: i32,
    tokens: &Path,
    shared: Option<&str>,
) -> anyhow::Result<Added> {
    if !(1..=1_000_000).contains(&count) {
        bail!("between 1 and 1,000,000 readers");
    }
    let real: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM accounts WHERE username !~ '^(bench|reader)-[0-9]+$'",
    )
    .fetch_one(db)
    .await?;
    if real > 0 {
        bail!(
            "the database has accounts made-up data did not make; readers go into a database of their own"
        );
    }
    let first: i64 = sqlx::query_scalar(
        "SELECT coalesce(max(substring(username FROM 8)::bigint), 0) + 1
         FROM accounts WHERE username LIKE 'reader-%'",
    )
    .fetch_one(db)
    .await?;
    let ids: Vec<i32> = sqlx::query_scalar(
        "INSERT INTO accounts (username)
         SELECT 'reader-' || n FROM generate_series($1::bigint, $1::bigint + $2 - 1) n
         RETURNING id",
    )
    .bind(first)
    .bind(i64::from(count))
    .fetch_all(db)
    .await?;

    // Drawn per reader: the number of feeds, then each feed by popularity.
    // Feeds drawn twice count once.
    sqlx::query(
        "WITH readers AS (
             SELECT id, least($2::int, 10 + floor(-ln(1 - random()) * $3)::int) AS feeds
             FROM unnest($1::int[]) id
         ), bounds AS (
             SELECT array_agg(id ORDER BY id) AS ids, count(*)::int AS n FROM feeds
         )
         INSERT INTO subscriptions (account_id, feed_id, position)
         SELECT r.id, b.ids[1 + floor(b.n * random() ^ 2)::int], g
         FROM readers r CROSS JOIN bounds b
         CROSS JOIN LATERAL generate_series(1, r.feeds) g
         WHERE b.n > 0
         ON CONFLICT DO NOTHING",
    )
    .bind(&ids)
    .bind(tree::MAX_FEEDS as i32)
    .bind(MEAN_EXTRA)
    .execute(db)
    .await
    .context("subscribing the readers")?;

    if let Some(url) = shared {
        sqlx::query(
            "INSERT INTO feeds (feed_url, title)
             SELECT $1, 'Shared feed' WHERE NOT EXISTS (SELECT 1 FROM feeds WHERE feed_url = $1)",
        )
        .bind(url)
        .execute(db)
        .await?;
        sqlx::query(
            "INSERT INTO subscriptions (account_id, feed_id, position)
             SELECT id, (SELECT min(id) FROM feeds WHERE feed_url = $2), 0
             FROM unnest($1::int[]) id
             ON CONFLICT DO NOTHING",
        )
        .bind(&ids)
        .bind(url)
        .execute(db)
        .await?;
    }

    // Most of it read.
    sqlx::query(
        "UPDATE subscriptions s
         SET read_upto = greatest(0, f.last_seq - floor(random() * 30)::int)
         FROM feeds f
         WHERE f.id = s.feed_id AND s.account_id = ANY($1)",
    )
    .bind(&ids)
    .execute(db)
    .await?;

    let mut file = std::fs::File::create(tokens)
        .with_context(|| format!("writing the tokens to {}", tokens.display()))?;
    for chunk in ids.chunks(10_000) {
        let mut hashes = Vec::with_capacity(chunk.len());
        for &id in chunk {
            let mut random = [0u8; 32];
            getrandom::fill(&mut random)
                .map_err(|err| anyhow::anyhow!("no random numbers: {err}"))?;
            let token = URL_SAFE_NO_PAD.encode(random);
            writeln!(file, "{id} {token}")?;
            hashes.push(sessions::token_hash(&token));
        }
        sqlx::query(
            "INSERT INTO sessions (token_hash, account_id, expires_at)
             SELECT h, id, now() + interval '30 days'
             FROM unnest($1::bytea[], $2::int[]) AS t (h, id)",
        )
        .bind(&hashes)
        .bind(chunk)
        .execute(db)
        .await?;
    }
    file.flush()?;
    sqlx::query("ANALYZE subscriptions").execute(db).await?;

    let (readers, subscriptions, most, mean): (i64, i64, i64, f64) = sqlx::query_as(
        "SELECT count(*), coalesce(sum(c), 0)::bigint, coalesce(max(c), 0), coalesce(avg(c), 0)::float8
         FROM (SELECT count(s.feed_id) AS c FROM unnest($1::int[]) id
               LEFT JOIN subscriptions s ON s.account_id = id GROUP BY id) x",
    )
    .bind(&ids)
    .fetch_one(db)
    .await?;
    let top_feed_readers: i64 = sqlx::query_scalar(
        "SELECT coalesce(max(c), 0) FROM (
             SELECT count(*) AS c FROM subscriptions WHERE account_id = ANY($1) GROUP BY feed_id
         ) x",
    )
    .bind(&ids)
    .fetch_one(db)
    .await?;
    Ok(Added {
        readers,
        subscriptions,
        most,
        mean,
        top_feed_readers,
    })
}
