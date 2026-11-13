//! How long the server keeps what it keeps: set by the operator with `rueder
//! retention set`, shown with `rueder retention show`, and kept in the one row
//! of the table `retention`, so that a change counts at once, without a restart.
//!
//! Two things are kept for a time. The status of the server ([`Settings`],
//! see [`crate::history`]) for so many days, seven as rueder comes. And what
//! is old of the articles ([`Articles`], work package 5.3, A21, E4), which as
//! rueder comes stays for ever. Of that, once an hour [`sweep`] clears in small
//! portions, each in a transaction of its own, so that fetching never waits
//! for long:
//!
//! - articles that arrived more than so many months ago, as the operator
//!   chose either whole or only their text, the line in the lists staying;
//! - versions, by whole months, as `article_versions` is partitioned by them;
//! - feeds nobody subscribed to for so many days, with their articles.
//!
//! Articles with a star or a label of any account stay, with their text.
//!
//! Numbers are never given out again, and unread counts go on being worked out
//! from the read mark and its exceptions, without counting articles. For that,
//! a feed is cleared from its oldest article up to a number, `feeds.cut_seq`,
//! and every read mark of the feed moves up to it in the same transaction.
//! What the cut leaves standing, the starred and labelled, keeps its state for
//! every reader as an exception from the new mark. A new subscription starts at
//! the cut too (a trigger in the database sees to it, whoever subscribes), and
//! the ids of the articles that went are kept, so that a feed that still lists
//! one does not bring it back as new.

use std::{collections::HashSet, fmt, ops::RangeInclusive, str::FromStr, time::Instant};

use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;

use crate::events::{Change, Events};

/// How many months an article or a version may be kept at most before it goes,
/// when the operator sets it.
pub const MONTHS: RangeInclusive<i32> = 1..=1200;
/// How many days a feed may be without subscribers before it goes, when set.
pub const ORPHAN_DAYS: RangeInclusive<i32> = 1..=36500;
/// How many articles one transaction clears at most.
pub const PORTION: i64 = 500;
/// What PostgreSQL answers when dropping a month of versions waited longer for
/// the table than it may (see `drop_versions_partitions`): left to the next run.
const LOCK_NOT_AVAILABLE: &str = "55P03";

/// The fewest and the most days the status is kept for.
pub const DAYS: RangeInclusive<i32> = 1..=3650;

/// How long the status is kept, as the operator set it.
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

/// What goes of an article that is old.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
pub enum Remove {
    /// All of it: it leaves the lists, the counts and the search.
    Article,
    /// Its text: the line in the lists stays as it was, and opening it says
    /// that the text is no longer kept.
    Text,
}

impl FromStr for Remove {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "article" => Ok(Self::Article),
            "text" => Ok(Self::Text),
            _ => Err("`article` or `text`".into()),
        }
    }
}

impl fmt::Display for Remove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Article => "article",
            Self::Text => "text",
        })
    }
}

/// How long articles, their versions and feeds nobody reads are kept, as the
/// operator set it. `None` keeps for ever, which is what rueder comes with.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Articles {
    /// After how many months since they arrived articles go.
    pub articles_months: Option<i32>,
    /// What goes of them then; set exactly when `articles_months` is.
    pub articles_what: Option<Remove>,
    /// After how many months versions go.
    pub versions_months: Option<i32>,
    /// After how many days without subscribers a feed goes.
    pub orphans_days: Option<i32>,
}

impl Articles {
    /// Whether anything is ever cleared.
    pub fn clears(&self) -> bool {
        self.articles_months.is_some()
            || self.versions_months.is_some()
            || self.orphans_days.is_some()
    }

    /// Why these settings cannot be stored, if they cannot.
    pub fn check(&self) -> Result<(), String> {
        let within = |value: Option<i32>, bounds: &RangeInclusive<i32>, what: &str| match value {
            Some(value) if !bounds.contains(&value) => Err(format!(
                "{what} must be from {} to {}, not {value}",
                bounds.start(),
                bounds.end()
            )),
            _ => Ok(()),
        };
        within(self.articles_months, &MONTHS, "the months of articles")?;
        within(self.versions_months, &MONTHS, "the months of versions")?;
        within(
            self.orphans_days,
            &DAYS,
            "the days of feeds without subscribers",
        )?;
        match (self.articles_months, self.articles_what) {
            (Some(_), None) => Err("say what goes of an old article: `article` or `text`".into()),
            (None, Some(_)) => Err("say after how many months articles go".into()),
            _ => Ok(()),
        }
    }
}

