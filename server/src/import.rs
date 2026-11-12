//! Brings the data of the old rueder into the new database: feeds, articles,
//! accounts, folders and subscriptions. What an account has read and its labels
//! are not in the old database but in the browsers; they follow at the switch.
//!
//! The new rueder is already in use when this runs, with accounts and feeds of
//! its own, so the old data is merged into what is there:
//!
//! - An old feed with the same address as one here becomes that feed. Its old
//!   articles come before the ones the new rueder fetched itself: they get the
//!   numbers 1 to n and the ones that are there move up by n, together with
//!   what accounts noted on them. An article both have stays as it is here.
//! - An old user becomes the account with the same name, or the one the call
//!   names (`--account old=new`); one without a counterpart gets a new account.
//!   What the account set up here stays; the old subscriptions it does not
//!   have yet come with their folders.
//!
//! The old database is only read, in one transaction, so the import sees it as
//! it was when it started. The new one is written one feed and one account at a
//! time, each in a transaction of its own: an import that stops halfway is
//! simply run again and carries on with what is missing. The server must not
//! run meanwhile.
//!
//! Articles keep their place: the number in the feed is the one the old rueder
//! works out on every request (`row_number() over (partition by feed_id order by
//! seq)`), unless the new rueder had one of the articles before, and the order
//! of arrival across feeds is the old `seq`, ahead of everything the new rueder
//! fetched itself. Where an article stands in the old rueder's count stays in
//! `legacy_articles`, for the read state of the browsers.

use std::{
    collections::{HashMap, HashSet},
    fmt,
    time::Instant,
};

use anyhow::{Context, bail};
use serde::{Deserialize, de::DeserializeOwned};
use sqlx::{Connection, PgConnection, PgPool};
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};
use url::Url;
use uuid::Uuid;

use crate::{
    articles::{self, NewArticle, Original, Placed},
    cleaning::{self, Base, Rules},
    feeds::add_inline,
    filters,
};

/// Articles read and written in one go.
const BATCH: usize = 2000;

/// The bounds of the time between two fetches, as the old fetcher kept them.
const FETCH_INTERVAL_S: (i32, i32) = (15 * 60, 12 * 60 * 60);

/// How a run goes.
#[derive(Debug, Default, Clone)]
pub struct Options {
    /// Old users that go to an account of another name: (old name, new name).
    pub accounts: Vec<(String, String)>,
    /// Only say what would be merged and what added, and write nothing.
    pub dry_run: bool,
}

/// Reads `old=new` from the command line.
pub fn account_pair(text: &str) -> Result<(String, String), String> {
    match text.split_once('=') {
        Some((old, new)) if !old.trim().is_empty() && !new.trim().is_empty() => {
            Ok((old.trim().to_owned(), new.trim().to_owned()))
        }
        _ => Err("expected <old name>=<new name>".into()),
    }
}

/// What a run did, or with `Options::dry_run` would do.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Report {
    pub dry_run: bool,
    pub feeds_added: u64,
    /// Old feeds that became a feed the new rueder had.
    pub feeds_merged: u64,
    pub feeds_known: u64,
    pub articles_added: u64,
    pub articles_known: u64,
    /// Articles the new rueder had fetched itself before the old one's copy came.
    pub articles_matched: u64,
    pub accounts_added: u64,
    /// Old users that became an account the new rueder had.
    pub accounts_merged: u64,
    pub accounts_known: u64,
    pub folders_added: u64,
    pub subscriptions_added: u64,
    /// What went together with what was there, one line each.
    pub merged: Vec<String>,
    /// Whatever else the person running the import should know.
    pub notes: Vec<String>,
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.dry_run {
            writeln!(
                f,
                "Dry run: this is what the import would do. Nothing was written.\n"
            )?;
        }
        writeln!(
            f,
            "Feeds:         {} added, {} merged, {} already there",
            self.feeds_added, self.feeds_merged, self.feeds_known
        )?;
        if self.dry_run {
            writeln!(
                f,
                "Articles:      {} to bring, {} already there",
                self.articles_added, self.articles_known
            )?;
        } else {
            writeln!(
                f,
                "Articles:      {} added, {} already there, {} already fetched by the new rueder",
                self.articles_added, self.articles_known, self.articles_matched
            )?;
        }
        writeln!(
            f,
            "Accounts:      {} added, {} merged, {} already there",
            self.accounts_added, self.accounts_merged, self.accounts_known
        )?;
        if !self.dry_run {
            writeln!(f, "Folders:       {} added", self.folders_added)?;
            writeln!(f, "Subscriptions: {} added", self.subscriptions_added)?;
        }
        if !self.merged.is_empty() {
            writeln!(f, "\nMerged with what the new rueder has:")?;
            for line in &self.merged {
                writeln!(f, "- {line}")?;
            }
        }
        if !self.notes.is_empty() {
            writeln!(f, "\nNotes:")?;
            for note in &self.notes {
                writeln!(f, "- {note}")?;
            }
        }
        Ok(())
    }
}

/// Imports what the new database does not have yet. `legacy` is a connection to
/// the old rueder's database.
pub async fn run(
    db: &PgPool,
    legacy: &mut PgConnection,
    rules: &Rules,
    options: &Options,
) -> anyhow::Result<Report> {
    let mut old = legacy
        .begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .await
        .context("starting to read the old database")?;
    let mut report = Report {
        dry_run: options.dry_run,
        ..Report::default()
    };
    let feeds = plan_feeds(db, &mut old, &mut report).await?;
    let users = plan_accounts(db, &mut old, &options.accounts, &mut report).await?;
    if !options.dry_run {
        make_room(db, &mut old).await?;
        let targets = import_feeds(db, &mut old, feeds, rules, &mut report).await?;
        set_sequence(&mut *db.acquire().await?).await?;
        import_accounts(db, &mut old, users, &targets, &mut report).await?;
    }
    note_unused_tables(&mut old, &mut report).await?;
    old.rollback().await?;
    Ok(report)
}

