//! Brings the data of the old rueder into the new database: feeds, articles,
//! accounts, folders and subscriptions. What an account has read and its labels
//! are not in the old database but in the browsers; they follow at the switch.
//!
//! The old database is only read, in one transaction, so the import sees it as
//! it was when it started. The new one is written in small transactions: an
//! import that stops halfway is simply run again.
//!
//! A run only adds what the new database does not have yet, found by the UUIDs
//! of the old one (`legacy_id`), and changes nothing that is there. So the
//! import can run once to try the new rueder and again right before the switch.
//!
//! Articles keep their place: the number in the feed is the one the old rueder
//! works out on every request (`row_number() over (partition by feed_id order by
//! seq)`), which is what the browsers' read state counts in, and the order of
//! arrival across feeds is the old `seq`.

use std::{
    collections::{HashMap, HashSet},
    fmt,
    time::Instant,
};

use anyhow::{Context, bail};
use serde::{Deserialize, de::DeserializeOwned};
use sqlx::{Connection, PgConnection, PgPool};
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::articles::{self, NewArticle, Original, Placed, guid_hash};

/// Articles read and written in one go.
const BATCH: i64 = 2000;

/// The bounds of the time between two fetches, as the old fetcher kept them.
const FETCH_INTERVAL_S: (i32, i32) = (15 * 60, 12 * 60 * 60);