pub async fn articles(db: &PgPool) -> anyhow::Result<Articles> {
    Ok(sqlx::query_as!(
        Articles,
        r#"SELECT articles_months, articles_what AS "articles_what: Remove", versions_months,
                  orphans_days
           FROM retention"#
    )
    .fetch_one(db)
    .await?)
}

/// Stores how long articles are kept; the next run goes by it. Fails if it
/// does not hold together, see [`Articles::check`].
pub async fn set_articles(db: &PgPool, settings: &Articles) -> anyhow::Result<()> {
    settings.check().map_err(anyhow::Error::msg)?;
    sqlx::query!(
        "UPDATE retention
         SET articles_months = $1, articles_what = $2, versions_months = $3, orphans_days = $4",
        settings.articles_months,
        settings.articles_what as Option<Remove>,
        settings.versions_months,
        settings.orphans_days,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// What a run cleared.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Swept {
    /// Articles that went whole.
    pub articles: u64,
    /// Articles whose text went.
    pub texts: u64,
    /// The months of versions that went, as `2024_10`.
    pub months: Vec<String>,
    /// Feeds without subscribers that went, with their articles.
    pub feeds: u64,
    /// The articles that went with feeds nobody subscribes to.
    pub feed_articles: u64,
    /// Not all is done: the time was up, or a month of versions had to wait.
    pub more: bool,
}

impl Swept {
    pub fn is_empty(&self) -> bool {
        self.articles == 0
            && self.texts == 0
            && self.months.is_empty()
            && self.feeds == 0
            && self.feed_articles == 0
    }
}

/// Clears what the settings say is old at `now`, one portion after the other,
/// until nothing is left or `until` has passed. The readers of a feed whose
/// articles went are told, so that their open tabs count anew.
pub async fn sweep(
    db: &PgPool,
    events: Option<&Events>,
    now: OffsetDateTime,
    until: Option<Instant>,
) -> anyhow::Result<Swept> {
    let settings = articles(db).await?;
    let mut swept = Swept::default();
    if !settings.clears() {
        return Ok(swept);
    }
    let time_left = || until.is_none_or(|until| Instant::now() < until);

    // First what is known to be without readers, then the rest.
    if let Some(days) = settings.orphans_days {
        note_orphans(db, now).await?;
        let before = now - time::Duration::days(i64::from(days));
        for feed_id in orphans(db, before).await? {
            if !time_left() {
                swept.more = true;
                break;
            }
            remove_orphan(db, feed_id, &mut swept, &time_left).await?;
        }
    }

    if let (Some(months), Some(what)) = (settings.articles_months, settings.articles_what) {
        let before = months_before(db, now, months).await?;
        for feed_id in feeds_to_cut(db, what, before).await? {
            if !time_left() {
                swept.more = true;
                break;
            }
            let mut gone = 0;
            loop {
                let cleared = match what {
                    Remove::Article => cut_articles(db, feed_id, before).await?,
                    Remove::Text => drop_texts(db, feed_id, before).await?,
                };
                let Some(cleared) = cleared else { break };
                gone += cleared.count;
                if !cleared.more {
                    break;
                }
                if !time_left() {
                    swept.more = true;
                    break;
                }
            }
            match what {
                Remove::Article => swept.articles += gone,
                Remove::Text => swept.texts += gone,
            }
            // Counts and lists of the readers change only where whole articles went.
            if gone > 0
                && what == Remove::Article
                && let Some(events) = events
            {
                let change = Change::Reading {
                    feed: None,
                    seq: None,
                    by: None,
                };
                events.publish_to_readers(db, feed_id, change).await?;
            }
        }
    }

    if let Some(months) = settings.versions_months {
        let before = months_before(db, now, months).await?;
        match drop_months(db, before).await {
            Ok(months) => swept.months = months,
            Err(err) if is_lock_timeout(&err) => {
                tracing::warn!("dropping old versions had to wait for the table; next time");
                swept.more = true;
            }
            Err(err) => return Err(err),
        }
    }
    Ok(swept)
}

/// The time so many months before `now`, as PostgreSQL counts months.
async fn months_before(
    db: &PgPool,
    now: OffsetDateTime,
    months: i32,
) -> anyhow::Result<OffsetDateTime> {
    Ok(sqlx::query_scalar!(
        r#"SELECT $1::timestamptz - make_interval(months => $2) AS "before!""#,
        now,
        months,
    )
    .fetch_one(db)
    .await?)
}

/// The feeds whose oldest article above the cut arrived before `before`.
async fn feeds_to_cut(
    db: &PgPool,
    what: Remove,
    before: OffsetDateTime,
) -> anyhow::Result<Vec<i32>> {
    let feeds = match what {
        Remove::Article => {
            sqlx::query_scalar!(
                "SELECT f.id FROM feeds f
                 CROSS JOIN LATERAL (
                     SELECT a.arrived_at FROM articles a
                     WHERE a.feed_id = f.id AND a.seq > f.cut_seq
                     ORDER BY a.seq LIMIT 1) oldest
                 WHERE oldest.arrived_at < $1
                 ORDER BY f.id",
                before,
            )
            .fetch_all(db)
            .await?
        }
        Remove::Text => {
            sqlx::query_scalar!(
                "SELECT f.id FROM feeds f
                 CROSS JOIN LATERAL (
                     SELECT a.arrived_at FROM articles a
                     WHERE a.feed_id = f.id AND a.seq > f.text_cut_seq
                     ORDER BY a.seq LIMIT 1) oldest
                 WHERE oldest.arrived_at < $1
                 ORDER BY f.id",
                before,
            )
            .fetch_all(db)
            .await?
        }
    };
    Ok(feeds)
}

/// What one portion cleared.
struct Cleared {
    /// How many articles went, or lost their text.
    count: u64,
    /// The portion was full, so there may be more.
    more: bool,
}

/// The oldest articles above the cut that arrived before `before`, at most a
/// portion, locked: whoever labels one of them meanwhile waits, and is then
/// seen. With the subscriptions locked, nobody stars one either. `from` is the
/// cut.
async fn oldest(
    conn: &mut PgConnection,
    feed_id: i32,
    from: i32,
    before: OffsetDateTime,
) -> anyhow::Result<(Vec<i32>, bool)> {
    let rows = sqlx::query!(
        "SELECT seq, arrived_at FROM articles
         WHERE feed_id = $1 AND seq > $2
         ORDER BY seq LIMIT $3
         FOR UPDATE",
        feed_id,
        from,
        PORTION,
    )
    .fetch_all(&mut *conn)
    .await?;
    let full = rows.len() as i64 == PORTION;
    let old: Vec<i32> = rows
        .iter()
        .take_while(|row| row.arrived_at < before)
        .map(|row| row.seq)
        .collect();
    // More can only follow if the whole portion was old.
    let more = full && old.len() == rows.len();
    Ok((old, more))
}

/// Of these articles, those a reader starred or labelled: they stay.
async fn marked(
    conn: &mut PgConnection,
    feed_id: i32,
    accounts: &[i32],
    seqs: &[i32],
) -> anyhow::Result<HashSet<i32>> {
    let seqs = sqlx::query_scalar!(
        r#"SELECT seq AS "seq!" FROM article_states
           WHERE account_id = ANY($1) AND feed_id = $2 AND seq = ANY($3) AND starred
           UNION
           SELECT seq FROM article_labels WHERE feed_id = $2 AND seq = ANY($3)"#,
        accounts,
        feed_id,
        seqs,
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok(seqs.into_iter().collect())
}

/// Deletes the oldest articles of a feed above its cut that arrived before
/// `before`, a portion at most, and moves the cut and every read mark of the
/// feed over them. `None` if there was nothing to do.
async fn cut_articles(
    db: &PgPool,
    feed_id: i32,
    before: OffsetDateTime,
) -> anyhow::Result<Option<Cleared>> {
    let mut tx = db.begin().await?;
    // Like a fetch, which waits meanwhile, and so does a new subscription.
    let Some(cut) = sqlx::query_scalar!(
        "SELECT cut_seq FROM feeds WHERE id = $1 FOR UPDATE",
        feed_id
    )
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(None);
    };
    // What readers do to the feed's articles waits too.
    let accounts = sqlx::query_scalar!(
        "SELECT account_id FROM subscriptions WHERE feed_id = $1 ORDER BY account_id FOR UPDATE",
        feed_id
    )
    .fetch_all(&mut *tx)
    .await?;
    let (old, more) = oldest(&mut tx, feed_id, cut, before).await?;
    let Some(&upto) = old.last() else {
        return Ok(None);
    };
    let stay: Vec<i32> = {
        let marked = marked(&mut tx, feed_id, &accounts, &old).await?;
        old.iter()
            .copied()
            .filter(|seq| marked.contains(seq))
            .collect()
    };
    let go: Vec<i32> = old
        .iter()
        .copied()
        .filter(|seq| !stay.contains(seq))
        .collect();

    // A reader whose mark is below the new cut: the starred and labelled ones
    // that stay and were unread for them stay unread, as exceptions below the
    // mark. Those read or hidden need no row for that.
    sqlx::query!(
        "INSERT INTO article_states (account_id, feed_id, seq, kept_unread)
         SELECT s.account_id, s.feed_id, k.seq, true
         FROM subscriptions s CROSS JOIN unnest($2::int4[]) AS k (seq)
         WHERE s.feed_id = $1 AND k.seq > s.read_upto AND k.seq <= $3
         ON CONFLICT (account_id, feed_id, seq) DO UPDATE SET kept_unread = true
         WHERE NOT article_states.read AND NOT article_states.hidden",
        feed_id,
        &stay,
        upto,
    )
    .execute(&mut *tx)
    .await?;
    // What readers kept of the articles that go goes with them.
    sqlx::query!(
        "DELETE FROM article_states WHERE account_id = ANY($1) AND feed_id = $2 AND seq = ANY($3)",
        &accounts,
        feed_id,
        &go,
    )
    .execute(&mut *tx)
    .await?;
    // Read above the old mark is what the new mark says anyway.
    sqlx::query!(
        "DELETE FROM article_states st USING subscriptions s
         WHERE s.feed_id = $1 AND st.account_id = s.account_id AND st.feed_id = s.feed_id
           AND st.seq > s.read_upto AND st.seq <= $2
           AND st.read AND NOT st.kept_unread AND NOT st.hidden AND NOT st.starred",
        feed_id,
        upto,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE article_states st SET read = false FROM subscriptions s
         WHERE s.feed_id = $1 AND st.account_id = s.account_id AND st.feed_id = s.feed_id
           AND st.seq > s.read_upto AND st.seq <= $2 AND st.read",
        feed_id,
        upto,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE subscriptions SET read_upto = $2 WHERE feed_id = $1 AND read_upto < $2",
        feed_id,
        upto,
    )
    .execute(&mut *tx)
    .await?;

    // The cut moves, and what a move of an account says of a mark there.
    cut_at(&mut tx, feed_id, upto).await?;
    // So that a feed that still lists one does not bring it back.
    sqlx::query!(
        "INSERT INTO article_tombstones (feed_id, guid_hash)
         SELECT feed_id, guid_hash FROM article_content WHERE feed_id = $1 AND seq = ANY($2)
         ON CONFLICT DO NOTHING",
        feed_id,
        &go,
    )
    .execute(&mut *tx)
    .await?;
    let gone = sqlx::query!(
        "DELETE FROM articles WHERE feed_id = $1 AND seq = ANY($2)",
        feed_id,
        &go,
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    tx.commit().await?;
    Ok(Some(Cleared { count: gone, more }))
}

/// Moves a feed's cut up to `upto` and notes the article there, while it is
/// still there: a read mark at the cut names it when the account moves to
/// another rueder (see `transfer`), which finds it by its id or its time.
async fn cut_at(conn: &mut PgConnection, feed_id: i32, upto: i32) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE feeds f
         SET cut_seq = $2, cut_guid = c.guid, cut_url = a.url, cut_published_at = a.published_at
         FROM articles a JOIN article_content c ON c.feed_id = a.feed_id AND c.seq = a.seq
         WHERE f.id = $1 AND a.feed_id = $1 AND a.seq = $2 AND f.cut_seq < $2",
        feed_id,
        upto,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Takes the text of the oldest articles of a feed above its text cut that
/// arrived before `before`, a portion at most, with their versions; their line
/// in the lists stays as it was. `None` if there was nothing to do.
async fn drop_texts(
    db: &PgPool,
    feed_id: i32,
    before: OffsetDateTime,
) -> anyhow::Result<Option<Cleared>> {
    let mut tx = db.begin().await?;
    let Some(cut) = sqlx::query_scalar!(
        "SELECT text_cut_seq FROM feeds WHERE id = $1 FOR UPDATE",
        feed_id
    )
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(None);
    };
    // Nobody stars one meanwhile; read state does not change.
    let accounts = sqlx::query_scalar!(
        "SELECT account_id FROM subscriptions WHERE feed_id = $1 ORDER BY account_id FOR SHARE",
        feed_id
    )
    .fetch_all(&mut *tx)
    .await?;
    let (old, more) = oldest(&mut tx, feed_id, cut, before).await?;
    let Some(&upto) = old.last() else {
        return Ok(None);
    };
    let marked = marked(&mut tx, feed_id, &accounts, &old).await?;
    let go: Vec<i32> = old
        .into_iter()
        .filter(|seq| !marked.contains(seq))
        .collect();
    // The text and what the search finds in it; tags, files and the picture
    // are small and stay, so a podcast's episode can still be played.
    let gone = sqlx::query!(
        "UPDATE article_content SET html = '', text_dropped = true
         WHERE feed_id = $1 AND seq = ANY($2) AND NOT text_dropped",
        feed_id,
        &go,
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    // The versions are the text as it was fetched.
    sqlx::query!(
        "DELETE FROM article_versions WHERE feed_id = $1 AND seq = ANY($2)",
        feed_id,
        &go,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE feeds SET text_cut_seq = $2 WHERE id = $1",
        feed_id,
        upto
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Some(Cleared { count: gone, more }))
}

/// Notes which feeds have no subscribers since when, as far as this run sees,
/// and forgets it of those that have some again.
async fn note_orphans(db: &PgPool, now: OffsetDateTime) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE feeds f SET orphaned_at = $1
         WHERE orphaned_at IS NULL AND NOT EXISTS (SELECT 1 FROM subscriptions s WHERE s.feed_id = f.id)",
        now,
    )
    .execute(db)
    .await?;
    sqlx::query!(
        "UPDATE feeds f SET orphaned_at = NULL
         WHERE orphaned_at IS NOT NULL AND EXISTS (SELECT 1 FROM subscriptions s WHERE s.feed_id = f.id)"
    )
    .execute(db)
    .await?;
    Ok(())
}