#[derive(sqlx::FromRow)]
struct OldFeed {
    id: Uuid,
    created_at: Option<OffsetDateTime>,
    fetched_at: Option<OffsetDateTime>,
    fetch_delay_s: i32,
    fetcher_state: serde_json::Value,
    feed_url: String,
    site_url: Option<String>,
    title: Option<String>,
    icon: Option<String>,
}

/// What the old fetcher kept about a feed, as JSON.
#[derive(Deserialize, Default)]
#[serde(default)]
struct OldFetcherState {
    etag: String,
    last_modified: String,
    working: bool,
    last_success: String,
    last_error: String,
    message: String,
}

// The old rueder keeps times without a zone, in UTC, and writes "never" as the
// year 1. Such times come out empty.
const OLD_FEEDS: &str = "
    SELECT id,
           CASE WHEN created_at >= '1970-01-01' THEN created_at AT TIME ZONE 'UTC' END AS created_at,
           CASE WHEN fetched_at >= '1970-01-01' THEN fetched_at AT TIME ZONE 'UTC' END AS fetched_at,
           fetch_delay_s, fetcher_state, feed_url, site_url, title, icon
    FROM feeds
    ORDER BY created_at, id";

/// Where an old feed goes.
enum Place {
    /// An earlier run brought it.
    Done(i32),
    /// Into this feed of the new rueder.
    Merge(i32),
    /// Into a feed of its own.
    New,
}

struct FeedPlan {
    feed: OldFeed,
    place: Place,
    /// How many articles the old rueder has for it.
    articles: i64,
}

/// What a feed address comes down to when two are compared: the same feed may
/// be known under http and https, with the host in capitals or with a slash at
/// the end.
fn address_key(address: &str) -> String {
    let address = address.trim();
    match Url::parse(address) {
        Ok(url) if matches!(url.scheme(), "http" | "https") => {
            let port = url
                .port()
                .map(|port| format!(":{port}"))
                .unwrap_or_default();
            let query = url
                .query()
                .map(|query| format!("?{query}"))
                .unwrap_or_default();
            format!(
                "{}{port}{}{query}",
                url.host_str().unwrap_or_default(),
                url.path().trim_end_matches('/'),
            )
        }
        _ => address.to_owned(),
    }
}

/// Decides for each old feed whether it was brought before, goes into a feed the
/// new rueder has, or comes as a feed of its own. Writes nothing.
async fn plan_feeds(
    db: &PgPool,
    old: &mut PgConnection,
    report: &mut Report,
) -> anyhow::Result<Vec<FeedPlan>> {
    let old_feeds: Vec<OldFeed> = sqlx::query_as(OLD_FEEDS)
        .fetch_all(&mut *old)
        .await
        .context("reading the old feeds")?;
    let counts: HashMap<Uuid, i64> =
        sqlx::query_as::<_, (Uuid, i64)>("SELECT feed_id, count(*) FROM articles GROUP BY feed_id")
            .fetch_all(&mut *old)
            .await
            .context("reading the old articles")?
            .into_iter()
            .collect();

    let new_feeds = sqlx::query!("SELECT id, feed_url, title, legacy_id FROM feeds ORDER BY id")
        .fetch_all(db)
        .await?;
    let mut done = HashMap::new();
    // Feeds that nothing old went into yet, by exact address and by its key.
    let mut by_address: HashMap<&str, Vec<usize>> = HashMap::new();
    let mut by_key: HashMap<String, Vec<usize>> = HashMap::new();
    for (n, feed) in new_feeds.iter().enumerate() {
        match feed.legacy_id {
            Some(legacy_id) => {
                done.insert(legacy_id, feed.id);
            }
            None => {
                by_address.entry(feed.feed_url.trim()).or_default().push(n);
                by_key
                    .entry(address_key(&feed.feed_url))
                    .or_default()
                    .push(n);
            }
        }
    }

    let mut claimed = HashSet::new();
    let (mut plans, mut without_url, mut articles_left_out) = (Vec::new(), 0, 0);
    let mut planned = HashSet::new();
    for feed in old_feeds {
        if feed.feed_url.trim().is_empty() {
            without_url += 1;
            continue;
        }
        let articles = counts.get(&feed.id).copied().unwrap_or(0);
        planned.insert(feed.id);
        let place = if let Some(&id) = done.get(&feed.id) {
            report.feeds_known += 1;
            report.articles_known += articles as u64;
            Place::Done(id)
        } else {
            // The same address first, then one that only looks different.
            let unclaimed = |candidates: Option<&Vec<usize>>| {
                candidates
                    .into_iter()
                    .flatten()
                    .copied()
                    .find(|n| !claimed.contains(n))
            };
            let found = unclaimed(by_address.get(feed.feed_url.trim()))
                .or_else(|| unclaimed(by_key.get(&address_key(&feed.feed_url))));
            report.articles_added += articles as u64;
            match found {
                Some(n) => {
                    claimed.insert(n);
                    let there = &new_feeds[n];
                    report.feeds_merged += 1;
                    let title = if there.title.is_empty() {
                        String::new()
                    } else {
                        format!(" \"{}\"", there.title)
                    };
                    let mut line = format!(
                        "feed {} with {articles} articles: into feed {}{title}",
                        feed.feed_url.trim(),
                        there.id,
                    );
                    if there.feed_url.trim() != feed.feed_url.trim() {
                        line.push_str(&format!(", which is at {}", there.feed_url.trim()));
                    }
                    report.merged.push(line);
                    Place::Merge(there.id)
                }
                None => {
                    report.feeds_added += 1;
                    Place::New
                }
            }
        };
        plans.push(FeedPlan {
            feed,
            place,
            articles,
        });
    }
    for (feed, count) in &counts {
        if !planned.contains(feed) {
            articles_left_out += count;
        }
    }
    if without_url > 0 {
        report.notes.push(format!(
            "{without_url} feeds without an address were left out, with their articles"
        ));
    }
    if articles_left_out > 0 {
        report.notes.push(format!(
            "{articles_left_out} articles of feeds that were left out were left out"
        ));
    }
    Ok(plans)
}

