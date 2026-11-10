//! Made-up data in the size of a real installation, and timings of the queries
//! the reader lives on. This is how the layout of the article store is checked
//! before anything is built on it, and checked again when it changes.
//!
//! The data is made by PostgreSQL itself, in the shape that matters for the
//! queries: a few very large feeds and many small ones, arrivals mixed across
//! feeds over ten years, readers in every state from "all read" to "never
//! opened". Texts are random characters of realistic length.

use std::{
    fmt,
    time::{Duration, Instant},
};

use anyhow::{Context, bail};
use sqlx::PgPool;

use crate::{
    articles::{self, Page},
    reading,
};

/// How much to make up.
#[derive(Debug, Clone, Copy)]
pub struct Size {
    pub accounts: i32,
    pub feeds: i32,
    pub articles: i64,
}

/// Feeds are picked through this many slots, handed out by weight.
const SLOTS: usize = 100_000;
/// Articles made per statement.
const BATCH: i64 = 200_000;
/// The made-up articles span this long, up to now.
const SPAN_SECONDS: i64 = 10 * 365 * 24 * 60 * 60;

/// Fills an empty database.
pub async fn generate(db: &PgPool, size: Size) -> anyhow::Result<()> {
    if size.accounts < 2 || size.feeds < 1 || size.articles < 1 {
        bail!("at least 2 accounts, 1 feed and 1 article");
    }
    let existing: i64 = sqlx::query_scalar("SELECT count(*) FROM feeds")
        .fetch_one(db)
        .await?;
    if existing > 0 {
        bail!("the database already has feeds; made-up data goes into an empty one");
    }
    let started = Instant::now();

    sqlx::query(
        "INSERT INTO accounts (username) SELECT 'bench-' || n FROM generate_series(1, $1) n",
    )
    .bind(size.accounts)
    .execute(db)
    .await?;
    sqlx::query(
        "INSERT INTO feeds (feed_url, site_url, title)
         SELECT 'https://feed-' || n || '.example/feed.xml', 'https://feed-' || n || '.example/',
                'Feed ' || n
         FROM generate_series(1, $1) n",
    )
    .bind(size.feeds)
    .execute(db)
    .await?;

    // Feed n gets a share of 1/n of the articles (Zipf): the largest of 300 feeds
    // holds a sixth of everything, the smallest a few thousandths of a percent.
    let feed_ids: Vec<i32> = sqlx::query_scalar("SELECT id FROM feeds ORDER BY id")
        .fetch_all(db)
        .await?;
    let total: f64 = (1..=feed_ids.len()).map(|n| 1.0 / n as f64).sum();
    let mut slots = Vec::with_capacity(SLOTS);
    let mut share = 0.0;
    for (n, &feed_id) in feed_ids.iter().enumerate() {
        share += 1.0 / (n + 1) as f64 / total;
        let upto = if n + 1 == feed_ids.len() {
            SLOTS
        } else {
            (share * SLOTS as f64).round() as usize
        };
        slots.resize(upto.max(slots.len()), feed_id);
    }
    sqlx::query("DROP TABLE IF EXISTS bench_slots")
        .execute(db)
        .await?;
    sqlx::query(
        "CREATE UNLOGGED TABLE bench_slots (slot integer PRIMARY KEY, feed_id integer NOT NULL)",
    )
    .execute(db)
    .await?;
    sqlx::query(
        "INSERT INTO bench_slots SELECT s.ord - 1, s.feed_id
         FROM unnest($1::integer[]) WITH ORDINALITY AS s (feed_id, ord)",
    )
    .bind(&slots)
    .execute(db)
    .await?;

    let step = SPAN_SECONDS as f64 / size.articles as f64;
    sqlx::query(
        "SELECT ensure_versions_partition(d::date)
         FROM generate_series(date_trunc('month', now() - $1 * interval '1 second'),
                              now() + interval '3 months', interval '1 month') d",
    )
    .bind(SPAN_SECONDS as f64)
    .fetch_all(db)
    .await?;

    let mut from = 1;
    while from <= size.articles {
        let to = (from + BATCH - 1).min(size.articles);
        sqlx::query(ARTICLES)
            .bind(from)
            .bind(to)
            .bind(SLOTS as i32)
            .bind(size.articles)
            .bind(step)
            .execute(db)
            .await
            .context("making up articles")?;
        tracing::info!(
            "{to} of {} articles after {} s",
            size.articles,
            started.elapsed().as_secs()
        );
        from = to + 1;
    }
    sqlx::query("SELECT setval('articles_id_seq', (SELECT max(id) FROM articles))")
        .fetch_one(db)
        .await?;
    sqlx::query("DROP TABLE bench_slots").execute(db).await?;

    sqlx::query(FOLDERS).execute(db).await?;
    sqlx::query(SUBSCRIPTIONS).execute(db).await?;
    sqlx::query(STATES)
        .execute(db)
        .await
        .context("making up what was read")?;
    tracing::info!(
        "accounts, folders and what they read after {} s",
        started.elapsed().as_secs()
    );

    // As a database looks that has been running for a while: all rows known to be
    // visible, statistics up to date.
    for table in [
        "accounts",
        "feeds",
        "folders",
        "subscriptions",
        "articles",
        "article_content",
        "article_versions",
        "article_states",
    ] {
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!("VACUUM (ANALYZE) {table}")))
            .execute(db)
            .await?;
    }
    tracing::info!("done after {} s", started.elapsed().as_secs());
    Ok(())
}

