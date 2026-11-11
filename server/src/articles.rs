//! The article store: three tables, so that lists never read text and old
//! versions never touch what new articles are read from.
//!
//! - `articles` (hot): one narrow row per article, all a list shows.
//! - `article_content` (warm): the cleaned text of the current version.
//! - `article_versions` (cold): every version as it was fetched, compressed,
//!   partitioned by month, append-only.
//!
//! An article is addressed by its feed and its number in the feed (`seq`), which
//! is given out once, without gaps, as articles arrive. Lists across feeds follow
//! `id`, the order of arrival over all feeds; within a feed it rises with `seq`.

use anyhow::Context;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;

/// An article keeps at most this many versions: the first and the newest ones.
pub const MAX_VERSIONS: i64 = 20;

/// Versions are a few kilobytes each, where higher levels gain little and cost time.
const ZSTD_LEVEL: i32 = 3;

/// An article as it was fetched, before anything is cleaned. This is what a
/// version keeps, so cleaning can be done again later with better rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Original {
    pub title: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// The article's page, without tracking parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Tells two versions of an article apart: by its title and by the text as it
/// is read, not as it was sent. What the cleaning takes out does not count, so
/// a feed that puts a new tracking token into its links on every fetch does
/// not make a new version every time, and neither does one that only reflows
/// its markup: runs of white space count as one blank.
pub fn checksum(title: &str, html: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    for part in [title, html] {
        for word in part.split_whitespace() {
            hasher.update(word.as_bytes());
            hasher.update(b" ");
        }
        hasher.update([0]);
    }
    hasher.finalize().to_vec()
}

impl Original {
    fn compress(&self) -> anyhow::Result<Vec<u8>> {
        let json = serde_json::to_vec(self)?;
        Ok(zstd::encode_all(json.as_slice(), ZSTD_LEVEL)?)
    }

    pub(crate) fn decompress(raw: &[u8]) -> anyhow::Result<Self> {
        let json = zstd::decode_all(raw).context("decompressing a version")?;
        serde_json::from_slice(&json).context("reading a version")
    }
}

/// What the fetcher recognizes an article by: the hash of its id in the feed.
pub fn guid_hash(guid: &str) -> Vec<u8> {
    Sha256::digest(guid.as_bytes())[..16].to_vec()
}

/// An article on its way into the store.
#[derive(Debug, Clone)]
pub struct NewArticle {
    /// The article's id in its feed (GUID, or its link where there is none).
    pub guid: String,
    pub published_at: OffsetDateTime,
    pub fetched_at: OffsetDateTime,
    /// As fetched; kept as the version.
    pub original: Original,
    /// For lists.
    pub teaser: String,
    pub thumbnail_url: Option<String>,
    /// The cleaned text, for reading.
    pub html: String,
    pub enclosures: serde_json::Value,
    pub image_url: Option<String>,
    pub image_title: Option<String>,
}

impl NewArticle {
    fn author(&self) -> String {
        self.original.authors.join(", ")
    }

    /// See [`checksum`].
    pub fn checksum(&self) -> Vec<u8> {
        checksum(&self.original.title, &self.html)
    }
}

/// Adds articles to a feed, oldest first, and returns the number of the first.
/// Call it inside a transaction: the feed's row stays locked until the end, so
/// only one writer hands out numbers and they come without gaps.
///
/// Fails if the feed already has an article with one of the ids; the caller
/// sorts out beforehand what is new.
pub async fn insert(
    conn: &mut PgConnection,
    feed_id: i32,
    articles: &[NewArticle],
) -> anyhow::Result<i32> {
    let count = i32::try_from(articles.len())?;
    let last = sqlx::query_scalar!(
        "UPDATE feeds SET last_seq = last_seq + $2 WHERE id = $1 RETURNING last_seq",
        feed_id,
        count
    )
    .fetch_optional(&mut *conn)
    .await?
    .with_context(|| format!("no feed {feed_id}"))?;
    let first = last - count + 1;
    let placed: Vec<_> = articles
        .iter()
        .zip(first..)
        .map(|(article, seq)| Placed {
            feed_id,
            seq,
            id: None,
            article,
        })
        .collect();
    write(conn, &placed).await?;
    Ok(first)
}