/// Makes room for the old articles in the order of arrival. They come before
/// everything the new rueder fetched itself, and their place is their old
/// `seq`; the articles that are there move behind the newest old one. That
/// happens once, before the first old article comes in.
async fn make_room(db: &PgPool, old: &mut PgConnection) -> anyhow::Result<()> {
    let newest_old: Option<i64> = sqlx::query_scalar("SELECT max(seq) FROM articles")
        .fetch_one(&mut *old)
        .await
        .context("reading the old articles")?;
    let Some(newest_old) = newest_old else {
        return Ok(());
    };
    let mut tx = db.begin().await?;
    let there = sqlx::query!(
        r#"SELECT min(id), max(id), EXISTS (SELECT 1 FROM legacy_articles) AS "imported!"
           FROM articles"#
    )
    .fetch_one(&mut *tx)
    .await?;
    if let (Some(first), Some(last), false) = (there.min, there.max, there.imported)
        && first <= newest_old
    {
        // Past the highest id there is, so no two articles of a feed share a
        // place at any moment of the update.
        let by = newest_old.max(last);
        sqlx::query!("UPDATE articles SET id = id + $1", by)
            .execute(&mut *tx)
            .await?;
    }
    set_sequence(&mut tx).await?;
    tx.commit().await?;
    Ok(())
}

/// New articles arrive behind all that are there.
async fn set_sequence(conn: &mut PgConnection) -> anyhow::Result<()> {
    sqlx::query!(
        "SELECT setval('articles_id_seq', greatest(
             (SELECT max(id) FROM articles), (SELECT last_value FROM articles_id_seq)))"
    )
    .fetch_one(conn)
    .await?;
    Ok(())
}

/// An old feed's place in the new database.
struct Target {
    id: i32,
    /// The old rueder's title, to tell a name a user gave a feed from it.
    title: String,
    /// The number of the last old article. Everything up to it was there in
    /// the old rueder, what comes after it the new rueder fetched itself.
    old_end: i32,
    /// Old articles the new rueder had fetched itself and that therefore come
    /// after `old_end`.
    matched: Vec<i32>,
}

async fn import_feeds(
    db: &PgPool,
    old: &mut PgConnection,
    plans: Vec<FeedPlan>,
    rules: &Rules,
    report: &mut Report,
) -> anyhow::Result<HashMap<Uuid, Target>> {
    let started = Instant::now();
    let mut targets = HashMap::new();
    let mut done = Vec::new();
    let mut unreadable = 0;
    for plan in plans {
        let title = match plan.feed.title.as_deref().map(str::trim) {
            Some(title) if !title.is_empty() => title.to_owned(),
            _ => plan.feed.feed_url.clone(),
        };
        if let Place::Done(id) = plan.place {
            done.push((plan.feed.id, id, title, plan.articles));
            continue;
        }
        let old_id = plan.feed.id;
        let brought = import_feed(db, old, plan, rules).await?;
        report.articles_matched += brought.matched.len() as u64;
        unreadable += brought.unreadable;
        targets.insert(
            old_id,
            Target {
                id: brought.id,
                title,
                old_end: brought.old_end,
                matched: brought.matched,
            },
        );
        tracing::info!(
            "{} feeds brought, after {} s",
            targets.len(),
            started.elapsed().as_secs()
        );
    }
    // What the planning counted as to bring includes what turned out to be there.
    report.articles_added -= report.articles_matched;

    // Feeds of an earlier run: which of their articles the new rueder had.
    let ids: Vec<_> = done.iter().map(|feed| feed.1).collect();
    let mut matched: HashMap<i32, Vec<i32>> = HashMap::new();
    for row in sqlx::query!(
        "SELECT feed_id, seq FROM legacy_articles WHERE matched AND feed_id = ANY($1)",
        &ids
    )
    .fetch_all(db)
    .await?
    {
        matched.entry(row.feed_id).or_default().push(row.seq);
    }
    for (old_id, id, title, articles) in done {
        let matched = matched.remove(&id).unwrap_or_default();
        let old_end = i32::try_from(articles)? - i32::try_from(matched.len())?;
        targets.insert(
            old_id,
            Target {
                id,
                title,
                old_end,
                matched,
            },
        );
    }
    if unreadable > 0 {
        report.notes.push(format!(
            "{unreadable} articles had content the import could not read; they came without text"
        ));
    }
    Ok(targets)
}

#[derive(sqlx::FromRow)]
struct OldArticle {
    id: Uuid,
    seq: i64,
    site_guid: String,
    created_at: Option<OffsetDateTime>,
    posted_at: Option<OffsetDateTime>,
    link: Option<String>,
    thumbnail: Option<String>,
    image: Option<String>,
    image_title: Option<String>,
    title: Option<String>,
    teaser: Option<String>,
    content: serde_json::Value,
}