/// One batch of articles into the three tables. $1 and $2 are the first and last
/// place in the order of arrival, $3 the number of slots, $4 the number of all
/// articles, $5 the seconds between two arrivals.
///
/// A twentieth of the articles has a second version, a hundredth a third.
const ARTICLES: &str = r"
WITH picked AS (
    SELECT g, s.feed_id, row_number() OVER (PARTITION BY s.feed_id ORDER BY g) AS n
    FROM generate_series($1::bigint, $2::bigint) g
    JOIN bench_slots s ON s.slot = (hashint8(g) & 2147483647) % $3
), placed AS MATERIALIZED (
    SELECT p.g, p.feed_id, (f.last_seq + p.n)::integer AS seq,
           now() - (($4 - p.g) * $5::float8) * interval '1 second' AS at,
           encode(sha512(('t' || p.g)::bytea), 'hex') AS noise,
           hashint8(p.g + 1) & 1023 AS r
    FROM picked p JOIN feeds f ON f.id = p.feed_id
), sized AS MATERIALIZED (
    SELECT p.*,
           'https://feed-' || p.feed_id || '.example/posts/' || left(p.noise, 40) AS url,
           -- Chunks of text: mostly short notes, some long reads.
           CASE WHEN p.r < 600 THEN 2 + p.r % 5
                WHEN p.r < 950 THEN 6 + p.r % 10
                WHEN p.r < 1015 THEN 16 + p.r % 32
                ELSE 60 + p.r % 100 END AS chunks
    FROM placed p
), counted AS (
    UPDATE feeds f SET last_seq = f.last_seq + c.count
    FROM (SELECT feed_id, count(*)::integer AS count FROM sized GROUP BY feed_id) c
    WHERE f.id = c.feed_id
), hot AS (
    INSERT INTO articles
        (feed_id, seq, id, published_at, updated_at, version, title, teaser, author, url,
         thumbnail_url)
    SELECT feed_id, seq, g, at,
           CASE WHEN g % 100 = 0 THEN at + interval '40 days'
                WHEN g % 20 = 0 THEN at + interval '2 days' END,
           1 + (g % 20 = 0)::integer + (g % 100 = 0)::integer,
           left(noise, 25 + (r & 63)),
           left(encode(sha512(('s' || g)::bytea), 'hex') || noise, 120 + (hashint8(g + 2) & 127)),
           'author-' || (hashint8(g + 3) & 31),
           url,
           CASE WHEN g % 2 = 0
                THEN 'https://feed-' || feed_id || '.example/media/' || left(noise, 48) || '.jpg' END
    FROM sized ORDER BY g
), warm AS (
    INSERT INTO article_content (feed_id, seq, guid_hash, checksum, guid, html, tags)
    SELECT feed_id, seq, substring(sha256(url::bytea) FOR 16), sha256(('c' || g)::bytea), url,
           (SELECT string_agg(encode(sha256((g || ':' || i)::bytea), 'hex'), '')
            FROM generate_series(1, chunks) i),
           ARRAY['tag-' || (r & 15), 'tag-' || (r & 7)]
    FROM sized ORDER BY g
)
INSERT INTO article_versions (feed_id, seq, version, fetched_at, checksum, raw)
SELECT s.feed_id, s.seq, v.version, s.at + v.later, sha256(('c' || s.g || v.version)::bytea),
       (SELECT string_agg(sha256((s.g || ':' || v.version || ':' || i)::bytea), ''::bytea)
        FROM generate_series(1, s.chunks) i)