/// The feeds without subscribers since before `before`.
async fn orphans(db: &PgPool, before: OffsetDateTime) -> anyhow::Result<Vec<i32>> {
    Ok(sqlx::query_scalar!(
        "SELECT id FROM feeds WHERE orphaned_at < $1 ORDER BY id",
        before
    )
    .fetch_all(db)
    .await?)
}

/// Deletes a feed nobody subscribes to, its oldest articles first, a portion
/// in each transaction, and at last the feed. A blocked feed keeps its row, so
/// that the block stays: nobody subscribes to its address anew. Whoever
/// subscribes meanwhile keeps the feed and what is left of it, from the cut on.
async fn remove_orphan(
    db: &PgPool,
    feed_id: i32,
    swept: &mut Swept,
    time_left: &impl Fn() -> bool,
) -> anyhow::Result<()> {
    loop {
        let mut tx = db.begin().await?;
        let Some(blocked) = sqlx::query_scalar!(
            r#"SELECT blocked_at IS NOT NULL AS "blocked!" FROM feeds WHERE id = $1 FOR UPDATE"#,
            feed_id
        )
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(());
        };
        // A subscription that came meanwhile has been waiting for the lock
        // above, and is seen now.
        let read = sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM subscriptions WHERE feed_id = $1) AS "read!""#,
            feed_id
        )
        .fetch_one(&mut *tx)
        .await?;
        if read {
            sqlx::query!("UPDATE feeds SET orphaned_at = NULL WHERE id = $1", feed_id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            return Ok(());
        }
        let portion = sqlx::query_scalar!(
            "SELECT seq FROM articles WHERE feed_id = $1 ORDER BY seq LIMIT $2",
            feed_id,
            PORTION,
        )
        .fetch_all(&mut *tx)
        .await?;
        if let Some(&upto) = portion.last() {
            // Whoever subscribes later starts here, as after any cut.
            cut_at(&mut tx, feed_id, upto).await?;
            sqlx::query!(
                "UPDATE feeds SET text_cut_seq = greatest(text_cut_seq, $2) WHERE id = $1",
                feed_id,
                upto,
            )
            .execute(&mut *tx)
            .await?;
            let gone = sqlx::query!(
                "DELETE FROM articles WHERE feed_id = $1 AND seq = ANY($2)",
                feed_id,
                &portion,
            )
            .execute(&mut *tx)
            .await?
            .rows_affected();
            tx.commit().await?;
            swept.feed_articles += gone;
            if !time_left() {
                swept.more = true;
                return Ok(());
            }
            continue;
        }
        if blocked {
            // Empty, and blocked for good: nothing to count it as gone.
            sqlx::query!("UPDATE feeds SET orphaned_at = NULL WHERE id = $1", feed_id)
                .execute(&mut *tx)
                .await?;
        } else {
            sqlx::query!("DELETE FROM feeds WHERE id = $1", feed_id)
                .execute(&mut *tx)
                .await?;
            swept.feeds += 1;
        }
        tx.commit().await?;
        return Ok(());
    }
}