/// What the old rueder keeps of an article besides its columns, as JSON.
#[derive(Deserialize, Default)]
#[serde(default)]
struct OldContent {
    authors: Vec<String>,
    tags: Vec<String>,
    enclosures: Vec<serde_json::Value>,
    text: String,
}

const OLD_ARTICLES: &str = "
    SELECT id, seq, site_guid,
           CASE WHEN created_at >= '1970-01-01' THEN created_at AT TIME ZONE 'UTC' END AS created_at,
           CASE WHEN posted_at >= '1970-01-01' THEN posted_at AT TIME ZONE 'UTC' END AS posted_at,
           link, thumbnail, image, image_title, title, teaser, content
    FROM articles
    WHERE seq = ANY($1)
    ORDER BY seq";

/// What bringing one feed came to.
struct Brought {
    id: i32,
    old_end: i32,
    matched: Vec<i32>,
    unreadable: u64,
}

/// Brings one old feed with all its articles, in one transaction.
async fn import_feed(
    db: &PgPool,
    old: &mut PgConnection,
    plan: FeedPlan,
    rules: &Rules,
) -> anyhow::Result<Brought> {
    let FeedPlan { feed, place, .. } = plan;
    let merge = match place {
        Place::Done(_) => bail!("feed {} was brought before", feed.id),
        Place::Merge(id) => Some(id),
        Place::New => None,
    };
    // The old articles in their order, without their texts; a feed may have
    // hundreds of thousands. The old database has no index for this order, so
    // it sorts once here instead of for every batch.
    let seqs: Vec<i64> =
        sqlx::query_scalar("SELECT seq FROM articles WHERE feed_id = $1 ORDER BY seq")
            .bind(feed.id)
            .fetch_all(&mut *old)
            .await
            .context("reading the old articles")?;

    let mut tx = db.begin().await?;
    let (feed_id, own_last) = match merge {
        Some(id) => {
            let last = sqlx::query_scalar!(
                "SELECT last_seq FROM feeds WHERE id = $1 AND legacy_id IS NULL FOR UPDATE",
                id
            )
            .fetch_optional(&mut *tx)
            .await?
            .context("the feed to merge with changed during the import")?;
            (id, last)
        }
        None => (add_feed(&mut tx, &feed).await?, 0),
    };

    // Old articles the new rueder fetched itself, found by their ids in the feed.
    let mut matched: HashMap<i64, i32> = HashMap::new();
    if own_last > 0 {
        let own = sqlx::query!(
            "SELECT guid, seq FROM article_content WHERE feed_id = $1",
            feed_id
        )
        .fetch_all(&mut *tx)
        .await?;
        let guids: Vec<_> = own.iter().map(|row| row.guid.clone()).collect();
        let at: HashMap<String, i32> = own.into_iter().map(|row| (row.guid, row.seq)).collect();
        let found: Vec<(i64, String)> = sqlx::query_as(
            "SELECT seq, site_guid FROM articles WHERE feed_id = $1 AND site_guid = ANY($2)",
        )
        .bind(feed.id)
        .bind(&guids)
        .fetch_all(&mut *old)
        .await
        .context("reading the old articles")?;
        for (seq, guid) in found {
            if let Some(&own_seq) = at.get(&guid) {
                matched.insert(seq, own_seq);
            }
        }
    }
    let old_end = i32::try_from(seqs.len() - matched.len())?;
    if own_last > 0 && old_end > 0 {
        move_up(&mut tx, feed_id, own_last, old_end).await?;
    }
    // For the accounts that have the feed, the old articles count as read until
    // the browsers say otherwise.
    sqlx::query!(
        "UPDATE subscriptions SET read_upto = read_upto + $2 WHERE feed_id = $1",
        feed_id,
        old_end
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE feeds SET last_seq = last_seq + $2, legacy_id = $3 WHERE id = $1",
        feed_id,
        old_end,
        feed.id
    )
    .execute(&mut *tx)
    .await?;

    let now = OffsetDateTime::now_utc();
    let (mut seq, mut counted, mut unreadable) = (0_i32, 0_i32, 0_u64);
    let mut moved_matched = Vec::new();
    for batch in seqs.chunks(BATCH) {
        let rows: Vec<OldArticle> = sqlx::query_as(OLD_ARTICLES)
            .bind(batch)
            .fetch_all(&mut *old)
            .await
            .context("reading the old articles")?;
        let mut arrivals = Vec::new();
        let mut mapped = Vec::new();
        for row in rows {
            // The number the old rueder gives the article.
            counted += 1;
            if let Some(&own_seq) = matched.get(&row.seq) {
                moved_matched.push(own_seq + old_end);
                mapped.push((row.id, own_seq + old_end, counted, true));
                continue;
            }
            seq += 1;
            mapped.push((row.id, seq, counted, false));
            let arrived = row.seq;
            let (article, readable) = old_article(row, &feed.feed_url, rules, now);
            unreadable += u64::from(!readable);
            arrivals.push((arrived, article));
        }
        let first = seq - arrivals.len() as i32 + 1;
        let placed: Vec<_> = arrivals
            .iter()
            .zip(first..)
            .map(|((arrived, article), seq)| Placed {
                feed_id,
                seq,
                id: Some(*arrived),
                article,
            })
            .collect();
        articles::write(&mut tx, &placed).await?;
        let legacy_ids: Vec<_> = mapped.iter().map(|m| m.0).collect();
        let new_seqs: Vec<_> = mapped.iter().map(|m| m.1).collect();
        let old_seqs: Vec<_> = mapped.iter().map(|m| m.2).collect();
        let were_there: Vec<_> = mapped.iter().map(|m| m.3).collect();
        sqlx::query!(
            "INSERT INTO legacy_articles (legacy_id, feed_id, seq, legacy_seq, matched)
             SELECT n.legacy_id, $1, n.seq, n.legacy_seq, n.matched
             FROM unnest($2::uuid[], $3::integer[], $4::integer[], $5::boolean[])
                  AS n (legacy_id, seq, legacy_seq, matched)",
            feed_id,
            &legacy_ids,
            &new_seqs,
            &old_seqs,
            &were_there,
        )
        .execute(&mut *tx)
        .await?;
    }
    if seq != old_end {
        bail!(
            "the old articles of feed {} changed during the import",
            feed.id
        );
    }
    tx.commit().await?;
    Ok(Brought {
        id: feed_id,
        old_end,
        matched: moved_matched,
        unreadable,
    })
}

