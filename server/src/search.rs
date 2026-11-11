//! The search over the articles of the feeds an account subscribes to.
//!
//! An index over `article_content` holds the words of every article's current
//! version: title, author, tags and text (`migrations/0002_search.sql`). A
//! search asks for articles that have all the words, or words that begin with
//! them, newest first, and pages through them with the same anchor as the lists.
//!
//! Words are taken as PostgreSQL's parser splits them, without stems: "wahl"
//! finds "Wahl", "Wahlen" and "Wahlkampf". Words of one or two letters are
//! only found whole, as there would be too many beginning with them.
//!
//! The index knows which articles have a word, not in which order they came.
//! Sorting all of them would cost as much as there are, which for a common word
//! is a large part of all articles. So a search goes back in time in windows of
//! the order of arrival: first the newest 20,000 articles, then twice as many
//! before them, and so on, until the page is full. A common word fills the
//! page in the first window, a rare one is found in the large windows further
//! back, where it has few hits. Either way the work per window stays small.

use std::time::{Duration, Instant};

use sqlx::PgPool;

use crate::articles::ListItem;

/// The longest search, in characters, and the most words in it.
pub const MAX_CHARS: usize = 200;
pub const MAX_WORDS: usize = 10;
/// How many of the newest articles the first window holds.
const FIRST_WINDOW: i64 = 20_000;
/// After this long a search stops at the end of a window and says where to
/// go on, so that no request holds a connection for long.
pub const BUDGET: Duration = Duration::from_millis(500);
/// A window that takes longer than this is given up.
pub const TIMEOUT: Duration = Duration::from_secs(3);

/// Which part of the hits to read.
#[derive(Debug, Clone, Copy)]
pub struct Page {
    /// Only articles that arrived before this one (`ListItem::id`, or `next`
    /// of the page before); `None` starts at the newest.
    pub before: Option<i64>,
    pub limit: i64,
    /// Leave out what the account has read.
    pub unread_only: bool,
}

/// A page of hits, newest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hits {
    pub articles: Vec<ListItem>,
    /// Where the next page begins (its `before`); `None` at the oldest
    /// article. A page may have fewer hits than asked for, even none, and
    /// still go on: then the search ran out of time.
    pub next: Option<i64>,
}

/// What came of a search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Found(Hits),
    /// The words were too many or too long, or there was no word in them.
    Unusable(&'static str),
    /// A window took longer than [`TIMEOUT`].
    TooSlow,
}

/// The query PostgreSQL searches with: every word, and from three letters on
/// every word beginning with it. `None` if there is no word in the text.
pub async fn query(db: &PgPool, words: &str) -> anyhow::Result<Option<String>> {
    let query = sqlx::query_scalar!(
        r#"SELECT string_agg(
                      '''' || replace(replace(w, '\', '\\'), '''', '''''') || ''''
                      || CASE WHEN char_length(w) >= 3 THEN ':*' ELSE '' END,
                      ' & ' ORDER BY w) AS "query"
           FROM unnest(tsvector_to_array(to_tsvector('simple', $1))) w"#,
        words
    )
    .fetch_one(db)
    .await?;
    Ok(query)
}