/// Drops the months of versions that lie wholly before `before`.
async fn drop_months(db: &PgPool, before: OffsetDateTime) -> anyhow::Result<Vec<String>> {
    let mut tx = db.begin().await?;
    let dropped = sqlx::query_scalar!(
        r#"SELECT part AS "part!" FROM drop_versions_partitions($1) AS part"#,
        before
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(dropped
        .into_iter()
        .map(|name| name.trim_start_matches("article_versions_").to_owned())
        .collect())
}

/// Whether the error is PostgreSQL giving up on a lock it waited for.
fn is_lock_timeout(err: &anyhow::Error) -> bool {
    err.downcast_ref::<sqlx::Error>()
        .and_then(|err| err.as_database_error())
        .and_then(|err| err.code())
        .is_some_and(|code| code == LOCK_NOT_AVAILABLE)
}

/// How much the retention has to look after, for `rueder retention show`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Kept {
    /// Ids of articles that went whole, kept so that they do not come back.
    pub tombstones: i64,
    /// Articles whose text went.
    pub dropped_texts: i64,
    /// Feeds without subscribers, as the last run saw them.
    pub orphans: i64,
    /// The months of versions there are, oldest first.
    pub months: Vec<String>,
}

pub async fn kept(db: &PgPool) -> anyhow::Result<Kept> {
    let row = sqlx::query!(
        r#"SELECT (SELECT count(*) FROM article_tombstones) AS "tombstones!",
                  (SELECT count(*) FROM article_content WHERE text_dropped) AS "dropped!",
                  (SELECT count(*) FROM feeds WHERE orphaned_at IS NOT NULL) AS "orphans!""#
    )
    .fetch_one(db)
    .await?;
    let months = sqlx::query_scalar!(
        r#"SELECT c.relname AS "name!" FROM pg_inherits i JOIN pg_class c ON c.oid = i.inhrelid
           WHERE i.inhparent = 'article_versions'::regclass
           ORDER BY c.relname"#
    )
    .fetch_all(db)
    .await?;
    Ok(Kept {
        tombstones: row.tombstones,
        dropped_texts: row.dropped,
        orphans: row.orphans,
        months: months
            .into_iter()
            .map(|name| name.trim_start_matches("article_versions_").to_owned())
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_goes_is_named_and_read_back() {
        for remove in [Remove::Article, Remove::Text] {
            assert_eq!(remove.to_string().parse::<Remove>(), Ok(remove));
        }
        assert!("everything".parse::<Remove>().is_err());
    }

    #[test]
    fn settings_hold_together() {
        assert_eq!(Articles::default().check(), Ok(()));
        assert!(!Articles::default().clears());
        let articles = Articles {
            articles_months: Some(24),
            articles_what: Some(Remove::Text),
            ..Articles::default()
        };
        assert_eq!(articles.check(), Ok(()));
        assert!(articles.clears());
        // What goes has to be said, and only with a time.
        let unsaid = Articles {
            articles_what: None,
            ..articles
        };
        assert!(unsaid.check().is_err());
        let untimed = Articles {
            articles_months: None,
            ..articles
        };
        assert!(untimed.check().is_err());
        for wrong in [
            Articles {
                versions_months: Some(0),
                ..Articles::default()
            },
            Articles {
                versions_months: Some(1201),
                ..Articles::default()
            },
            Articles {
                orphans_days: Some(0),
                ..Articles::default()
            },
        ] {
            assert!(wrong.check().is_err(), "{wrong:?}");
        }
    }
}
