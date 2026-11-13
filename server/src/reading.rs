//! What an account has read, and how many articles that leaves unread.
//!
//! Per subscription there is a mark ("everything up to number N is read",
//! `subscriptions.read_upto`) and the exceptions from it in `article_states`:
//! read above the mark, kept unread at or below it, hidden by a filter rule,
//! starred. The unread count is worked out from these and the feed's newest
//! number every time. No counter is stored, so nothing can drift.
//!
//! The same model runs in the browser (`web/src/lib/unread.ts`), so a change shows
//! at once and the server only has to confirm it.

use serde::Serialize;
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use ts_rs::TS;
use utoipa::ToSchema;

/// How far the mark looks ahead in one go when it moves up.
const COMPACT_CHUNK: i64 = 1000;

/// A subscription's numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS, ToSchema)]
pub struct FeedUnread {
    pub feed_id: i32,
    /// The number of the feed's newest article.
    pub last_seq: i32,
    pub read_upto: i32,
    pub unread: i64,
}

/// The unread count of every feed the account subscribes to. A folder's count
/// is the sum over its feeds.
pub async fn unread_counts(db: &PgPool, account_id: i32) -> anyhow::Result<Vec<FeedUnread>> {
    let counts = sqlx::query_as!(
        FeedUnread,
        r#"
        SELECT s.feed_id, f.last_seq, s.read_upto,
               f.last_seq - s.read_upto
               - (SELECT count(*) FROM article_states x
                  WHERE x.account_id = s.account_id AND x.feed_id = s.feed_id
                    AND x.seq > s.read_upto AND (x.read OR x.hidden))
               + (SELECT count(*) FROM article_states x
                  WHERE x.account_id = s.account_id AND x.feed_id = s.feed_id
                    AND x.seq <= s.read_upto AND x.kept_unread AND NOT x.hidden)
               AS "unread!"
        FROM subscriptions s
        JOIN feeds f ON f.id = s.feed_id
        WHERE s.account_id = $1
        ORDER BY s.feed_id
        "#,
        account_id
    )
    .fetch_all(db)
    .await?;
    Ok(counts)
}

/// One subscription's numbers, or `None` if the account does not subscribe to
/// the feed.
pub async fn unread_of(
    db: &PgPool,
    account_id: i32,
    feed_id: i32,
) -> anyhow::Result<Option<FeedUnread>> {
    let counts = sqlx::query_as!(
        FeedUnread,
        r#"
        SELECT s.feed_id, f.last_seq, s.read_upto,
               f.last_seq - s.read_upto
               - (SELECT count(*) FROM article_states x
                  WHERE x.account_id = s.account_id AND x.feed_id = s.feed_id
                    AND x.seq > s.read_upto AND (x.read OR x.hidden))
               + (SELECT count(*) FROM article_states x
                  WHERE x.account_id = s.account_id AND x.feed_id = s.feed_id
                    AND x.seq <= s.read_upto AND x.kept_unread AND NOT x.hidden)
               AS "unread!"
        FROM subscriptions s
        JOIN feeds f ON f.id = s.feed_id
        WHERE s.account_id = $1 AND s.feed_id = $2
        "#,
        account_id,
        feed_id
    )
    .fetch_optional(db)
    .await?;
    Ok(counts)
}

/// A subscription, locked for the rest of the transaction.
struct Locked {
    /// Its mark.
    mark: i32,
    /// The feed's newest number.
    last: i32,
    /// Up to where the retention cleared the feed (`crate::retention`): at or
    /// below it only the articles it kept are still there.
    cut: i32,
}

impl Locked {
    /// Whether the feed has an article of this number: every one above the
    /// cut up to the newest, and at or below it those the retention kept.
    async fn has(&self, conn: &mut PgConnection, feed_id: i32, seq: i32) -> anyhow::Result<bool> {
        if seq < 1 || seq > self.last {
            return Ok(false);
        }
        if seq > self.cut {
            return Ok(true);
        }
        Ok(sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM articles WHERE feed_id = $1 AND seq = $2) AS "has!""#,
            feed_id,
            seq,
        )
        .fetch_one(conn)
        .await?)
    }
}