FROM sized s
JOIN (VALUES (1, interval '10 minutes', 1), (2, interval '2 days', 20), (3, interval '40 days', 100))
     AS v (version, later, every) ON s.g % v.every = 0
ORDER BY s.g
";

/// Per account three folders, each with two folders inside, one of them with
/// another two levels: four levels in all.
const FOLDERS: &str = r"
WITH top AS (
    INSERT INTO folders (account_id, title, position)
    SELECT a.id, 'Folder ' || n, n FROM accounts a, generate_series(1, 3) n
    RETURNING id, account_id, title
), second AS (
    INSERT INTO folders (account_id, parent_id, title, position)
    SELECT t.account_id, t.id, t.title || '.' || n, n FROM top t, generate_series(1, 2) n
    RETURNING id, account_id, title
), third AS (
    INSERT INTO folders (account_id, parent_id, title, position)
    SELECT s.account_id, s.id, s.title || '.1', 1 FROM second s WHERE s.title LIKE '%.1'
    RETURNING id, account_id, title
)
INSERT INTO folders (account_id, parent_id, title, position)
SELECT t.account_id, t.id, t.title || '.1', 1 FROM third t
";

/// The first account subscribes to every feed, the others to six in ten. Feeds
/// are spread over the folders and the top level.
const SUBSCRIPTIONS: &str = r"
WITH numbered AS (
    SELECT f.account_id, f.id, row_number() OVER (PARTITION BY f.account_id ORDER BY f.id) AS n,
           count(*) OVER (PARTITION BY f.account_id) AS of
    FROM folders f
), first AS (
    SELECT min(id) AS id FROM accounts
)
INSERT INTO subscriptions (account_id, feed_id, folder_id, position)
SELECT a.id, f.id, n.id, f.id
FROM accounts a
CROSS JOIN feeds f
CROSS JOIN first
LEFT JOIN numbered n ON n.account_id = a.id
    AND n.n = 1 + (hashint4(a.id * 7919 + f.id) & 2147483647) % (n.of + 2)
WHERE a.id = first.id OR (hashint4(a.id * 104729 + f.id) & 2147483647) % 10 < 6
";