/// Adds an old feed as a feed of its own, without articles yet.
async fn add_feed(conn: &mut PgConnection, feed: &OldFeed) -> anyhow::Result<i32> {
    let now = OffsetDateTime::now_utc();
    let (state, _) = lenient::<OldFetcherState>(feed.fetcher_state.clone());
    let interval = feed
        .fetch_delay_s
        .clamp(FETCH_INTERVAL_S.0, FETCH_INTERVAL_S.1);
    // The first fetch comes when the old rueder would have fetched again, and
    // asks with the ETag it got, so it only loads a feed that changed.
    let next_fetch = feed
        .fetched_at
        .map_or(now, |at| at + Duration::seconds(interval.into()));
    let failing = !state.working && !state.message.is_empty();
    let id = sqlx::query_scalar!(
        "INSERT INTO feeds
             (feed_url, site_url, title, icon_url, fetch_interval_s, next_fetch_at, fetched_at,
              etag, last_modified, last_success_at, last_error_at, error_message, legacy_id,
              created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, coalesce($14, now()))
         RETURNING id",
        feed.feed_url,
        filled(feed.site_url.clone()),
        feed.title.clone().unwrap_or_default(),
        filled(feed.icon.clone()),
        interval,
        next_fetch,
        feed.fetched_at,
        filled(Some(state.etag)),
        filled(Some(state.last_modified)),
        old_time(&state.last_success),
        old_time(&state.last_error),
        failing.then_some(state.message),
        feed.id,
        feed.created_at,
    )
    .fetch_one(conn)
    .await?;
    Ok(id)
}