/// An article and where it goes.
pub(crate) struct Placed<'a> {
    pub feed_id: i32,
    pub seq: i32,
    /// Its place in the order of arrival; `None` puts it at the end. Only the
    /// import of an old database says where articles stood.
    pub id: Option<i64>,
    pub article: &'a NewArticle,
}

/// Writes articles as their first version into the three tables, in the order
/// given. Keeping the numbers in step with `feeds.last_seq` is the caller's part.
pub(crate) async fn write(conn: &mut PgConnection, placed: &[Placed<'_>]) -> anyhow::Result<()> {
    if placed.is_empty() {
        return Ok(());
    }
    let articles = || placed.iter().map(|p| p.article);
    let feed_ids: Vec<_> = placed.iter().map(|p| p.feed_id).collect();
    let seqs: Vec<_> = placed.iter().map(|p| p.seq).collect();
    let ids: Vec<_> = placed.iter().map(|p| p.id).collect();

    let published: Vec<_> = articles().map(|a| a.published_at).collect();
    let titles: Vec<_> = articles().map(|a| a.original.title.clone()).collect();
    let teasers: Vec<_> = articles().map(|a| a.teaser.clone()).collect();
    let authors: Vec<_> = articles().map(NewArticle::author).collect();
    let urls: Vec<_> = articles().map(|a| a.original.url.clone()).collect();
    let thumbnails: Vec<_> = articles().map(|a| a.thumbnail_url.clone()).collect();
    sqlx::query!(
        "INSERT INTO articles
             (feed_id, seq, id, published_at, title, teaser, author, url, thumbnail_url)
         SELECT n.feed_id, n.seq, coalesce(n.id, nextval('articles_id_seq')), n.published_at,
                n.title, n.teaser, n.author, n.url, n.thumbnail_url
         FROM unnest($1::integer[], $2::integer[], $3::bigint[], $4::timestamptz[], $5::text[],
                     $6::text[], $7::text[], $8::text[], $9::text[])
              WITH ORDINALITY
              AS n (feed_id, seq, id, published_at, title, teaser, author, url, thumbnail_url, ord)
         ORDER BY n.ord",
        &feed_ids,
        &seqs,
        &ids as &[Option<i64>],
        &published,
        &titles,
        &teasers,
        &authors,
        &urls as &[Option<String>],
        &thumbnails as &[Option<String>],
    )
    .execute(&mut *conn)
    .await?;

    let guids: Vec<_> = articles().map(|a| a.guid.clone()).collect();
    let guid_hashes: Vec<_> = articles().map(|a| guid_hash(&a.guid)).collect();
    let checksums: Vec<_> = articles().map(NewArticle::checksum).collect();
    let html: Vec<_> = articles().map(|a| a.html.clone()).collect();
    let tags = articles()
        .map(|a| serde_json::to_string(&a.original.tags))
        .collect::<Result<Vec<_>, _>>()?;
    let enclosures: Vec<_> = articles().map(|a| a.enclosures.to_string()).collect();
    let image_urls: Vec<_> = articles().map(|a| a.image_url.clone()).collect();
    let image_titles: Vec<_> = articles().map(|a| a.image_title.clone()).collect();
    sqlx::query!(
        "INSERT INTO article_content
             (feed_id, seq, guid_hash, checksum, guid, html, tags, enclosures, image_url, image_title)
         SELECT n.feed_id, n.seq, n.guid_hash, n.checksum, n.guid, n.html,
                ARRAY(SELECT jsonb_array_elements_text(n.tags::jsonb)), n.enclosures::jsonb,
                n.image_url, n.image_title
         FROM unnest($1::integer[], $2::integer[], $3::bytea[], $4::bytea[], $5::text[],
                     $6::text[], $7::text[], $8::text[], $9::text[], $10::text[])
              AS n (feed_id, seq, guid_hash, checksum, guid, html, tags, enclosures, image_url,
                    image_title)",
        &feed_ids,
        &seqs,
        &guid_hashes,
        &checksums,
        &guids,
        &html,
        &tags,
        &enclosures,
        &image_urls as &[Option<String>],
        &image_titles as &[Option<String>],
    )
    .execute(&mut *conn)
    .await?;

    let fetched: Vec<_> = articles().map(|a| a.fetched_at).collect();
    let raw = articles()
        .map(|a| a.original.compress())
        .collect::<Result<Vec<_>, _>>()?;
    ensure_partitions(conn, &fetched).await?;
    sqlx::query!(
        "INSERT INTO article_versions (feed_id, seq, version, fetched_at, checksum, raw)
         SELECT n.feed_id, n.seq, 1, n.fetched_at, n.checksum, n.raw
         FROM unnest($1::integer[], $2::integer[], $3::timestamptz[], $4::bytea[], $5::bytea[])
              AS n (feed_id, seq, fetched_at, checksum, raw)",
        &feed_ids,
        &seqs,
        &fetched,
        &checksums,
        &raw,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Makes sure the months these times fall into have their partition.
async fn ensure_partitions(
    conn: &mut PgConnection,
    times: &[OffsetDateTime],
) -> anyhow::Result<()> {
    let mut days: Vec<_> = times
        .iter()
        .map(|t| t.to_offset(time::UtcOffset::UTC).date().replace_day(1))
        .collect::<Result<_, _>>()?;
    days.sort_unstable();
    days.dedup();
    // Asking is cheap, creating takes a lock on the table: only create what is missing.
    sqlx::query!(
        "SELECT ensure_versions_partition(d.day)
         FROM unnest($1::date[]) AS d (day)
         WHERE to_regclass('article_versions_' || to_char(d.day, 'YYYY_MM')) IS NULL",
        &days
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok(())
}

/// The partitions for this month and the next. The server calls it when it starts
/// and from time to time after that.
pub async fn ensure_current_partitions(db: &PgPool) -> anyhow::Result<()> {
    let now = OffsetDateTime::now_utc();
    let next = now.date().replace_day(1)? + time::Duration::days(32);
    let mut conn = db.acquire().await?;
    ensure_partitions(&mut conn, &[now, next.midnight().assume_utc()]).await
}

/// Replaces an article's current version with a changed one and returns the new
/// version's number. What somebody has read stays as it is; the article only
/// gets its time of change, which lists show as "updated".
///
/// Returns `None` if the feed has no such article or nothing changed. Call it
/// inside a transaction.
pub async fn add_version(
    conn: &mut PgConnection,
    feed_id: i32,
    seq: i32,
    article: &NewArticle,
) -> anyhow::Result<Option<i16>> {
    let checksum = article.checksum();
    let changed = sqlx::query_scalar!(
        "UPDATE article_content
         SET checksum = $3, html = $4,
             tags = ARRAY(SELECT jsonb_array_elements_text($5::text::jsonb)),
             enclosures = $6::text::jsonb, image_url = $7, image_title = $8
         WHERE feed_id = $1 AND seq = $2 AND checksum <> $3
         RETURNING seq",
        feed_id,
        seq,
        &checksum,
        article.html,
        serde_json::to_string(&article.original.tags)?,
        article.enclosures.to_string(),
        article.image_url,
        article.image_title,
    )
    .fetch_optional(&mut *conn)
    .await?;
    if changed.is_none() {
        return Ok(None);
    }

    let version = sqlx::query_scalar!(
        "UPDATE articles
         SET version = version + 1, updated_at = $3, title = $4, teaser = $5, author = $6,
             url = $7, thumbnail_url = $8
         WHERE feed_id = $1 AND seq = $2
         RETURNING version",
        feed_id,
        seq,
        article.fetched_at,
        article.original.title,
        article.teaser,
        article.author(),
        article.original.url,
        article.thumbnail_url,
    )
    .fetch_one(&mut *conn)
    .await?;

    ensure_partitions(conn, &[article.fetched_at]).await?;
    sqlx::query!(
        "INSERT INTO article_versions (feed_id, seq, version, fetched_at, checksum, raw)
         VALUES ($1, $2, $3, $4, $5, $6)",
        feed_id,
        seq,
        version,
        article.fetched_at,
        &checksum,
        article.original.compress()?,
    )
    .execute(&mut *conn)
    .await?;

    // Over the limit, the oldest version after the first one goes.
    sqlx::query!(
        "DELETE FROM article_versions v
         WHERE v.feed_id = $1 AND v.seq = $2
           AND (SELECT count(*) FROM article_versions c WHERE c.feed_id = $1 AND c.seq = $2) > $3
           AND v.version = (
               SELECT o.version FROM article_versions o
               WHERE o.feed_id = $1 AND o.seq = $2
               ORDER BY o.version OFFSET 1 LIMIT 1)",
        feed_id,
        seq,
        MAX_VERSIONS,
    )
    .execute(&mut *conn)
    .await?;
    Ok(Some(version))
}

/// The article as its current version was fetched, if the feed has it.
pub(crate) async fn current_original(
    conn: &mut PgConnection,
    feed_id: i32,
    seq: i32,
) -> anyhow::Result<Option<Original>> {
    let raw = sqlx::query_scalar!(
        "SELECT v.raw FROM articles a
         JOIN article_versions v ON v.feed_id = a.feed_id AND v.seq = a.seq AND v.version = a.version
         WHERE a.feed_id = $1 AND a.seq = $2",
        feed_id,
        seq,
    )
    .fetch_optional(&mut *conn)
    .await?;
    raw.map(|raw| Original::decompress(&raw)).transpose()
}

/// Puts the text of an article's current version in place again, cleaned
/// anew. Nothing about the article changed, only the rules it is cleaned by,
/// so it gets no new version.
pub(crate) async fn recleaned(
    conn: &mut PgConnection,
    feed_id: i32,
    seq: i32,
    article: &NewArticle,
) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE article_content SET checksum = $3, html = $4, enclosures = $5::text::jsonb
         WHERE feed_id = $1 AND seq = $2",
        feed_id,
        seq,
        article.checksum(),
        article.html,
        article.enclosures.to_string(),
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// One line of a list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListItem {
    pub feed_id: i32,
    pub seq: i32,
    /// Where the article stands in the order of arrival; the anchor for the next page.
    pub id: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub published_at: OffsetDateTime,
    /// When the current version replaced the one before, if it ever did.
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<OffsetDateTime>,
    pub version: i16,
    pub title: String,
    pub teaser: String,
    pub author: String,
    pub url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub read: bool,
    pub starred: bool,
}

/// Which part of a list to read.
#[derive(Debug, Clone, Copy)]
pub struct Page {
    /// Only articles that arrived before this one (`ListItem::id`); `None` starts
    /// at the newest.
    pub before: Option<i64>,
    pub limit: i64,
    /// Leave out what the account has read.
    pub unread_only: bool,
}

/// A page of the articles of some feeds, newest first, as the account sees them:
/// only feeds it subscribes to, without what its filter rules hide, each article
/// with whether it is read and starred. One feed, a folder's feeds and all feeds
/// are the same query.
///
/// The cost does not depend on how many articles the feeds hold, only on the
/// number of feeds and the length of the page. The feeds are merged the way
/// sorted lists are: an index gives each feed's newest article that belongs on
/// the list, the newest of these goes onto the page, and its feed gives its next
/// one. That is one step into the index per feed and one per line of the page.
/// With `unread_only`, the read mark of each subscription is a floor under which
/// the index is never looked at.
pub async fn page(
    db: &PgPool,
    account_id: i32,
    feed_ids: &[i32],
    page: Page,
) -> anyhow::Result<Vec<ListItem>> {
    let mut tx = db.begin().await?;
    // The plan is the same whatever the feeds and the place in the list: walk
    // each index backwards and stop early. So it is made once per connection
    // and kept, which saves more time than the query takes to run. And the
    // planner, not knowing how few rows a step reads, must not think of a
    // bitmap scan, which would read all of a feed to sort it.
    sqlx::query!(
        "SELECT set_config('plan_cache_mode', 'force_generic_plan', true) AS plan,
                set_config('enable_bitmapscan', 'off', true) AS scan"
    )
    .fetch_one(&mut *tx)
    .await?;
    let items = sqlx::query_as!(
        ListItem,
        r#"
        WITH RECURSIVE subs AS MATERIALIZED (
            SELECT s.feed_id, s.read_upto,
                   CASE WHEN $5 THEN coalesce(
                       (SELECT m.id FROM articles m
                        WHERE m.feed_id = s.feed_id AND m.seq = s.read_upto), 0)
                   ELSE 0 END AS floor
            FROM subscriptions s
            WHERE s.account_id = $1 AND s.feed_id = ANY($2)
        ), merge AS (
            -- Every feed's newest article that belongs on the list.
            SELECT array_agg(subs.feed_id) AS feeds, array_agg(subs.floor) AS floors,
                   array_agg(head.id) AS heads, 0::bigint AS n, NULL::integer AS feed_id,
                   NULL::bigint AS id
            FROM subs CROSS JOIN LATERAL (
                SELECT (SELECT a.id FROM articles a
                        WHERE a.feed_id = subs.feed_id AND a.id > subs.floor AND a.id < $3
                          AND NOT EXISTS (
                              SELECT 1 FROM article_states x
                              WHERE x.account_id = $1 AND x.feed_id = a.feed_id AND x.seq = a.seq
                                AND (x.hidden OR ($5 AND x.read))
                              OFFSET 0)
                        ORDER BY a.id DESC LIMIT 1) AS id
            ) head
          UNION ALL
            -- The newest of them goes onto the page, and its feed's next one
            -- takes its place.
            SELECT m.feeds, m.floors,
                   m.heads[:newest.at - 1] || next.id || m.heads[newest.at + 1:],
                   m.n + 1, m.feeds[newest.at], m.heads[newest.at]
            FROM merge m
            CROSS JOIN LATERAL (
                SELECT h.at FROM unnest(m.heads) WITH ORDINALITY AS h (id, at)
                WHERE h.id IS NOT NULL
                ORDER BY h.id DESC LIMIT 1
            ) newest
            CROSS JOIN LATERAL (
                SELECT (SELECT a.id FROM articles a
                        WHERE a.feed_id = m.feeds[newest.at] AND a.id > m.floors[newest.at]
                          AND a.id < m.heads[newest.at]
                          AND NOT EXISTS (
                              SELECT 1 FROM article_states x
                              WHERE x.account_id = $1 AND x.feed_id = a.feed_id AND x.seq = a.seq
                                AND (x.hidden OR ($5 AND x.read))
                              OFFSET 0)
                        ORDER BY a.id DESC LIMIT 1) AS id
            ) next
            WHERE m.n < $4
        ), picked AS (
            SELECT m.feed_id, m.id FROM merge m WHERE m.n > 0
          UNION ALL
            -- Set back to unread under the read mark: few, found from their state.
            SELECT a.feed_id, a.id
            FROM subs
            JOIN article_states k ON k.account_id = $1 AND k.feed_id = subs.feed_id
                AND k.kept_unread AND NOT k.hidden AND k.seq <= subs.read_upto
            JOIN articles a ON a.feed_id = k.feed_id AND a.seq = k.seq
            WHERE $5 AND a.id < $3
            ORDER BY id DESC
            LIMIT $4
        )
        SELECT a.feed_id, a.seq, a.id, a.published_at, a.updated_at, a.version, a.title,
               a.teaser, a.author, a.url, a.thumbnail_url,
               CASE WHEN a.seq <= subs.read_upto THEN NOT coalesce(st.kept_unread, false)
                    ELSE coalesce(st.read, false) END AS "read!",
               coalesce(st.starred, false) AS "starred!"
        FROM picked p
        JOIN subs ON subs.feed_id = p.feed_id
        JOIN articles a ON a.feed_id = p.feed_id AND a.id = p.id
        -- One step into the index per line; never the whole of the account's states.
        LEFT JOIN LATERAL (
            SELECT x.read, x.kept_unread, x.starred FROM article_states x
            WHERE x.account_id = $1 AND x.feed_id = a.feed_id AND x.seq = a.seq
            OFFSET 0
        ) st ON true
        ORDER BY a.id DESC
        "#,
        account_id,
        feed_ids,
        page.before.unwrap_or(i64::MAX),
        page.limit,
        page.unread_only,
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(items)
}

/// The lists that are not a feed's: what the account starred, what its rules
/// or the reader hid, and what carries one of its labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marked {
    Starred,
    Hidden,
    Label(i32),
}

/// A page of such a list, newest first. Only articles of feeds the account
/// subscribes to, and, except in the list of the hidden ones, nothing hidden.
/// These lists are as long as what a reader marked by hand, so they are read
/// from the marks and not from the feeds.
pub async fn marked(
    db: &PgPool,
    account_id: i32,
    which: Marked,
    page: Page,
) -> anyhow::Result<Vec<ListItem>> {
    let (kind, label) = match which {
        Marked::Starred => ("starred", None),
        Marked::Hidden => ("hidden", None),
        Marked::Label(id) => ("label", Some(id)),
    };
    let items = sqlx::query_as!(
        ListItem,
        r#"
        WITH marks AS (
            SELECT x.feed_id, x.seq FROM article_states x
            WHERE $2 = 'starred' AND x.account_id = $1 AND x.starred
          UNION ALL
            SELECT x.feed_id, x.seq FROM article_states x
            WHERE $2 = 'hidden' AND x.account_id = $1 AND x.hidden
          UNION ALL
            SELECT al.feed_id, al.seq
            FROM labels l JOIN article_labels al ON al.label_id = l.id
            WHERE $2 = 'label' AND l.account_id = $1 AND l.id = $3
        )
        SELECT a.feed_id AS "feed_id!", a.seq AS "seq!", a.id AS "id!",
               a.published_at AS "published_at!", a.updated_at, a.version AS "version!",
               a.title AS "title!", a.teaser AS "teaser!", a.author AS "author!", a.url,
               a.thumbnail_url,
               CASE WHEN a.seq <= s.read_upto THEN NOT coalesce(st.kept_unread, false)
                    ELSE coalesce(st.read, false) END AS "read!",
               coalesce(st.starred, false) AS "starred!"
        FROM marks m
        JOIN subscriptions s ON s.account_id = $1 AND s.feed_id = m.feed_id
        JOIN articles a ON a.feed_id = m.feed_id AND a.seq = m.seq
        LEFT JOIN article_states st
            ON st.account_id = $1 AND st.feed_id = a.feed_id AND st.seq = a.seq
        WHERE a.id < $4 AND ($2 = 'hidden' OR NOT coalesce(st.hidden, false))
              AND ($5::boolean IS NOT TRUE
                   OR NOT CASE WHEN a.seq <= s.read_upto THEN NOT coalesce(st.kept_unread, false)
                               ELSE coalesce(st.read, false) END)
        ORDER BY a.id DESC
        LIMIT $6
        "#,
        account_id,
        kind,
        label,
        page.before.unwrap_or(i64::MAX),
        page.unread_only,
        page.limit,
    )
    .fetch_all(db)
    .await?;
    Ok(items)
}

/// An article opened for reading: the cleaned text of its current version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Opened {
    pub feed_id: i32,
    pub seq: i32,
    pub id: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub published_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub updated_at: Option<OffsetDateTime>,
    pub version: i16,
    pub title: String,
    pub author: String,
    pub url: Option<String>,
    pub html: String,
    pub tags: Vec<String>,
    pub enclosures: serde_json::Value,
    pub image_url: Option<String>,
    pub image_title: Option<String>,
}

/// An article with its text, if the account subscribes to its feed.
pub async fn open(
    db: &PgPool,
    account_id: i32,
    feed_id: i32,
    seq: i32,
) -> anyhow::Result<Option<Opened>> {
    let opened = sqlx::query_as!(
        Opened,
        "SELECT a.feed_id, a.seq, a.id, a.published_at, a.updated_at, a.version, a.title,
                a.author, a.url, c.html, c.tags, c.enclosures, c.image_url, c.image_title
         FROM subscriptions s
         JOIN articles a ON a.feed_id = s.feed_id
         JOIN article_content c ON c.feed_id = a.feed_id AND c.seq = a.seq
         WHERE s.account_id = $1 AND s.feed_id = $2 AND a.seq = $3",
        account_id,
        feed_id,
        seq,
    )
    .fetch_optional(db)
    .await?;
    Ok(opened)
}

/// One of an article's versions, as it was fetched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Version {
    pub version: i16,
    #[serde(with = "time::serde::rfc3339")]
    pub fetched_at: OffsetDateTime,
    pub original: Original,
}

/// The versions of an article that are kept, oldest first, if the account
/// subscribes to its feed. This is the only reader of the cold table.
pub async fn versions(
    db: &PgPool,
    account_id: i32,
    feed_id: i32,
    seq: i32,
) -> anyhow::Result<Vec<Version>> {
    let rows = sqlx::query!(
        "SELECT v.version, v.fetched_at, v.raw
         FROM subscriptions s
         JOIN article_versions v ON v.feed_id = s.feed_id
         WHERE s.account_id = $1 AND s.feed_id = $2 AND v.seq = $3
         ORDER BY v.version",
        account_id,
        feed_id,
        seq,
    )
    .fetch_all(db)
    .await?;
    rows.into_iter()
        .map(|row| {
            Ok(Version {
                version: row.version,
                fetched_at: row.fetched_at,
                original: Original::decompress(&row.raw)?,
            })
        })
        .collect()
}