/// What the accounts have read. Six subscriptions in ten are nearly read up,
/// a quarter lags some hundred articles, a tenth was given up long ago, the rest
/// was never opened. Above the mark every fifth article is read out of order,
/// some are hidden, under it a few are kept unread; stars are everywhere. The
/// second account has stars and hidden articles by the tens of thousands.
const STATES: &str = r"
WITH subs AS (
    SELECT s.account_id, s.feed_id, f.last_seq,
           (hashint4(s.account_id * 31 + s.feed_id) & 2147483647) AS h,
           s.account_id = (SELECT min(id) + 1 FROM accounts) AS heavy
    FROM subscriptions s JOIN feeds f ON f.id = s.feed_id
), marks AS (
    SELECT subs.*,
           greatest(0, subs.last_seq - CASE
               WHEN subs.h % 100 < 60 THEN subs.h % 30
               WHEN subs.h % 100 < 85 THEN subs.h % 500
               WHEN subs.h % 100 < 95 THEN subs.h % 20000
               ELSE subs.last_seq END) AS mark
    FROM subs
), moved AS (
    UPDATE subscriptions s SET read_upto = m.mark
    FROM marks m WHERE s.account_id = m.account_id AND s.feed_id = m.feed_id
), flags AS (
    -- Read out of order above the mark, never right above it.
    SELECT m.account_id, m.feed_id, m.mark + n AS seq, true AS read, false AS kept_unread,
           false AS hidden, false AS starred
    FROM marks m, generate_series(2, least(m.last_seq - m.mark, 400)) n
    WHERE (hashint4(m.h + n) & 2147483647) % 5 = 0
  UNION ALL
    -- Hidden by a rule above the mark.
    SELECT m.account_id, m.feed_id, m.mark + n, false, false, true, false
    FROM marks m, generate_series(2, least(m.last_seq - m.mark, 400)) n
    WHERE (hashint4(m.h - n) & 2147483647) % 40 = 0
  UNION ALL
    -- Kept unread under the mark.
    SELECT m.account_id, m.feed_id, 1 + (hashint4(m.h + n * 17) & 2147483647) % m.mark, false,
           true, false, false
    FROM marks m, generate_series(1, 3) n
    WHERE m.mark > 0 AND (m.h + n) % 2 = 0
  UNION ALL
    -- Stars anywhere; very many for the heavy account.
    SELECT m.account_id, m.feed_id, 1 + (hashint4(m.h + n * 13) & 2147483647) % m.last_seq,
           false, false, false, true
    FROM marks m, generate_series(1, CASE WHEN m.heavy THEN 150 ELSE m.h % 12 END) n
    WHERE m.last_seq > 0
  UNION ALL
    -- Old hidden articles under the mark, for the heavy account.
    SELECT m.account_id, m.feed_id, 1 + (hashint4(m.h + n * 11) & 2147483647) % m.mark, false,
           false, true, false
    FROM marks m, generate_series(1, 40) n
    WHERE m.heavy AND m.mark > 0
)
INSERT INTO article_states (account_id, feed_id, seq, read, kept_unread, hidden, starred)
SELECT account_id, feed_id, seq, bool_or(read) AND NOT bool_or(kept_unread), bool_or(kept_unread),
       bool_or(hidden), bool_or(starred)
FROM flags
GROUP BY account_id, feed_id, seq
";

/// How long one query took over its runs.
#[derive(Debug, Clone)]
pub struct Timing {
    pub name: &'static str,
    pub runs: usize,
    pub median: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub max: Duration,
}

impl Timing {
    fn of(name: &'static str, mut times: Vec<Duration>) -> Self {
        times.sort_unstable();
        let at = |part: f64| times[((times.len() - 1) as f64 * part).round() as usize];
        Self {
            name,
            runs: times.len(),
            median: at(0.5),
            p95: at(0.95),
            p99: at(0.99),
            max: at(1.0),
        }
    }
}