/// Moves the articles of a feed up by `by`, with everything that hangs on their
/// numbers: the foreign keys take text, versions, labels and old ids along, the
/// exceptions from the read marks are moved here. Moving a number past the last
/// one first keeps two rows from ever having the same one.
async fn move_up(conn: &mut PgConnection, feed_id: i32, last: i32, by: i32) -> anyhow::Result<()> {
    let past = last.checked_add(by).context("too many articles")?;
    sqlx::query!(
        "UPDATE articles SET seq = seq + $2 WHERE feed_id = $1",
        feed_id,
        past
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "UPDATE articles SET seq = seq - $2 WHERE feed_id = $1",
        feed_id,
        last
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "UPDATE article_states SET seq = seq + $2 WHERE feed_id = $1",
        feed_id,
        past
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "UPDATE article_states SET seq = seq - $2 WHERE feed_id = $1",
        feed_id,
        last
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// An old article as the new rueder keeps it, and whether its content could be
/// read.
fn old_article(
    row: OldArticle,
    feed_url: &str,
    rules: &Rules,
    now: OffsetDateTime,
) -> (NewArticle, bool) {
    let (content, readable) = lenient::<OldContent>(row.content);
    // The old rueder noted when it stored an article, which is as close to the
    // time of the fetch as it gets.
    let fetched_at = row.created_at.or(row.posted_at).unwrap_or(now);
    // The old rueder took scripts out when it fetched a text and did the rest
    // in the browser, on every reading. Here the text is cleaned once, like one
    // that is fetched today.
    let url = filled(row.link).map(|link| rules.clean_url(&link));
    let base = Base {
        article: url.as_deref(),
        feed: Some(feed_url),
    };
    let cleaned = cleaning::clean(&content.text, base, rules);
    let mut enclosures = content.enclosures;
    add_inline(&mut enclosures, cleaned.enclosures);
    let address = |url: Option<String>| filled(url).map(|url| rules.clean_url(&url));
    let article = NewArticle {
        guid: row.site_guid,
        published_at: row.posted_at.unwrap_or(fetched_at),
        fetched_at,
        original: Original {
            title: row.title.unwrap_or_default(),
            content: content.text,
            authors: content.authors,
            tags: content.tags,
            url,
        },
        teaser: row.teaser.unwrap_or_default(),
        thumbnail_url: address(row.thumbnail),
        html: cleaned.html,
        enclosures: enclosures.into(),
        image_url: address(row.image),
        image_title: filled(row.image_title),
    };
    (article, readable)
}

#[derive(sqlx::FromRow)]
struct OldUser {
    id: Uuid,
    created_at: Option<OffsetDateTime>,
    auth_origin: String,
    auth_subject: String,
    folders: serde_json::Value,
}

/// A folder as the old rueder keeps it in a user's JSON.
#[derive(Deserialize, Default)]
#[serde(default)]
struct OldFolder {
    title: String,
    feeds: Vec<OldFolderFeed>,
}

#[derive(Deserialize)]
struct OldFolderFeed {
    id: Uuid,
    /// The feed's title when it was put there, or the name the user gave it.
    #[serde(default)]
    title: String,
}

const OLD_USERS: &str = "
    SELECT id,
           CASE WHEN created_at >= '1970-01-01' THEN created_at AT TIME ZONE 'UTC' END AS created_at,
           auth_origin, auth_subject, folders
    FROM users
    ORDER BY created_at, id";

/// Where an old user goes.
enum Who {
    /// An earlier run brought it.
    Done,
    /// Into this account of the new rueder.
    Merge(i32),
    /// Into a new account of this name.
    New(String),
}

struct UserPlan {
    user: OldUser,
    who: Who,
}

/// Decides for each old user whether it was brought before, goes into an
/// account the new rueder has, or gets a new one. Writes nothing, and stops
/// before anything is written if `mapped` names a user or an account that is
/// not there.
async fn plan_accounts(
    db: &PgPool,
    old: &mut PgConnection,
    mapped: &[(String, String)],
    report: &mut Report,
) -> anyhow::Result<Vec<UserPlan>> {
    let users: Vec<OldUser> = sqlx::query_as(OLD_USERS)
        .fetch_all(&mut *old)
        .await
        .context("reading the old users")?;
    let accounts = sqlx::query!("SELECT id, username, legacy_id FROM accounts ORDER BY id")
        .fetch_all(db)
        .await?;
    let by_name: HashMap<String, usize> = accounts
        .iter()
        .enumerate()
        .map(|(n, account)| (account.username.to_lowercase(), n))
        .collect();

    // Which old user goes where by the call, checked before anything happens.
    let mut chosen: HashMap<Uuid, usize> = HashMap::new();
    for (old_name, new_name) in mapped {
        let mut named = users.iter().filter(|user| user.auth_subject == *old_name);
        let (Some(user), None) = (named.next(), named.next()) else {
            bail!("--account {old_name}={new_name}: there is not exactly one old user {old_name}");
        };
        let Some(&n) = by_name.get(&new_name.to_lowercase()) else {
            bail!("--account {old_name}={new_name}: there is no account {new_name}");
        };
        if accounts[n].legacy_id.is_some_and(|id| id != user.id) {
            bail!("--account {old_name}={new_name}: {new_name} already has another old user");
        }
        if chosen.values().any(|&other| other == n) {
            bail!("--account {old_name}={new_name}: {new_name} is named twice");
        }
        if chosen.insert(user.id, n).is_some() {
            bail!("--account {old_name}={new_name}: {old_name} is named twice");
        }
    }

    let done: HashSet<Uuid> = accounts.iter().filter_map(|a| a.legacy_id).collect();
    let mut claimed: HashSet<usize> = chosen.values().copied().collect();
    let mut taken: HashSet<String> = by_name.keys().cloned().collect();
    let mut renamed = Vec::new();
    let mut plans = Vec::new();
    for user in users {
        let who = if done.contains(&user.id) {
            report.accounts_known += 1;
            Who::Done
        } else if let Some(&n) = chosen.get(&user.id) {
            report.accounts_merged += 1;
            report.merged.push(format!(
                "user {}: into account {}, as the call says",
                user.auth_subject, accounts[n].username
            ));
            Who::Merge(accounts[n].id)
        } else if let Some(&n) = by_name
            .get(&user.auth_subject.trim().to_lowercase())
            .filter(|&&n| accounts[n].legacy_id.is_none() && !claimed.contains(&n))
        {
            claimed.insert(n);
            report.accounts_merged += 1;
            report.merged.push(format!(
                "user {}: into account {} of the same name",
                user.auth_subject, accounts[n].username
            ));
            Who::Merge(accounts[n].id)
        } else {
            let name = free_username(&taken, &user.auth_subject);
            taken.insert(name.to_lowercase());
            if name != user.auth_subject {
                renamed.push(format!("{} is now {name}", user.auth_subject));
            }
            report.accounts_added += 1;
            Who::New(name)
        };
        plans.push(UserPlan { user, who });
    }
    if !renamed.is_empty() {
        report.notes.push(format!(
            "login names that were taken or empty got another one: {}",
            renamed.join(", ")
        ));
    }
    Ok(plans)
}

/// The login name an old user gets: its old one, with a number where that is
/// taken. Login names do not tell upper from lower case.
fn free_username(taken: &HashSet<String>, wanted: &str) -> String {
    let wanted = match wanted.trim() {
        "" => "user",
        wanted => wanted,
    };
    let mut name = wanted.to_owned();
    let mut n = 1;
    while taken.contains(&name.to_lowercase()) {
        n += 1;
        name = format!("{wanted}-{n}");
    }
    name
}

/// A subscription on its way into the new database.
struct NewSubscription<'a> {
    target: &'a Target,
    /// The user's own name for the feed.
    title: Option<String>,
}

async fn import_accounts(
    db: &PgPool,
    old: &mut PgConnection,
    users: Vec<UserPlan>,
    feeds: &HashMap<Uuid, Target>,
    report: &mut Report,
) -> anyhow::Result<()> {
    let mut subscribed: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    let user_feeds: Vec<(Uuid, Uuid)> =
        sqlx::query_as("SELECT user_id, feed_id FROM user_feeds ORDER BY id")
            .fetch_all(&mut *old)
            .await
            .context("reading the old subscriptions")?;
    for (user_id, feed_id) in user_feeds {
        subscribed.entry(user_id).or_default().push(feed_id);
    }

    let mut unknown_feeds = 0_u64;
    for UserPlan { user, who } in users {
        let in_no_folder = subscribed.remove(&user.id).unwrap_or_default();
        let mut tx = db.begin().await?;
        let (account_id, fresh) = match who {
            Who::Done => continue,
            Who::Merge(account_id) => {
                let updated = sqlx::query!(
                    "UPDATE accounts SET legacy_id = $2 WHERE id = $1 AND legacy_id IS NULL",
                    account_id,
                    user.id,
                )
                .execute(&mut *tx)
                .await?
                .rows_affected();
                if updated != 1 {
                    bail!("the account to merge with changed during the import");
                }
                (account_id, false)
            }
            Who::New(username) => {
                let account_id = sqlx::query_scalar!(
                    "INSERT INTO accounts (username, legacy_id, created_at)
                     VALUES ($1, $2, coalesce($3, now())) RETURNING id",
                    username,
                    user.id,
                    user.created_at,
                )
                .fetch_one(&mut *tx)
                .await?;
                (account_id, true)
            }
        };
        // How the user logged in so far. A password comes from the command
        // line; the old hashes are not taken over.
        sqlx::query!(
            "INSERT INTO account_identities (issuer, subject, account_id) VALUES ($1, $2, $3)
             ON CONFLICT DO NOTHING",
            user.auth_origin,
            user.auth_subject,
            account_id,
        )
        .execute(&mut *tx)
        .await?;

        let (folders, readable) = lenient::<Vec<OldFolder>>(user.folders);
        if !readable {
            report.notes.push(format!(
                "the folders of {} could not be read; the feeds came without folders",
                user.auth_subject
            ));
        }
        let tree = OldTree {
            folders: &folders,
            in_no_folder: &in_no_folder,
        };
        plant(
            &mut tx,
            account_id,
            tree,
            feeds,
            fresh,
            &mut unknown_feeds,
            report,
        )
        .await?;
        tx.commit().await?;
    }

    if unknown_feeds > 0 {
        report.notes.push(format!(
            "{unknown_feeds} subscriptions to feeds that do not exist were left out"
        ));
    }
    Ok(())
}

/// An old user's folders and the feeds that are in none of them.
struct OldTree<'a> {
    folders: &'a [OldFolder],
    in_no_folder: &'a [Uuid],
}

