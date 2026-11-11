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

/// How far the mark looks ahead in one go when it moves up.
const COMPACT_CHUNK: i64 = 1000;

/// A subscription's numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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

/// Locks the subscription for the rest of the transaction and returns its mark
/// and the feed's newest number, or `None` if the account does not subscribe to
/// the feed.
async fn lock(
    conn: &mut PgConnection,
    account_id: i32,
    feed_id: i32,
) -> anyhow::Result<Option<(i32, i32)>> {
    let row = sqlx::query!(
        "SELECT s.read_upto, f.last_seq
         FROM subscriptions s JOIN feeds f ON f.id = s.feed_id
         WHERE s.account_id = $1 AND s.feed_id = $2
         FOR UPDATE OF s",
        account_id,
        feed_id
    )
    .fetch_optional(&mut *conn)
    .await?;
    Ok(row.map(|row| (row.read_upto, row.last_seq)))
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

/// Moves the mark up over the articles right above it that are read or hidden,
/// so the exceptions stay few however long somebody reads article by article.
/// A hidden article the mark passes without it being read is kept unread, so it
/// is unread again should its rule go.
async fn compact(
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
    let Some((mark, last)) = lock(conn, account_id, feed_id).await? else {
        return Ok(false);
    };
    if seq < 1 || seq > last {
        return Ok(false);
    }
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
    let Some((mark, last)) = lock(&mut tx, account_id, feed_id).await? else {
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
    compact(&mut tx, account_id, feed_id, new_mark).await?;
    tx.commit().await?;
    Ok(true)
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
    let Some((_, last)) = lock(&mut tx, account_id, feed_id).await? else {
        return Ok(false);
    };
    if seq < 1 || seq > last {
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
    let Some((mark, last)) = lock(conn, account_id, feed_id).await? else {
        return Ok(false);
    };
    if seq < 1 || seq > last {
        return Ok(false);
    }
    set_flags(conn, account_id, feed_id, seq, Change::Hidden(hidden)).await?;
    if hidden {
        compact(conn, account_id, feed_id, mark).await?;
    }
    Ok(true)
}