/// The timings and what was measured on, printed as Markdown.
#[derive(Debug, Clone)]
pub struct Report {
    pub accounts: i64,
    pub feeds: i64,
    pub articles: i64,
    pub largest_feed: i32,
    pub states: i64,
    /// Table or index, and its size in bytes.
    pub sizes: Vec<(String, i64)>,
    pub timings: Vec<Timing>,
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{} accounts, {} feeds, {} articles (largest feed {}), {} article states\n",
            self.accounts, self.feeds, self.articles, self.largest_feed, self.states
        )?;
        writeln!(f, "| Query | Runs | Median | p95 | p99 | Max |")?;
        writeln!(f, "|---|---:|---:|---:|---:|---:|")?;
        let ms = |d: Duration| format!("{:.2} ms", d.as_secs_f64() * 1000.0);
        for t in &self.timings {
            writeln!(
                f,
                "| {} | {} | {} | {} | {} | {} |",
                t.name,
                t.runs,
                ms(t.median),
                ms(t.p95),
                ms(t.p99),
                ms(t.max)
            )?;
        }
        writeln!(f, "\n| Table or index | Size |")?;
        writeln!(f, "|---|---:|")?;
        for (name, bytes) in &self.sizes {
            writeln!(
                f,
                "| {name} | {:.0} MB |",
                *bytes as f64 / (1024.0 * 1024.0)
            )?;
        }
        Ok(())
    }
}

/// Numbers that repeat from run to run.
struct Random(u64);

impl Random {
    fn below(&mut self, n: i64) -> i64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n.max(1) as u64) as i64
    }

    fn pick<T: Copy>(&mut self, from: &[T]) -> T {
        from[self.below(from.len() as i64) as usize]
    }
}