/// Puts an old user's tree into an account. What the account has stays where
/// it is: a feed it subscribes to already keeps its folder, title and place.
/// The other feeds come with their old folders, in the order of the old JSON,
/// behind what is there; an old folder goes into one on the top level with the
/// same title. Feeds that were in no folder follow at the end of the top level.
/// A `fresh` account also gets the old folders that stay empty.
async fn plant(
    conn: &mut PgConnection,
    account_id: i32,
    tree: OldTree<'_>,
    feeds: &HashMap<Uuid, Target>,
    fresh: bool,
    unknown_feeds: &mut u64,
    report: &mut Report,
) -> anyhow::Result<()> {
    let mut taken: HashSet<i32> = sqlx::query_scalar!(
        "SELECT feed_id FROM subscriptions WHERE account_id = $1",
        account_id
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .collect();
    let mut end = sqlx::query_scalar!(
        r#"SELECT coalesce(max(position), 0) AS "end!" FROM (
               SELECT position FROM folders WHERE account_id = $1 AND parent_id IS NULL
               UNION ALL
               SELECT position FROM subscriptions WHERE account_id = $1 AND folder_id IS NULL
           ) top"#,
        account_id
    )
    .fetch_one(&mut *conn)
    .await?;
    let mut there: HashMap<String, i32> = HashMap::new();
    for folder in sqlx::query!(
        "SELECT id, title FROM folders WHERE account_id = $1 AND parent_id IS NULL
         ORDER BY position DESC",
        account_id
    )
    .fetch_all(&mut *conn)
    .await?
    {
        // The first of the same title wins.
        there.insert(folder.title.trim().to_lowercase(), folder.id);
    }

    // Every feed once: where it stands first, unless the account has it.
    let mut subscription = |feed: Uuid, title: &str| {
        let Some(target) = feeds.get(&feed) else {
            *unknown_feeds += 1;
            return None;
        };
        taken.insert(target.id).then(|| NewSubscription {
            target,
            title: (!title.is_empty() && title != target.title).then(|| title.to_owned()),
        })
    };

    for (n, folder) in tree.folders.iter().enumerate() {
        let title = match folder.title.trim() {
            "" => format!("Folder {}", n + 1),
            title => title.to_owned(),
        };
        let inside: Vec<_> = folder
            .feeds
            .iter()
            .filter_map(|feed| subscription(feed.id, &feed.title))
            .collect();
        if inside.is_empty() && !fresh {
            continue;
        }
        let existing = if fresh {
            None
        } else {
            there.get(&title.to_lowercase()).copied()
        };
        let (folder_id, last) = match existing {
            Some(folder_id) => {
                let last = sqlx::query_scalar!(
                    r#"SELECT coalesce(max(position), 0) AS "end!" FROM (
                           SELECT position FROM folders WHERE parent_id = $1
                           UNION ALL
                           SELECT position FROM subscriptions WHERE folder_id = $1
                       ) inside"#,
                    folder_id
                )
                .fetch_one(&mut *conn)
                .await?;
                (folder_id, last)
            }
            None => {
                end += 1.0;
                let folder_id = sqlx::query_scalar!(
                    "INSERT INTO folders (account_id, title, position) VALUES ($1, $2, $3)
                     RETURNING id",
                    account_id,
                    title,
                    end,
                )
                .fetch_one(&mut *conn)
                .await?;
                report.folders_added += 1;
                (folder_id, 0.0)
            }
        };
        report.subscriptions_added +=
            subscribe(conn, account_id, Some(folder_id), last, inside).await?;
    }
    let top: Vec<_> = tree
        .in_no_folder
        .iter()
        .filter_map(|&feed| subscription(feed, ""))
        .collect();
    report.subscriptions_added += subscribe(conn, account_id, None, end, top).await?;
    Ok(())
}