/// What a run did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Report {
    pub feeds_added: u64,
    pub feeds_known: u64,
    pub articles_added: u64,
    pub articles_known: u64,
    /// Articles the new rueder had fetched itself before the old one's copy came.
    pub articles_matched: u64,
    pub accounts_added: u64,
    pub accounts_known: u64,
    pub folders_added: u64,
    pub subscriptions_added: u64,
    /// Feeds whose articles do not have the numbers the old rueder gives them.
    /// What a browser remembers as read does not fit these.
    pub renumbered_feeds: Vec<String>,
    /// Whatever else the person running the import should know.
    pub notes: Vec<String>,
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Feeds:         {} added, {} already there",
            self.feeds_added, self.feeds_known
        )?;
        writeln!(
            f,
            "Articles:      {} added, {} already there, {} already fetched by the new rueder",
            self.articles_added, self.articles_known, self.articles_matched
        )?;
        writeln!(
            f,
            "Accounts:      {} added, {} already there",
            self.accounts_added, self.accounts_known
        )?;
        writeln!(f, "Folders:       {} added", self.folders_added)?;
        writeln!(f, "Subscriptions: {} added", self.subscriptions_added)?;
        if !self.renumbered_feeds.is_empty() {
            writeln!(
                f,
                "\nArticles of these feeds do not have the numbers the old rueder gives them, \
                 so the read state of the browsers will not fit them:"
            )?;
            for feed in &self.renumbered_feeds {
                writeln!(f, "- {feed}")?;
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
pub async fn run(db: &PgPool, legacy: &mut PgConnection) -> anyhow::Result<Report> {
    let mut old = legacy
        .begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .await
        .context("starting to read the old database")?;
    let mut report = Report::default();
    let mut feeds = import_feeds(db, &mut old, &mut report).await?;
    import_articles(db, &mut old, &mut feeds, &mut report).await?;
    import_accounts(db, &mut old, &feeds, &mut report).await?;
    note_unused_tables(&mut old, &mut report).await?;
    old.rollback().await?;
    Ok(report)
}

/// An old feed's place in the new database.
struct Target {
    id: i32,
    title: String,
    /// The number of the feed's newest article in the new database.
    last_seq: i32,
    /// How many of the feed's old articles the import has come past. This is the
    /// number the old rueder gives the article it is at.
    seen: i32,
    renumbered: bool,
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

async fn import_feeds(
    db: &PgPool,
    old: &mut PgConnection,
    report: &mut Report,
) -> anyhow::Result<HashMap<Uuid, Target>> {
    let old_feeds: Vec<OldFeed> = sqlx::query_as(OLD_FEEDS)
        .fetch_all(&mut *old)
        .await
        .context("reading the old feeds")?;
    let now = OffsetDateTime::now_utc();
    let mut titles = HashMap::new();
    let mut without_url = 0;
    let mut tx = db.begin().await?;
    for feed in old_feeds {
        if feed.feed_url.trim().is_empty() {
            without_url += 1;
            continue;
        }
        let (state, _) = lenient::<OldFetcherState>(feed.fetcher_state);
        let interval = feed
            .fetch_delay_s
            .clamp(FETCH_INTERVAL_S.0, FETCH_INTERVAL_S.1);
        // The first fetch comes when the old rueder would have fetched again, and
        // asks with the ETag it got, so it only loads a feed that changed.
        let next_fetch = feed
            .fetched_at
            .map_or(now, |at| at + Duration::seconds(interval.into()));
        let failing = !state.working && !state.message.is_empty();
        let title = feed.title.unwrap_or_default();
        let added = sqlx::query!(
            "INSERT INTO feeds
                 (feed_url, site_url, title, icon_url, fetch_interval_s, next_fetch_at, fetched_at,
                  etag, last_modified, last_success_at, last_error_at, error_message, legacy_id,
                  created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, coalesce($14, now()))
             ON CONFLICT (legacy_id) DO NOTHING",
            feed.feed_url,
            filled(feed.site_url),
            title,
            filled(feed.icon),
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
        .execute(&mut *tx)
        .await?
        .rows_affected();
        report.feeds_added += added;
        report.feeds_known += 1 - added;
        titles.insert(feed.id, title);
    }
    tx.commit().await?;
    if without_url > 0 {
        report.notes.push(format!(
            "{without_url} feeds without an address were left out, with their articles"
        ));
    }

    let targets = sqlx::query!(
        r#"SELECT id, legacy_id AS "legacy_id!", last_seq, feed_url FROM feeds
           WHERE legacy_id IS NOT NULL"#
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .filter_map(|row| {
        // Feeds of an earlier run that the old database no longer has stay as they are.
        let title = titles.remove(&row.legacy_id)?;
        let target = Target {
            id: row.id,
            title: if title.is_empty() {
                row.feed_url
            } else {
                title
            },
            last_seq: row.last_seq,
            seen: 0,
            renumbered: false,
        };
        Some((row.legacy_id, target))
    })
    .collect();
    Ok(targets)
}

#[derive(sqlx::FromRow)]
struct OldArticle {
    id: Uuid,
    seq: i64,
    feed_id: Uuid,
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

// In the order of arrival, which is also how the articles come to lie on disk:
// what arrived together is read together.
const OLD_ARTICLES: &str = "
    SELECT id, seq, feed_id, site_guid,
           CASE WHEN created_at >= '1970-01-01' THEN created_at AT TIME ZONE 'UTC' END AS created_at,
           CASE WHEN posted_at >= '1970-01-01' THEN posted_at AT TIME ZONE 'UTC' END AS posted_at,
           link, thumbnail, image, image_title, title, teaser, content
    FROM articles
    WHERE seq > $1
    ORDER BY seq
    LIMIT $2";

/// An article of the batch that the new database gets.
struct Arrival {
    legacy_id: Uuid,
    feed_id: i32,
    seq: i32,
    /// The old `seq`.
    arrived: i64,
    article: NewArticle,
}

async fn import_articles(
    db: &PgPool,
    old: &mut PgConnection,
    feeds: &mut HashMap<Uuid, Target>,
    report: &mut Report,
) -> anyhow::Result<()> {
    // As long as every article of the new database came from the old one, the
    // order of arrival is the old one's. Once the new rueder has fetched
    // articles itself, what is imported after that takes its place behind them.
    let fetched_itself = sqlx::query_scalar!(
        r#"SELECT (SELECT count(*) FROM articles) > (SELECT count(*) FROM legacy_articles)
           AS "fetched_itself!""#
    )
    .fetch_one(db)
    .await?;

    let started = Instant::now();
    let now = OffsetDateTime::now_utc();
    let (mut after, mut without_feed, mut unreadable, mut batches) = (0_i64, 0_u64, 0_u64, 0_u64);
    loop {
        let rows: Vec<OldArticle> = sqlx::query_as(OLD_ARTICLES)
            .bind(after)
            .bind(BATCH)
            .fetch_all(&mut *old)
            .await
            .context("reading the old articles")?;
        let Some(last) = rows.last() else { break };
        after = last.seq;

        // What an earlier run brought, and where it went.
        let ids: Vec<_> = rows.iter().map(|row| row.id).collect();
        let known: HashMap<Uuid, i32> = sqlx::query!(
            "SELECT legacy_id, seq FROM legacy_articles WHERE legacy_id = ANY($1)",
            &ids
        )
        .fetch_all(db)
        .await?
        .into_iter()
        .map(|row| (row.legacy_id, row.seq))
        .collect();
        // What the new rueder fetched itself in the meantime, found by the
        // articles' ids in their feeds.
        let mut fetched: HashMap<(i32, Vec<u8>), i32> = HashMap::new();
        if fetched_itself {
            let (feed_ids, hashes): (Vec<_>, Vec<_>) = rows
                .iter()
                .filter(|row| !known.contains_key(&row.id))
                .filter_map(|row| Some((feeds.get(&row.feed_id)?.id, guid_hash(&row.site_guid))))
                .unzip();
            fetched = sqlx::query!(
                r#"SELECT c.feed_id, c.guid_hash, c.seq
                   FROM unnest($1::integer[], $2::bytea[]) AS n (feed_id, guid_hash)
                   JOIN article_content c ON c.feed_id = n.feed_id AND c.guid_hash = n.guid_hash"#,
                &feed_ids,
                &hashes,
            )
            .fetch_all(db)
            .await?
            .into_iter()
            .map(|row| ((row.feed_id, row.guid_hash), row.seq))
            .collect();
        }

        let mut arrivals = Vec::new();
        let mut matched = Vec::new();
        // The feeds that grow: their new last number and by how much.
        let mut grown: HashMap<i32, (i32, i32)> = HashMap::new();
        for row in rows {
            let Some(target) = feeds.get_mut(&row.feed_id) else {
                without_feed += 1;
                continue;
            };
            target.seen += 1;
            let seq = if let Some(&seq) = known.get(&row.id) {
                report.articles_known += 1;
                seq
            } else if let Some(&seq) = fetched.get(&(target.id, guid_hash(&row.site_guid))) {
                report.articles_matched += 1;
                matched.push((row.id, target.id, seq));
                seq
            } else {
                target.last_seq += 1;
                let feed = grown.entry(target.id).or_default();
                *feed = (target.last_seq, feed.1 + 1);
                let (content, readable) = lenient::<OldContent>(row.content);
                unreadable += u64::from(!readable);
                // The old rueder noted when it stored an article, which is as
                // close to the time of the fetch as it gets.
                let fetched_at = row.created_at.or(row.posted_at).unwrap_or(now);
                arrivals.push(Arrival {
                    legacy_id: row.id,
                    feed_id: target.id,
                    seq: target.last_seq,
                    arrived: row.seq,
                    article: NewArticle {
                        guid: row.site_guid,
                        published_at: row.posted_at.unwrap_or(fetched_at),
                        fetched_at,
                        original: Original {
                            title: row.title.unwrap_or_default(),
                            // The old rueder cleaned the text when it fetched it.
                            // The new cleaning runs over all of it once it is there.
                            content: content.text.clone(),
                            authors: content.authors,
                            tags: content.tags,
                            url: filled(row.link),
                        },
                        teaser: row.teaser.unwrap_or_default(),
                        thumbnail_url: filled(row.thumbnail),
                        html: content.text,
                        enclosures: content.enclosures.into(),
                        image_url: filled(row.image),
                        image_title: filled(row.image_title),
                    },
                });
                target.last_seq
            };
            target.renumbered |= seq != target.seen;
        }

        let mut tx = db.begin().await?;
        let (grown_ids, (grown_last, grown_by)): (Vec<_>, (Vec<_>, Vec<_>)) =
            grown.into_iter().unzip();
        // Numbers are given out by the feed's row. If it does not end where this
        // run counts to, somebody else is adding articles.
        let numbered = sqlx::query!(
            "UPDATE feeds f SET last_seq = n.last_seq
             FROM unnest($1::integer[], $2::integer[], $3::integer[]) AS n (feed_id, last_seq, by)
             WHERE f.id = n.feed_id AND f.last_seq + n.by = n.last_seq",
            &grown_ids,
            &grown_last,
            &grown_by,
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if numbered != grown_ids.len() as u64 {
            bail!(
                "a feed got articles from elsewhere during the import; \
                 stop the new rueder, then run the import again"
            );
        }
        let placed: Vec<_> = arrivals
            .iter()
            .map(|arrival| Placed {
                feed_id: arrival.feed_id,
                seq: arrival.seq,
                id: (!fetched_itself).then_some(arrival.arrived),
                article: &arrival.article,
            })
            .collect();
        articles::write(&mut tx, &placed).await?;
        let (legacy_ids, (feed_ids, seqs)): (Vec<_>, (Vec<_>, Vec<_>)) = arrivals
            .iter()
            .map(|a| (a.legacy_id, a.feed_id, a.seq))
            .chain(matched)
            .map(|(legacy_id, feed_id, seq)| (legacy_id, (feed_id, seq)))
            .unzip();
        sqlx::query!(
            "INSERT INTO legacy_articles (legacy_id, feed_id, seq)
             SELECT * FROM unnest($1::uuid[], $2::integer[], $3::integer[])",
            &legacy_ids,
            &feed_ids,
            &seqs,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        report.articles_added += arrivals.len() as u64;

        batches += 1;
        if batches.is_multiple_of(100) {
            tracing::info!(
                "{} articles added, {} already there, after {} s",
                report.articles_added,
                report.articles_known + report.articles_matched,
                started.elapsed().as_secs()
            );
        }
    }

    // New articles arrive behind the imported ones.
    sqlx::query!(
        "SELECT setval('articles_id_seq', greatest(
             (SELECT max(id) FROM articles), (SELECT last_value FROM articles_id_seq)))"
    )
    .fetch_one(db)
    .await?;

    let mut renumbered: Vec<_> = feeds
        .values()
        .filter(|target| target.renumbered)
        .map(|target| target.title.clone())
        .collect();
    renumbered.sort_unstable();
    report.renumbered_feeds = renumbered;
    if without_feed > 0 {
        report.notes.push(format!(
            "{without_feed} articles of feeds that were left out were left out"
        ));
    }
    if unreadable > 0 {
        report.notes.push(format!(
            "{unreadable} articles had content the import could not read; they came without text"
        ));
    }
    Ok(())
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

/// A subscription on its way into the new database.
struct NewSubscription {
    feed_id: i32,
    position: f64,
    /// The user's own name for the feed.
    title: Option<String>,
}

async fn import_accounts(
    db: &PgPool,
    old: &mut PgConnection,
    feeds: &HashMap<Uuid, Target>,
    report: &mut Report,
) -> anyhow::Result<()> {
    let users: Vec<OldUser> = sqlx::query_as(OLD_USERS)
        .fetch_all(&mut *old)
        .await
        .context("reading the old users")?;
    let mut subscribed: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    let user_feeds: Vec<(Uuid, Uuid)> =
        sqlx::query_as("SELECT user_id, feed_id FROM user_feeds ORDER BY id")
            .fetch_all(&mut *old)
            .await
            .context("reading the old subscriptions")?;
    for (user_id, feed_id) in user_feeds {
        subscribed.entry(user_id).or_default().push(feed_id);
    }

    let (mut unknown_feeds, mut renamed) = (0_u64, Vec::new());
    for user in users {
        let (folders, readable) = lenient::<Vec<OldFolder>>(user.folders);
        if !readable {
            report.notes.push(format!(
                "the folders of {} could not be read; the feeds came without folders",
                user.auth_subject
            ));
        }
        let in_no_folder = subscribed.remove(&user.id).unwrap_or_default();
        // Every feed once: where it stands first.
        let mut taken = HashSet::new();
        let mut subscription = |feed: Uuid, title: &str, position: usize| {
            let Some(target) = feeds.get(&feed) else {
                unknown_feeds += 1;
                return None;
            };
            taken.insert(target.id).then(|| NewSubscription {
                feed_id: target.id,
                position: position as f64,
                title: (!title.is_empty() && title != target.title).then(|| title.to_owned()),
            })
        };

        let mut tx = db.begin().await?;
        let account = sqlx::query_scalar!("SELECT id FROM accounts WHERE legacy_id = $1", user.id)
            .fetch_optional(&mut *tx)
            .await?;
        if let Some(account_id) = account {
            // The account's tree is its own by now. What it does not subscribe to
            // yet goes to the end of the top level.
            report.accounts_known += 1;
            let end = sqlx::query_scalar!(
                r#"SELECT coalesce(max(position), 0) AS "end!" FROM (
                       SELECT position FROM folders WHERE account_id = $1 AND parent_id IS NULL
                       UNION ALL
                       SELECT position FROM subscriptions
                       WHERE account_id = $1 AND folder_id IS NULL
                   ) top"#,
                account_id
            )
            .fetch_one(&mut *tx)
            .await?;
            let missing: Vec<_> = folders
                .iter()
                .flat_map(|folder| &folder.feeds)
                .map(|feed| (feed.id, feed.title.as_str()))
                .chain(in_no_folder.iter().map(|&feed| (feed, "")))
                .enumerate()
                .filter_map(|(n, (feed, title))| subscription(feed, title, end as usize + n + 1))
                .collect();
            let missing = subscribe(&mut tx, account_id, None, missing).await?;
            report.subscriptions_added += missing;
        } else {
            let username = free_username(&mut tx, &user.auth_subject).await?;
            if username != user.auth_subject {
                renamed.push(format!("{} is now {username}", user.auth_subject));
            }
            let account_id = sqlx::query_scalar!(
                "INSERT INTO accounts (username, legacy_id, created_at)
                 VALUES ($1, $2, coalesce($3, now())) RETURNING id",
                username,
                user.id,
                user.created_at,
            )
            .fetch_one(&mut *tx)
            .await?;
            // How the account logged in so far. A password comes from the
            // command line; the old hashes are not taken over.
            sqlx::query!(
                "INSERT INTO account_identities (issuer, subject, account_id) VALUES ($1, $2, $3)
                 ON CONFLICT DO NOTHING",
                user.auth_origin,
                user.auth_subject,
                account_id,
            )
            .execute(&mut *tx)
            .await?;
            report.accounts_added += 1;

            // Folders and the feeds in them stand in the order of the old JSON,
            // feeds that are in no folder follow the folders on the top level.
            for (n, folder) in folders.iter().enumerate() {
                let title = match folder.title.trim() {
                    "" => format!("Folder {}", n + 1),
                    title => title.to_owned(),
                };
                let folder_id = sqlx::query_scalar!(
                    "INSERT INTO folders (account_id, title, position) VALUES ($1, $2, $3)
                     RETURNING id",
                    account_id,
                    title,
                    (n + 1) as f64,
                )
                .fetch_one(&mut *tx)
                .await?;
                report.folders_added += 1;
                let mut place = 0;
                let inside: Vec<_> = folder
                    .feeds
                    .iter()
                    .filter_map(|feed| {
                        let new = subscription(feed.id, &feed.title, place + 1)?;
                        place += 1;
                        Some(new)
                    })
                    .collect();
                report.subscriptions_added +=
                    subscribe(&mut tx, account_id, Some(folder_id), inside).await?;
            }
            let mut place = folders.len();
            let top: Vec<_> = in_no_folder
                .iter()
                .filter_map(|&feed| {
                    let new = subscription(feed, "", place + 1)?;
                    place += 1;
                    Some(new)
                })
                .collect();
            report.subscriptions_added += subscribe(&mut tx, account_id, None, top).await?;
        }
        tx.commit().await?;
    }

    if unknown_feeds > 0 {
        report.notes.push(format!(
            "{unknown_feeds} subscriptions to feeds that do not exist were left out"
        ));
    }
    if !renamed.is_empty() {
        report.notes.push(format!(
            "login names that were taken or empty got another one: {}",
            renamed.join(", ")
        ));
    }
    Ok(())
}

/// Subscribes an account to feeds and returns how many it did not subscribe to
/// before. Everything the feeds hold counts as read: what was read is known only
/// to the browsers until the switch, and nobody wants to start with every
/// article of ten years unread.
async fn subscribe(
    conn: &mut PgConnection,
    account_id: i32,
    folder_id: Option<i32>,
    subscriptions: Vec<NewSubscription>,
) -> anyhow::Result<u64> {
    let feed_ids: Vec<_> = subscriptions.iter().map(|s| s.feed_id).collect();
    let positions: Vec<_> = subscriptions.iter().map(|s| s.position).collect();
    let titles: Vec<_> = subscriptions.into_iter().map(|s| s.title).collect();
    let added = sqlx::query!(
        "INSERT INTO subscriptions (account_id, feed_id, folder_id, position, title, read_upto)
         SELECT $1, f.id, $2, n.position, n.title, f.last_seq
         FROM unnest($3::integer[], $4::float8[], $5::text[]) AS n (feed_id, position, title)
         JOIN feeds f ON f.id = n.feed_id
         ON CONFLICT (account_id, feed_id) DO NOTHING",
        account_id,
        folder_id,
        &feed_ids,
        &positions,
        &titles as &[Option<String>],
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();
    Ok(added)
}

/// The login name an old user gets: its old one, with a number where that is
/// taken. Login names do not tell upper from lower case.
async fn free_username(conn: &mut PgConnection, wanted: &str) -> anyhow::Result<String> {
    let wanted = match wanted.trim() {
        "" => "user",
        wanted => wanted,
    };
    let mut name = wanted.to_owned();
    let mut n = 1;
    loop {
        let taken = sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM accounts WHERE lower(username) = lower($1)) AS "taken!""#,
            name
        )
        .fetch_one(&mut *conn)
        .await?;
        if !taken {
            return Ok(name);
        }
        n += 1;
        name = format!("{wanted}-{n}");
    }
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