/// A page of the articles of these feeds that have the words, as the account
/// sees them: only feeds it subscribes to, without what its rules hid, each
/// article with whether it is read and starred.
pub async fn search(
    db: &PgPool,
    account_id: i32,
    feed_ids: &[i32],
    words: &str,
    page: Page,
) -> anyhow::Result<Outcome> {
    if words.chars().count() > MAX_CHARS {
        return Ok(Outcome::Unusable("the search is too long"));
    }
    let Some(query) = query(db, words).await? else {
        return Ok(Outcome::Unusable("there is no word to search for"));
    };
    if query.matches(" & ").count() >= MAX_WORDS {
        return Ok(Outcome::Unusable("the search has too many words"));
    }
    let started = Instant::now();

    let mut tx = db.begin().await?;
    // A plan of its own for every window, made with the words in it, as the
    // planner needs to know how common they are: a common word is found
    // quicker by walking back through the window and looking at each article,
    // a rarer one through the index of words. Reading all of the table is
    // never the quicker way.
    sqlx::query!(
        "SELECT set_config('plan_cache_mode', 'force_custom_plan', true) AS plan,
                set_config('enable_seqscan', 'off', true) AS seq,
                set_config('statement_timeout', $1, true) AS timeout",
        TIMEOUT.as_millis().to_string()
    )
    .fetch_one(&mut *tx)
    .await?;
    let bounds = sqlx::query!("SELECT min(id) AS first, max(id) AS last FROM article_content")
        .fetch_one(&mut *tx)
        .await?;
    let (Some(first), Some(last)) = (bounds.first, bounds.last) else {
        return Ok(Outcome::Found(Hits {
            articles: Vec::new(),
            next: None,
        }));
    };

    let mut articles: Vec<ListItem> = Vec::new();
    let mut upto = page.before.unwrap_or(i64::MAX).min(last + 1);
    let mut width = FIRST_WINDOW;
    let next = loop {
        if upto <= first {
            break None;
        }
        let from = upto.saturating_sub(width).max(first);
        let wanted = page.limit - articles.len() as i64;
        let found = window(
            &mut tx, account_id, feed_ids, &query, from, upto, wanted, page,
        )
        .await;
        let found = match found {
            Ok(found) => found,
            Err(sqlx::Error::Database(err)) if err.code().as_deref() == Some("57014") => {
                // 57014: canceled, here by the statement timeout.
                return Ok(Outcome::TooSlow);
            }
            Err(err) => return Err(err.into()),
        };
        let full = found.len() as i64 == wanted;
        articles.extend(found);
        if full {
            break articles.last().map(|a| a.id);
        }
        upto = from;
        width = width.saturating_mul(2);
        if upto > first && started.elapsed() > BUDGET {
            break Some(upto);
        }
    };
    tx.commit().await?;
    Ok(Outcome::Found(Hits { articles, next }))
}

/// The hits among the articles from `from` up to before `upto` in the order
/// of arrival, at most `limit`, newest first.
#[allow(clippy::too_many_arguments)]
async fn window(
    conn: &mut sqlx::PgConnection,
    account_id: i32,
    feed_ids: &[i32],
    query: &str,
    from: i64,
    upto: i64,
    limit: i64,
    page: Page,
) -> Result<Vec<ListItem>, sqlx::Error> {
    sqlx::query_as!(
        ListItem,
        r#"
        SELECT a.feed_id, a.seq, a.id, a.published_at, a.updated_at, a.version, a.title,
               a.teaser, a.author, a.url, a.thumbnail_url,
               CASE WHEN a.seq <= hit.read_upto THEN NOT coalesce(st.kept_unread, false)
                    ELSE coalesce(st.read, false) END AS "read!",
               coalesce(st.starred, false) AS "starred!"
        FROM (
            SELECT x.feed_id, x.seq, x.id, s.read_upto
            FROM article_content x
            JOIN subscriptions s ON s.account_id = $1 AND s.feed_id = x.feed_id
            WHERE article_search_doc(x.title, x.author, x.tags, x.html) @@ $3::text::tsquery
              AND x.id >= $4 AND x.id < $5 AND x.feed_id = ANY($2)
              AND NOT EXISTS (
                  SELECT 1 FROM article_states h
                  WHERE h.account_id = $1 AND h.feed_id = x.feed_id AND h.seq = x.seq
                    AND (h.hidden OR ($6 AND h.read)))
              AND ($6 IS NOT TRUE OR x.seq > s.read_upto OR EXISTS (
                  SELECT 1 FROM article_states k
                  WHERE k.account_id = $1 AND k.feed_id = x.feed_id AND k.seq = x.seq
                    AND k.kept_unread))
            ORDER BY x.id DESC
            LIMIT $7
        ) hit
        JOIN articles a ON a.feed_id = hit.feed_id AND a.seq = hit.seq
        LEFT JOIN article_states st
            ON st.account_id = $1 AND st.feed_id = a.feed_id AND st.seq = a.seq
        ORDER BY a.id DESC
        "#,
        account_id,
        feed_ids,
        query,
        from,
        upto,
        page.unread_only,
        limit,
    )
    .fetch_all(&mut *conn)
    .await
}