/// Subscribes an account to feeds, placed one after the other behind `after`,
/// and returns how many it did not subscribe to before. The old articles count
/// as read: what was read is known only to the browsers until the switch, and
/// nobody wants to start with every article of ten years unread. What the new
/// rueder fetched since is unread, as for any new subscription, and the
/// account's rules look at it.
async fn subscribe(
    conn: &mut PgConnection,
    account_id: i32,
    folder_id: Option<i32>,
    after: f64,
    subscriptions: Vec<NewSubscription<'_>>,
) -> anyhow::Result<u64> {
    if subscriptions.is_empty() {
        return Ok(0);
    }
    let feed_ids: Vec<_> = subscriptions.iter().map(|s| s.target.id).collect();
    let positions: Vec<_> = (1..=subscriptions.len())
        .map(|n| after + n as f64)
        .collect();
    let read_upto: Vec<_> = subscriptions.iter().map(|s| s.target.old_end).collect();
    let titles: Vec<_> = subscriptions.iter().map(|s| s.title.clone()).collect();
    let added: HashSet<i32> = sqlx::query_scalar!(
        "INSERT INTO subscriptions (account_id, feed_id, folder_id, position, title, read_upto)
         SELECT $1, n.feed_id, $2, n.position, n.title, n.read_upto
         FROM unnest($3::integer[], $4::float8[], $5::text[], $6::integer[])
              AS n (feed_id, position, title, read_upto)
         -- Nobody subscribes anew to a feed that is blocked here.
         WHERE NOT EXISTS (
             SELECT 1 FROM feeds f WHERE f.id = n.feed_id AND f.blocked_at IS NOT NULL)
         ON CONFLICT (account_id, feed_id) DO NOTHING
         RETURNING feed_id",
        account_id,
        folder_id,
        &feed_ids,
        &positions,
        &titles as &[Option<String>],
        &read_upto,
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .collect();

    // Old articles the new rueder had already are behind the read mark, but
    // were there in the old rueder too.
    let (matched_feeds, matched_seqs): (Vec<_>, Vec<_>) = subscriptions
        .iter()
        .filter(|s| added.contains(&s.target.id))
        .flat_map(|s| s.target.matched.iter().map(|&seq| (s.target.id, seq)))
        .unzip();
    sqlx::query!(
        "INSERT INTO article_states (account_id, feed_id, seq, read)
         SELECT $1, n.feed_id, n.seq, true FROM unnest($2::integer[], $3::integer[]) AS n (feed_id, seq)
         ON CONFLICT DO NOTHING",
        account_id,
        &matched_feeds,
        &matched_seqs,
    )
    .execute(&mut *conn)
    .await?;
    for &feed_id in &feed_ids {
        if added.contains(&feed_id) {
            filters::subscribed(conn, account_id, feed_id).await?;
        }
    }
    Ok(added.len() as u64)
}

/// The old rueder has two tables it never used. Should they hold something
/// after all, somebody ought to look.
async fn note_unused_tables(old: &mut PgConnection, report: &mut Report) -> anyhow::Result<()> {
    for (table, count) in [
        ("user_states", "SELECT count(*) FROM user_states"),
        ("labels", "SELECT count(*) FROM labels"),
    ] {
        let exists: bool = sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
            .bind(table)
            .fetch_one(&mut *old)
            .await?;
        if !exists {
            continue;
        }
        let rows: i64 = sqlx::query_scalar(count).fetch_one(&mut *old).await?;
        if rows > 0 {
            report.notes.push(format!(
                "the old table {table} has {rows} rows; the old rueder never read them, \
                 and they were not imported"
            ));
        }
    }
    Ok(())
}

/// Reads JSON of the old database. Whatever does not have the expected shape
/// comes out empty, with `false`, so that one odd row does not stop an import of
/// millions.
fn lenient<T: DeserializeOwned + Default>(value: serde_json::Value) -> (T, bool) {
    if value.is_null() {
        return (T::default(), true);
    }
    match serde_json::from_value(value) {
        Ok(read) => (read, true),
        Err(_) => (T::default(), false),
    }
}

/// The old rueder writes what it does not have as an empty string in places.
fn filled(text: Option<String>) -> Option<String> {
    text.filter(|text| !text.trim().is_empty())
}

/// A time out of the old rueder's JSON, where "never" is the year 1.
fn old_time(text: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(text, &Rfc3339)
        .ok()
        .filter(|at| at.year() >= 1970)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_that_only_look_different_have_one_key() {
        let key = address_key("https://Blog.Example/feed/");
        assert_eq!(key, address_key("http://blog.example/feed"));
        assert_eq!(key, address_key("  https://blog.example:443/feed "));
        assert_ne!(key, address_key("https://blog.example/feed?page=2"));
        assert_ne!(key, address_key("https://blog.example:8443/feed"));
        assert_ne!(key, address_key("https://other.example/feed"));
        assert_eq!(address_key(" not a url "), "not a url");
    }

    #[test]
    fn account_pairs_need_both_names() {
        assert_eq!(
            account_pair(" alice = Alice2 "),
            Ok(("alice".into(), "Alice2".into()))
        );
        assert!(account_pair("alice").is_err());
        assert!(account_pair("=alice").is_err());
        assert!(account_pair("alice=").is_err());
    }

    #[test]
    fn taken_names_get_a_number() {
        let taken: HashSet<String> = ["alice".into(), "alice-2".into(), "user".into()].into();
        assert_eq!(free_username(&taken, "Alice"), "Alice-3");
        assert_eq!(free_username(&taken, " "), "user-2");
        assert_eq!(free_username(&taken, "bob"), "bob");
    }
}