/// Times the reader's queries on whatever the database holds, each `runs` times
/// with changing accounts, feeds and places in the lists. The queries that write
/// come last, since they change what was read.
pub async fn measure(db: &PgPool, runs: usize) -> anyhow::Result<Report> {
    let runs = runs.max(1);
    let mut random = Random(0x2545_F491_4F6C_DD1D);

    #[derive(sqlx::FromRow, Clone, Copy)]
    struct Sub {
        account_id: i32,
        feed_id: i32,
        last_seq: i32,
        folder_id: Option<i32>,
    }
    let subs: Vec<Sub> = sqlx::query_as(
        "SELECT s.account_id, s.feed_id, f.last_seq, s.folder_id
         FROM subscriptions s JOIN feeds f ON f.id = s.feed_id
         WHERE f.last_seq > 0
         ORDER BY s.account_id, s.feed_id",
    )
    .fetch_all(db)
    .await?;
    if subs.is_empty() {
        bail!("nothing to measure: no subscription with articles");
    }
    let accounts: Vec<i32> = {
        let mut ids: Vec<_> = subs.iter().map(|s| s.account_id).collect();
        ids.dedup();
        ids
    };
    let feeds_of = |account_id: i32| -> Vec<i32> {
        subs.iter()
            .filter(|s| s.account_id == account_id)
            .map(|s| s.feed_id)
            .collect()
    };
    // The account with the most subscriptions sees the longest lists.
    let widest = *accounts
        .iter()
        .max_by_key(|&&id| feeds_of(id).len())
        .context("no account")?;
    let all_feeds = feeds_of(widest);
    let largest = *subs
        .iter()
        .filter(|s| s.account_id == widest)
        .max_by_key(|s| s.last_seq)
        .context("no feed")?;
    let max_id: i64 =
        sqlx::query_scalar("SELECT coalesce(max(id), 0) FROM articles WHERE feed_id = $1")
            .bind(largest.feed_id)
            .fetch_one(db)
            .await?;
    // A folder's feeds: those of one folder of the widest account.
    let folder = subs
        .iter()
        .find(|s| s.account_id == widest && s.folder_id.is_some())
        .and_then(|s| s.folder_id);
    let folder_feeds: Vec<i32> = subs
        .iter()
        .filter(|s| s.account_id == widest && folder.is_some() && s.folder_id == folder)
        .map(|s| s.feed_id)
        .collect();

    let page = |before, unread_only| Page {
        before,
        limit: 50,
        unread_only,
    };
    let mut timings = Vec::new();
    // Runs one query `runs` times and notes how long each run took.
    macro_rules! time {
        ($name:literal, $run:expr) => {{
            let mut times = Vec::with_capacity(runs);
            for _ in 0..runs {
                let started = Instant::now();
                $run;
                times.push(started.elapsed());
            }
            timings.push(Timing::of($name, times));
        }};
    }

    time!("One feed, first page", {
        let sub = random.pick(&subs);
        articles::page(db, sub.account_id, &[sub.feed_id], page(None, false)).await?
    });
    time!("One feed, unread only", {
        let sub = random.pick(&subs);
        articles::page(db, sub.account_id, &[sub.feed_id], page(None, true)).await?
    });
    time!("Largest feed, a page deep inside", {
        let before = Some(1 + random.below(max_id));
        articles::page(db, widest, &[largest.feed_id], page(before, false)).await?
    });
    time!("A folder, unread only", {
        articles::page(db, widest, &folder_feeds, page(None, true)).await?
    });
    time!("All feeds, first page", {
        let account_id = random.pick(&accounts);
        articles::page(db, account_id, &feeds_of(account_id), page(None, false)).await?
    });
    time!("All feeds, unread only", {
        let account_id = random.pick(&accounts);
        articles::page(db, account_id, &feeds_of(account_id), page(None, true)).await?
    });
    time!("All feeds, a page deep inside", {
        let before = Some(1 + random.below(max_id));
        articles::page(db, widest, &all_feeds, page(before, false)).await?
    });
    time!("All feeds, unread only, deep inside", {
        let before = Some(1 + random.below(max_id));
        articles::page(db, widest, &all_feeds, page(before, true)).await?
    });
    time!("Unread counts of all feeds", {
        reading::unread_counts(db, random.pick(&accounts)).await?
    });
    time!("Open a new article", {
        let sub = random.pick(&subs);
        let seq = sub.last_seq - random.below(i64::from(sub.last_seq.min(100))) as i32;
        articles::open(db, sub.account_id, sub.feed_id, seq).await?
    });
    time!("Open an article of any age", {
        let sub = random.pick(&subs);
        let seq = 1 + random.below(i64::from(sub.last_seq)) as i32;
        articles::open(db, sub.account_id, sub.feed_id, seq).await?
    });
    time!("Versions of an article of any age", {
        let sub = random.pick(&subs);
        let seq = 1 + random.below(i64::from(sub.last_seq)) as i32;
        sqlx::query(
            "SELECT version, fetched_at, raw FROM article_versions WHERE feed_id = $1 AND seq = $2",
        )
        .bind(sub.feed_id)
        .bind(seq)
        .fetch_all(db)
        .await?
    });
    time!("Mark one article as read", {
        let sub = random.pick(&subs);
        let seq = sub.last_seq - random.below(i64::from(sub.last_seq.min(300))) as i32;
        reading::set_read(db, sub.account_id, sub.feed_id, seq, true).await?
    });
    time!("Mark a feed as read", {
        let sub = random.pick(&subs);
        reading::mark_all_read(db, sub.account_id, sub.feed_id, sub.last_seq).await?
    });

    let count = |table: &'static str| async move {
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
            .fetch_one(db)
            .await
            .and_then(|row| sqlx::Row::try_get::<i64, _>(&row, 0))
    };
    let sizes = sqlx::query_as(
        "SELECT c.relname::text, pg_relation_size(c.oid) + coalesce(pg_relation_size(c.reltoastrelid), 0)
         FROM pg_class c
         WHERE c.relnamespace = 'public'::regnamespace AND c.relkind IN ('r', 'i')
           AND c.relname ~ '^(articles|article_content|article_states|subscriptions)'
           AND c.relname !~ '^article_versions'
         UNION ALL
         SELECT 'article_versions (all partitions, with indexes)',
                coalesce(sum(pg_total_relation_size(i.inhrelid)), 0)::bigint
         FROM pg_inherits i WHERE i.inhparent = 'article_versions'::regclass
         ORDER BY 1",
    )
    .fetch_all(db)
    .await?;
    Ok(Report {
        accounts: accounts.len() as i64,
        feeds: count("feeds").await?,
        articles: count("articles").await?,
        largest_feed: largest.last_seq,
        states: count("article_states").await?,
        sizes,
        timings,
    })
}