/// Locks the subscription for the rest of the transaction, or `None` if the
/// account does not subscribe to the feed.
async fn lock(
    conn: &mut PgConnection,
    account_id: i32,
    feed_id: i32,
) -> anyhow::Result<Option<Locked>> {
    let row = sqlx::query!(
        "SELECT s.read_upto, f.last_seq, f.cut_seq
         FROM subscriptions s JOIN feeds f ON f.id = s.feed_id
         WHERE s.account_id = $1 AND s.feed_id = $2
         FOR UPDATE OF s",
        account_id,
        feed_id
    )
    .fetch_optional(&mut *conn)
    .await?;
    Ok(row.map(|row| Locked {
        mark: row.read_upto,
        last: row.last_seq,
        cut: row.cut_seq,
    }))
}

/// Changes flags of one article's state. The row comes and goes with its flags,
/// and the table's own check wants at least one set, so clearing the last flag is
/// a delete.
async fn set_flags(
    conn: &mut PgConnection,
    account_id: i32,
    feed_id: i32,
    seq: i32,
    change: Change,
) -> anyhow::Result<()> {
    let (read, kept_unread, hidden, starred) = match change {
        Change::Read(on) => (Some(on), None, None, None),
        Change::KeptUnread(on) => (None, Some(on), None, None),
        Change::Hidden(on) => (None, None, Some(on), None),
        Change::Starred(on) => (None, None, None, Some(on)),
    };
    // Delete first where the change would leave nothing, then upsert the rest.
    let deleted = sqlx::query!(
        "DELETE FROM article_states
         WHERE account_id = $1 AND feed_id = $2 AND seq = $3
           AND NOT (coalesce($4, read) OR coalesce($5, kept_unread)
                    OR coalesce($6, hidden) OR coalesce($7, starred))",
        account_id,
        feed_id,
        seq,
        read,
        kept_unread,
        hidden,
        starred,
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();
    let any = [read, kept_unread, hidden, starred].contains(&Some(true));
    if deleted == 0 && any {
        sqlx::query!(
            "INSERT INTO article_states (account_id, feed_id, seq, read, kept_unread, hidden, starred)
             VALUES ($1, $2, $3, coalesce($4, false), coalesce($5, false), coalesce($6, false),
                     coalesce($7, false))
             ON CONFLICT (account_id, feed_id, seq) DO UPDATE
             SET read = coalesce($4, article_states.read),
                 kept_unread = coalesce($5, article_states.kept_unread),
                 hidden = coalesce($6, article_states.hidden),
                 starred = coalesce($7, article_states.starred)",
            account_id,
            feed_id,
            seq,
            read,
            kept_unread,
            hidden,
            starred,
        )
        .execute(&mut *conn)
        .await?;
    } else if deleted == 0 {
        sqlx::query!(
            "UPDATE article_states
             SET read = coalesce($4, read), kept_unread = coalesce($5, kept_unread),
                 hidden = coalesce($6, hidden), starred = coalesce($7, starred)
             WHERE account_id = $1 AND feed_id = $2 AND seq = $3",
            account_id,
            feed_id,
            seq,
            read,
            kept_unread,
            hidden,
            starred,
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum Change {
    Read(bool),
    KeptUnread(bool),
    Hidden(bool),
    Starred(bool),
}

/// What apps change about many articles of a feed at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Many {
    Read(bool),
    Starred(bool),
}

/// Changes many articles of one feed at once, as apps send them, in a few
/// statements whatever their number; the same as one [`set_read_on`] or
/// [`set_starred`] after the other. Numbers the feed does not have are left
/// out. Returns false if the account does not subscribe to the feed.
pub async fn set_many_on(
    conn: &mut PgConnection,
    account_id: i32,
    feed_id: i32,
    seqs: &[i32],
    change: Many,
) -> anyhow::Result<bool> {
    let Some(locked) = lock(conn, account_id, feed_id).await? else {
        return Ok(false);
    };
    let mark = locked.mark;
    let mut seqs: Vec<i32> = seqs
        .iter()
        .copied()
        .filter(|seq| (1..=locked.last).contains(seq))
        .collect();
    seqs.sort_unstable();
    seqs.dedup();
    // At or below the cut only the articles the retention kept are there; a
    // mark on one that went would count as unread for nothing.
    let below: Vec<i32> = seqs
        .iter()
        .copied()
        .filter(|seq| *seq <= locked.cut)
        .collect();
    if !below.is_empty() {
        let kept: std::collections::HashSet<i32> = sqlx::query_scalar!(
            "SELECT seq FROM articles WHERE feed_id = $1 AND seq = ANY($2)",
            feed_id,
            &below,
        )
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .collect();
        seqs.retain(|seq| *seq > locked.cut || kept.contains(seq));
    }
    match change {
        Many::Starred(on) => {
            set_flags_many(conn, account_id, feed_id, &seqs, Change::Starred(on)).await?;
        }
        Many::Read(read) => {
            let (below, above): (Vec<i32>, Vec<i32>) = seqs.iter().partition(|seq| **seq <= mark);
            set_flags_many(conn, account_id, feed_id, &below, Change::KeptUnread(!read)).await?;
            set_flags_many(conn, account_id, feed_id, &above, Change::Read(read)).await?;
            if read && !above.is_empty() {
                compact(conn, account_id, feed_id, mark).await?;
            }
        }
    }
    Ok(true)
}

/// [`set_flags`] for many articles of a feed at once.
async fn set_flags_many(
    conn: &mut PgConnection,
    account_id: i32,
    feed_id: i32,
    seqs: &[i32],
    change: Change,
) -> anyhow::Result<()> {
    if seqs.is_empty() {
        return Ok(());
    }
    let (read, kept_unread, hidden, starred) = match change {
        Change::Read(on) => (Some(on), None, None, None),
        Change::KeptUnread(on) => (None, Some(on), None, None),
        Change::Hidden(on) => (None, None, Some(on), None),
        Change::Starred(on) => (None, None, None, Some(on)),
    };
    if [read, kept_unread, hidden, starred].contains(&Some(true)) {
        // A flag is set: no row ends up empty.
        sqlx::query!(
            "INSERT INTO article_states (account_id, feed_id, seq, read, kept_unread, hidden, starred)
             SELECT $1, $2, n.seq, coalesce($4, false), coalesce($5, false), coalesce($6, false),
                    coalesce($7, false)
             FROM unnest($3::int4[]) AS n (seq)
             ON CONFLICT (account_id, feed_id, seq) DO UPDATE
             SET read = coalesce($4, article_states.read),
                 kept_unread = coalesce($5, article_states.kept_unread),
                 hidden = coalesce($6, article_states.hidden),
                 starred = coalesce($7, article_states.starred)",
            account_id,
            feed_id,
            seqs,
            read,
            kept_unread,
            hidden,
            starred,
        )
        .execute(&mut *conn)
        .await?;
        return Ok(());
    }
    // A flag is cleared: rows left with none go, the others lose it.
    sqlx::query!(
        "DELETE FROM article_states
         WHERE account_id = $1 AND feed_id = $2 AND seq = ANY($3)
           AND NOT (coalesce($4, read) OR coalesce($5, kept_unread)
                    OR coalesce($6, hidden) OR coalesce($7, starred))",
        account_id,
        feed_id,
        seqs,
        read,
        kept_unread,
        hidden,
        starred,
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "UPDATE article_states
         SET read = coalesce($4, read), kept_unread = coalesce($5, kept_unread),
             hidden = coalesce($6, hidden), starred = coalesce($7, starred)
         WHERE account_id = $1 AND feed_id = $2 AND seq = ANY($3)",
        account_id,
        feed_id,
        seqs,
        read,
        kept_unread,
        hidden,
        starred,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Moves the mark up over the articles right above it that are read or hidden,
/// so the exceptions stay few however long somebody reads article by article.
/// A hidden article the mark passes without it being read is kept unread, so it
/// is unread again should its rule go.
pub(crate) async fn compact(
    conn: &mut PgConnection,
    account_id: i32,
    feed_id: i32,
    mark: i32,
) -> anyhow::Result<()> {
    let mut upto = mark;
    loop {
        // The length of the run without gaps that starts right above the mark.
        let reached = sqlx::query_scalar!(
            r#"
            SELECT max(run.seq) AS "seq"
            FROM (
                SELECT x.seq, row_number() OVER (ORDER BY x.seq) AS n
                FROM (
                    SELECT seq FROM article_states
                    WHERE account_id = $1 AND feed_id = $2 AND seq > $3 AND (read OR hidden)
                    ORDER BY seq LIMIT $4
                ) x
            ) run
            WHERE run.seq = $3 + run.n
            "#,
            account_id,
            feed_id,
            upto,
            COMPACT_CHUNK,
        )
        .fetch_one(&mut *conn)
        .await?;
        let Some(reached) = reached else { break };
        let whole_chunk = i64::from(reached - upto) == COMPACT_CHUNK;
        upto = reached;
        if !whole_chunk {
            break;
        }
    }
    if upto == mark {
        return Ok(());
    }
    // Under the mark "read" goes without saying: rows that said nothing else go,
    // the others lose the flag. (The table wants at least one flag per row.)
    sqlx::query!(
        "DELETE FROM article_states
         WHERE account_id = $1 AND feed_id = $2 AND seq > $3 AND seq <= $4
           AND read AND NOT (hidden OR starred)",
        account_id,
        feed_id,
        mark,
        upto,
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "UPDATE article_states
         SET read = false, kept_unread = hidden AND NOT read
         WHERE account_id = $1 AND feed_id = $2 AND seq > $3 AND seq <= $4
           AND (read OR hidden)",
        account_id,
        feed_id,
        mark,
        upto,
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "UPDATE subscriptions SET read_upto = $3 WHERE account_id = $1 AND feed_id = $2",
        account_id,
        feed_id,
        upto,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Marks an article as read or as unread. Returns false if the account does not
/// subscribe to the feed or the feed has no article of that number.
pub async fn set_read(
    db: &PgPool,
    account_id: i32,
    feed_id: i32,
    seq: i32,
    read: bool,
) -> anyhow::Result<bool> {
    let mut tx = db.begin().await?;
    let done = set_read_on(&mut tx, account_id, feed_id, seq, read).await?;
    tx.commit().await?;
    Ok(done)
}

/// [`set_read`] inside a transaction the caller has open.
pub async fn set_read_on(
    conn: &mut PgConnection,
    account_id: i32,
    feed_id: i32,
    seq: i32,
    read: bool,
) -> anyhow::Result<bool> {
    let Some(locked) = lock(conn, account_id, feed_id).await? else {
        return Ok(false);
    };
    if !locked.has(conn, feed_id, seq).await? {
        return Ok(false);
    }
    let mark = locked.mark;
    if seq <= mark {
        set_flags(conn, account_id, feed_id, seq, Change::KeptUnread(!read)).await?;
    } else {
        set_flags(conn, account_id, feed_id, seq, Change::Read(read)).await?;
        if read {
            compact(conn, account_id, feed_id, mark).await?;
        }
    }
    Ok(true)
}

/// Marks everything up to the number `upto` as read, usually the newest article
/// the list was showing. Returns false if the account does not subscribe to the
/// feed.
pub async fn mark_all_read(
    db: &PgPool,
    account_id: i32,
    feed_id: i32,
    upto: i32,
) -> anyhow::Result<bool> {
    let mut tx = db.begin().await?;
    let done = mark_all_read_on(&mut tx, account_id, feed_id, upto).await?;
    tx.commit().await?;
    Ok(done)
}

/// [`mark_all_read`] inside a transaction the caller has open.
pub async fn mark_all_read_on(
    tx: &mut PgConnection,
    account_id: i32,
    feed_id: i32,
    upto: i32,
) -> anyhow::Result<bool> {
    let Some(Locked { mark, last, .. }) = lock(&mut *tx, account_id, feed_id).await? else {
        return Ok(false);
    };
    let upto = upto.min(last);
    // The mark never moves back.
    let new_mark = mark.max(upto);
    // Up to the new mark "read" goes without saying, and what was kept unread up
    // to `upto` is read now: rows that said nothing else go, the others lose
    // these flags.
    sqlx::query!(
        "DELETE FROM article_states
         WHERE account_id = $1 AND feed_id = $2 AND seq <= $3
           AND (read OR (kept_unread AND seq <= $4)) AND NOT (hidden OR starred)",
        account_id,
        feed_id,
        new_mark,
        upto,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE article_states
         SET read = false, kept_unread = kept_unread AND seq > $4
         WHERE account_id = $1 AND feed_id = $2 AND seq <= $3 AND (read OR kept_unread)",
        account_id,
        feed_id,
        new_mark,
        upto,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE subscriptions SET read_upto = $3 WHERE account_id = $1 AND feed_id = $2",
        account_id,
        feed_id,
        new_mark,
    )
    .execute(&mut *tx)
    .await?;
    compact(&mut *tx, account_id, feed_id, new_mark).await?;
    Ok(true)
}

/// Marks as read what is older than `cutoff` among the articles up to the
/// number `upto`, by when they were published. Returns false if the account
/// does not subscribe to the feed.
///
/// Articles mostly arrive in the order they were published, so the mark moves
/// up as far as everything under it is old enough. An old article that
/// arrived among younger ones becomes an exception above the mark.
pub async fn mark_older_read(
    db: &PgPool,
    account_id: i32,
    feed_id: i32,
    upto: i32,
    cutoff: OffsetDateTime,
) -> anyhow::Result<bool> {
    let mut tx = db.begin().await?;
    let done = mark_older_read_on(&mut tx, account_id, feed_id, upto, cutoff).await?;
    tx.commit().await?;
    Ok(done)
}

/// [`mark_older_read`] inside a transaction the caller has open.
pub async fn mark_older_read_on(
    tx: &mut PgConnection,
    account_id: i32,
    feed_id: i32,
    upto: i32,
    cutoff: OffsetDateTime,
) -> anyhow::Result<bool> {
    let Some(Locked { mark, last, .. }) = lock(&mut *tx, account_id, feed_id).await? else {
        return Ok(false);
    };
    let upto = upto.min(last);
    // The first article above the mark that is too young stops the mark.
    let young = sqlx::query_scalar!(
        "SELECT min(seq) FROM articles
         WHERE feed_id = $1 AND seq > $2 AND seq <= $3 AND published_at > $4",
        feed_id,
        mark,
        upto,
        cutoff,
    )
    .fetch_one(&mut *tx)
    .await?;
    let new_mark = young.map_or(upto, |young| young - 1).max(mark);
    // Up to the new mark "read" goes without saying.
    sqlx::query!(
        "DELETE FROM article_states
         WHERE account_id = $1 AND feed_id = $2 AND seq <= $3 AND read AND NOT (hidden OR starred)",
        account_id,
        feed_id,
        new_mark,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE article_states SET read = false
         WHERE account_id = $1 AND feed_id = $2 AND seq <= $3 AND read",
        account_id,
        feed_id,
        new_mark,
    )
    .execute(&mut *tx)
    .await?;
    // What was kept unread and is old enough is read now.
    sqlx::query!(
        "DELETE FROM article_states x USING articles a
         WHERE x.account_id = $1 AND x.feed_id = $2 AND x.seq <= $3 AND x.kept_unread
           AND NOT (x.hidden OR x.starred)
           AND a.feed_id = x.feed_id AND a.seq = x.seq AND a.published_at <= $4",
        account_id,
        feed_id,
        upto,
        cutoff,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE article_states x SET kept_unread = false FROM articles a
         WHERE x.account_id = $1 AND x.feed_id = $2 AND x.seq <= $3 AND x.kept_unread
           AND a.feed_id = x.feed_id AND a.seq = x.seq AND a.published_at <= $4",
        account_id,
        feed_id,
        upto,
        cutoff,
    )
    .execute(&mut *tx)
    .await?;
    // The old ones above the mark, behind something younger.
    sqlx::query!(
        "INSERT INTO article_states (account_id, feed_id, seq, read)
         SELECT $1, a.feed_id, a.seq, true FROM articles a
         WHERE a.feed_id = $2 AND a.seq > $3 AND a.seq <= $4 AND a.published_at <= $5
         ON CONFLICT (account_id, feed_id, seq)
         DO UPDATE SET read = true, kept_unread = false",
        account_id,
        feed_id,
        new_mark,
        upto,
        cutoff,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE subscriptions SET read_upto = $3 WHERE account_id = $1 AND feed_id = $2",
        account_id,
        feed_id,
        new_mark,
    )
    .execute(&mut *tx)
    .await?;
    compact(&mut *tx, account_id, feed_id, new_mark).await?;
    Ok(true)
}

/// After the first fetch of a feed that came with a move from another rueder
/// (see `transfer`): what was published before the time the subscription
/// keeps in `read_before` counts as read, as it did there. The time is used
/// once and then forgotten. Runs in the transaction that stored the articles.
pub async fn settle_read_before(conn: &mut PgConnection, feed_id: i32) -> anyhow::Result<()> {
    let waiting = sqlx::query!(
        r#"SELECT s.account_id, s.read_before AS "read_before!", f.last_seq
           FROM subscriptions s JOIN feeds f ON f.id = s.feed_id
           WHERE s.feed_id = $1 AND s.read_before IS NOT NULL"#,
        feed_id
    )
    .fetch_all(&mut *conn)
    .await?;
    for sub in waiting {
        mark_older_read_on(
            &mut *conn,
            sub.account_id,
            feed_id,
            sub.last_seq,
            sub.read_before,
        )
        .await?;
        sqlx::query!(
            "UPDATE subscriptions SET read_before = NULL WHERE account_id = $1 AND feed_id = $2",
            sub.account_id,
            feed_id,
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// Sets or takes away an article's star. Returns false if there is no such
/// article for the account.
pub async fn set_starred(
    db: &PgPool,
    account_id: i32,
    feed_id: i32,
    seq: i32,
    starred: bool,
) -> anyhow::Result<bool> {
    let mut tx = db.begin().await?;
    let Some(locked) = lock(&mut tx, account_id, feed_id).await? else {
        return Ok(false);
    };
    if !locked.has(&mut tx, feed_id, seq).await? {
        return Ok(false);
    }
    set_flags(&mut tx, account_id, feed_id, seq, Change::Starred(starred)).await?;
    tx.commit().await?;
    Ok(true)
}

/// Hides an article for the account, or shows it again. This is what a filter
/// rule does to an article it matches. Returns false if there is no such article
/// for the account.
pub async fn set_hidden(
    db: &PgPool,
    account_id: i32,
    feed_id: i32,
    seq: i32,
    hidden: bool,
) -> anyhow::Result<bool> {
    let mut tx = db.begin().await?;
    let done = set_hidden_on(&mut tx, account_id, feed_id, seq, hidden).await?;
    tx.commit().await?;
    Ok(done)
}

/// [`set_hidden`] inside a transaction the caller has open.
pub async fn set_hidden_on(
    conn: &mut PgConnection,
    account_id: i32,
    feed_id: i32,
    seq: i32,
    hidden: bool,
) -> anyhow::Result<bool> {
    let Some(locked) = lock(conn, account_id, feed_id).await? else {
        return Ok(false);
    };
    if !locked.has(conn, feed_id, seq).await? {
        return Ok(false);
    }
    let mark = locked.mark;
    set_flags(conn, account_id, feed_id, seq, Change::Hidden(hidden)).await?;
    if hidden {
        compact(conn, account_id, feed_id, mark).await?;
    }
    Ok(true)
}
