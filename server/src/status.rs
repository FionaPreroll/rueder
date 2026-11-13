//! The status of the server, for its admins: `GET /api/status`.
//!
//! It says how the server fares: how long requests take and where they wait,
//! how the database is doing, how fetching, the feed search and the image
//! proxy went in the last hour, and the warnings and errors of the log. It
//! names hints where something looks like a bottleneck, as kinds with numbers;
//! the page has the words.
//!
//! An admin sees how many feeds fail and why, but not which ones: the status
//! names no feed, no address and no account, as no endpoint lists the feeds of
//! others. What the server logs is the exception, as an admin who runs the
//! server reads it anyway.

use std::{fs, time::Instant};

use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;
use time::OffsetDateTime;
use ts_rs::TS;

use crate::{
    AppError, AppState, moderation,
    monitor::{self, Severity, Timing},
    profile, reports,
    sessions::Session,
    version,
};

/// The types this module sends, for the frontend.
pub(crate) fn types(types: &mut crate::typescript::Types) {
    types.add::<version::Build>().add::<Status>();
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/status", get(status))
}

/// How many routes the status lists, the slowest first.
const ROUTES: usize = 12;
/// How many tables it lists, the largest first.
const TABLES: i64 = 8;
/// From how many requests in an hour on a route's times say something.
const ENOUGH: u64 = 5;
/// From how many milliseconds in the 95th percentile a route counts as slow.
const SLOW_MS: f64 = 500.0;
/// From how many seconds past its time a feed counts as overdue.
const OVERDUE_S: f64 = 10.0 * 60.0;

/// What `GET /api/status` answers, for admins only.
#[derive(Serialize, TS)]
#[ts(rename = "ServerStatus")]
struct Status {
    /// The version of the server.
    version: &'static str,
    /// What this build is: commit, branch, when and how it was built.
    build: version::Build,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    started_at: OffsetDateTime,
    uptime_s: u64,
    /// The memory the server holds, if the system says.
    memory_bytes: Option<u64>,
    /// What looks like a bottleneck or a problem, the most pressing first.
    hints: Vec<Hint>,
    counts: Counts,
    requests: Requests,
    database: Database,
    fetcher: Fetcher,
    finder: Finder,
    images: Images,
    problems: Problems,
    /// Errors in the browsers of readers who agreed to send them.
    browsers: reports::Summary,
    /// Feeds readers reported, with their address: the one place the status
    /// names feeds, as the admins have to look at them. Never who reported.
    reported: Vec<moderation::Reported>,
}

/// How long things took in the last hour, in milliseconds. The times are null
/// when nothing was timed.
#[derive(Serialize, TS)]
struct Durations {
    count: u64,
    mean_ms: Option<f64>,
    p50_ms: Option<f64>,
    p95_ms: Option<f64>,
    max_ms: Option<f64>,
}

impl From<&Timing> for Durations {
    fn from(timing: &Timing) -> Self {
        Self {
            count: timing.count,
            mean_ms: timing.mean(),
            p50_ms: timing.quantile(0.5),
            p95_ms: timing.quantile(0.95),
            max_ms: timing.max(),
        }
    }
}

/// Something on the status page that wants looking at.
#[derive(Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(rename = "StatusHint")]
enum Hint {
    /// Requests to a route take long.
    SlowRoute { route: String, p95_ms: f64 },
    /// A route failed with errors of the server.
    ServerErrors { route: String, count: u64 },
    /// Every connection to the database is in use.
    PoolBusy { size: u32 },
    /// Feeds are fetched later than they are due.
    FetchBacklog { overdue: i64, most_overdue_s: f64 },
    /// Rounds come full and feeds wait: the pace, not the sites, holds the
    /// fetching back, and `FETCH_ROUND` or `FETCH_EVERY` would let more through.
    FetchPaceFull { per_hour: u64 },
    /// Many feeds fail.
    FeedsFailing { failing: i64, subscribed: i64 },
    /// The fetching does not run.
    NotFetching,
    /// The database reads much from the disk.
    CacheMisses { hit_ratio: f64 },
    /// A query has been running for a long time.
    LongQuery { seconds: f64 },
    /// The image cache is nearly full.
    ImageCacheFull { percent: f64 },
    /// The server logged errors in the last hour.
    Errors { count: u64 },
    /// Readers were turned away for asking too much.
    Limited { count: u64 },
    /// Browsers reported errors in the last day.
    BrowserErrors { errors: usize },
    /// Readers reported feeds that are not blocked.
    ReportedFeeds { feeds: usize },
}

/// How much there is.
#[derive(Serialize, TS)]
#[ts(rename = "StatusCounts")]
struct Counts {
    accounts: i64,
    feeds: i64,
    /// Feeds somebody subscribes to, which are the ones fetched.
    subscribed_feeds: i64,
    /// About how many articles: what PostgreSQL estimates.
    articles: i64,
    /// Open streams of events: about how many tabs are open.
    live_connections: usize,
}

/// The requests of the last hour.
#[derive(Serialize, TS)]
#[ts(rename = "StatusRequests")]
struct Requests {
    all: Durations,
    /// Answers by their first digit: 1xx to 5xx.
    answers: [u64; 5],
    /// Answers that said "too many requests".
    limited: u64,
    /// The routes, the slowest first by their 95th percentile.
    routes: Vec<Route>,
}

#[derive(Serialize, TS)]
#[ts(rename = "StatusRoute")]
struct Route {
    /// The method and the route, such as "GET /api/feeds/{id}/articles".
    route: String,
    took: Durations,
    /// Answers with an error of the server.
    errors: u64,
}

/// The database.
#[derive(Serialize, TS)]
#[ts(rename = "StatusDatabase")]
struct Database {
    /// The connections of the server: open, idle, at most.
    pool_size: u32,
    pool_idle: usize,
    pool_max: u32,
    /// All connections to the database, the server's and others'.
    connections: i64,
    size_bytes: i64,
    /// The share of blocks found in memory, since the statistics began.
    cache_hit_ratio: Option<f64>,
    deadlocks: i64,
    /// The query that has been running longest, in seconds.
    longest_query_s: Option<f64>,
    tables: Vec<Table>,
}

#[derive(Serialize, TS)]
#[ts(rename = "StatusTable")]
struct Table {
    name: String,
    /// With its indexes and, for a table in parts, all of them.
    bytes: i64,
    /// What PostgreSQL estimates.
    rows: i64,
}

/// Fetching feeds.
#[derive(Serialize, TS)]
#[ts(rename = "StatusFetcher")]
struct Fetcher {
    /// Whether the server fetches feeds at all.
    running: bool,
    /// The pace it fetches at, where it does.
    pace: Option<FetchPace>,
    /// How many fetches the last hour had, of all kinds.
    per_hour: u64,
    /// How long the feed that has waited longest past its time has waited, in
    /// seconds; 0 when none is due.
    behind_s: f64,
    #[serde(with = "time::serde::rfc3339::option")]
    #[ts(type = "string | null")]
    last_round_at: Option<OffsetDateTime>,
    last_round_feeds: Option<usize>,
    last_round_s: Option<f64>,
    /// The fetches of the last hour.
    took: Durations,
    fetched: u64,
    unchanged: u64,
    failed: u64,
    /// Fetches that failed at the database.
    broken: u64,
    new_articles: u64,
    /// Feeds whose last fetch failed, by why.
    failing: Vec<Failing>,
    /// Feeds fetched more than ten minutes after they were due.
    overdue: i64,
    most_overdue_s: Option<f64>,
    /// Feeds not fetched yet at all.
    waiting: i64,
}

/// How fast feeds are fetched (`FETCH_ROUND`, `FETCH_AT_ONCE`, `FETCH_EVERY`).
#[derive(Serialize, TS)]
#[ts(rename = "StatusFetchPace")]
struct FetchPace {
    round: i64,
    at_once: usize,
    every_s: u64,
    /// The most fetches an hour can have at this pace.
    per_hour: u64,
}

/// Why fetches fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename = "FetchFailure")]
enum Failure {
    /// The site answered with an error.
    Status,
    Timeout,
    /// The address leads to a network rueder does not fetch from.
    Blocked,
    /// The name has no address.
    Dns,
    /// No connection, or it broke off.
    Network,
    TooLarge,
    Redirects,
    /// What came back was no feed rueder can read.
    Unreadable,
    Other,
}

#[derive(Serialize, TS)]
#[ts(rename = "StatusFailing")]
struct Failing {
    failure: Failure,
    /// The status the site answered with, for `status`.
    status: Option<u16>,
    feeds: i64,
}

/// The feed search, in the last hour.
#[derive(Serialize, TS)]
#[ts(rename = "StatusFinder")]
struct Finder {
    /// Whether the server looks at addresses at all.
    on: bool,
    took: Durations,
    found: u64,
    none: u64,
    failed: u64,
    limited: u64,
}

/// The image proxy.
#[derive(Serialize, TS)]
#[ts(rename = "StatusImages")]
struct Images {
    on: bool,
    cache_bytes: u64,
    cache_max_bytes: u64,
    /// Sources left alone for now because they failed.
    failing_sources: usize,
    /// Images not in the cache, in the last hour.
    took: Durations,
    cached: u64,
    fetched: u64,
    not_found: u64,
    source_failed: u64,
    not_image: u64,
    too_large: u64,
}

/// The warnings and errors of the log.
#[derive(Serialize, TS)]
#[ts(rename = "StatusProblems")]
struct Problems {
    errors_last_hour: u64,
    warnings_last_hour: u64,
    /// The latest first. The same text from the same place is one.
    recent: Vec<Problem>,
}

#[derive(Serialize, TS)]
#[ts(rename = "StatusProblem")]
struct Problem {
    error: bool,
    /// Where in the server it comes from, such as "rueder::fetch".
    target: String,
    text: String,
    count: u64,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    first: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    last: OffsetDateTime,
}

async fn status(State(state): State<AppState>, session: Session) -> Result<Json<Status>, AppError> {
    profile::admin(&session)?;
    let now = Instant::now();
    let monitor = &state.monitor;

    let counts = sqlx::query!(
        r#"SELECT (SELECT count(*) FROM accounts) AS "accounts!",
                  (SELECT count(*) FROM feeds) AS "feeds!",
                  (SELECT count(DISTINCT feed_id) FROM subscriptions) AS "subscribed!""#
    )
    .fetch_one(&state.db)
    .await?;
    let tables = sqlx::query!(
        r#"SELECT c.relname::text AS "name!",
                  (SELECT coalesce(sum(pg_total_relation_size(t.relid)), 0)
                   FROM (SELECT relid FROM pg_partition_tree(c.oid) UNION SELECT c.oid) t
                  )::bigint AS "bytes!",
                  (SELECT coalesce(sum(greatest(r.reltuples, 0)), 0)
                   FROM (SELECT relid FROM pg_partition_tree(c.oid) UNION SELECT c.oid) t
                   JOIN pg_class r ON r.oid = t.relid)::bigint AS "rows!"
           FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
           WHERE n.nspname = current_schema() AND c.relkind IN ('r', 'p') AND NOT c.relispartition
           ORDER BY 2 DESC, 1
           LIMIT $1"#,
        TABLES,
    )
    .fetch_all(&state.db)
    .await?;
    let tables: Vec<Table> = tables
        .into_iter()
        .map(|table| Table {
            name: table.name,
            bytes: table.bytes,
            rows: table.rows,
        })
        .collect();
    let articles = sqlx::query_scalar!(
        r#"SELECT coalesce(sum(greatest(r.reltuples, 0)), 0)::bigint AS "n!"
           FROM (SELECT relid FROM pg_partition_tree('articles'::regclass)
                 UNION SELECT 'articles'::regclass) t
           JOIN pg_class r ON r.oid = t.relid"#
    )
    .fetch_one(&state.db)
    .await?;
    let database = sqlx::query!(
        r#"SELECT pg_database_size(current_database()) AS "size!",
                  (SELECT count(*) FROM pg_stat_activity
                   WHERE datname = current_database()) AS "connections!",
                  (SELECT extract(epoch FROM max(now() - query_start))::float8
                   FROM pg_stat_activity
                   WHERE datname = current_database() AND state = 'active'
                     AND pid <> pg_backend_pid()) AS longest,
                  d.blks_hit AS "hit!", d.blks_read AS "read!", d.deadlocks AS "deadlocks!"
           FROM pg_stat_database d WHERE d.datname = current_database()"#
    )
    .fetch_one(&state.db)
    .await?;
    let fetching = sqlx::query!(
        r#"SELECT count(*) FILTER (WHERE f.fetched_at IS NULL) AS "waiting!",
                  count(*) FILTER (WHERE f.next_fetch_at < now() - $1::float8 * interval '1 second')
                      AS "overdue!",
                  extract(epoch FROM now() - min(f.next_fetch_at)
                      FILTER (WHERE f.next_fetch_at < now() - $1::float8 * interval '1 second')
                  )::float8 AS most_overdue,
                  extract(epoch FROM now() - min(f.next_fetch_at)
                      FILTER (WHERE f.next_fetch_at < now()))::float8 AS behind
           FROM feeds f
           WHERE EXISTS (SELECT 1 FROM subscriptions s WHERE s.feed_id = f.id)"#,
        OVERDUE_S,
    )
    .fetch_one(&state.db)
    .await?;
    let messages = sqlx::query!(
        r#"SELECT f.error_message AS "message!", count(*) AS "feeds!"
           FROM feeds f
           WHERE f.error_message IS NOT NULL
             AND EXISTS (SELECT 1 FROM subscriptions s WHERE s.feed_id = f.id)
           GROUP BY f.error_message"#
    )
    .fetch_all(&state.db)
    .await?;
    let mut failing: Vec<Failing> = Vec::new();
    for row in messages {
        let (failure, status) = failure(&row.message);
        match failing
            .iter_mut()
            .find(|known| known.failure == failure && known.status == status)
        {
            Some(known) => known.feeds += row.feeds,
            None => failing.push(Failing {
                failure,
                status,
                feeds: row.feeds,
            }),
        }
    }
    failing.sort_by(|a, b| b.feeds.cmp(&a.feeds).then(a.status.cmp(&b.status)));

    let mut routes: Vec<(String, monitor::Requests)> = monitor.requests(now);
    let mut all = monitor::Requests::default();
    for (_, requests) in &routes {
        monitor::Tally::add(&mut all, requests);
    }
    routes.sort_by(|(a_route, a), (b_route, b)| {
        let p95 = |requests: &monitor::Requests| requests.timing.quantile(0.95).unwrap_or(0.0);
        p95(b).total_cmp(&p95(a)).then_with(|| a_route.cmp(b_route))
    });

    let fetches = monitor.fetches(now);
    let round = monitor.last_round();
    let pace = monitor.fetching();
    let lookups = monitor.lookups(now);
    let images = state.images.as_deref();
    let served = images.map(|images| images.served(now)).unwrap_or_default();
    let problems = monitor.problems.last_hour(now);

    let hit_ratio = {
        let blocks = database.hit + database.read;
        (blocks > 0).then(|| database.hit as f64 / blocks as f64)
    };
    let database = Database {
        pool_size: state.db.size(),
        pool_idle: state.db.num_idle(),
        pool_max: state.db.options().get_max_connections(),
        connections: database.connections,
        size_bytes: database.size,
        cache_hit_ratio: hit_ratio,
        deadlocks: database.deadlocks,
        longest_query_s: database.longest,
        tables,
    };
    let fetcher = Fetcher {
        running: pace.is_some(),
        pace: pace.map(|pace| FetchPace {
            round: pace.round,
            at_once: pace.at_once,
            every_s: pace.every.as_secs(),
            per_hour: pace.per_hour(),
        }),
        per_hour: fetches.fetched + fetches.unchanged + fetches.failed + fetches.broken,
        behind_s: fetching.behind.unwrap_or_default().max(0.0),
        last_round_at: round.map(|round| round.at),
        last_round_feeds: round.map(|round| round.feeds),
        last_round_s: round.map(|round| round.took.as_secs_f64()),
        took: Durations::from(&fetches.timing),
        fetched: fetches.fetched,
        unchanged: fetches.unchanged,
        failed: fetches.failed,
        broken: fetches.broken,
        new_articles: fetches.new_articles,
        failing,
        overdue: fetching.overdue,
        most_overdue_s: fetching.most_overdue,
        waiting: fetching.waiting,
    };
    let counts = Counts {
        accounts: counts.accounts,
        feeds: counts.feeds,
        subscribed_feeds: counts.subscribed,
        articles,
        live_connections: state.events.all_streams(),
    };
    let images = Images {
        on: images.is_some(),
        cache_bytes: images.map_or(0, |images| images.cache().used()),
        cache_max_bytes: images.map_or(0, |images| images.cache().max_bytes()),
        failing_sources: images.map_or(0, |images| images.failing_sources()),
        took: Durations::from(&served.timing),
        cached: served.cached,
        fetched: served.fetched,
        not_found: served.not_found,
        source_failed: served.source_failed,
        not_image: served.not_image,
        too_large: served.too_large,
    };

    let browsers = reports::summary(&state.db, state.reports).await?;
    let mut hints = hints(
        &routes,
        &all,
        &database,
        &fetcher,
        &counts,
        &images,
        problems.errors,
    );
    let day_ago = OffsetDateTime::now_utc() - time::Duration::DAY;
    let lately = browsers.errors.iter().filter(|error| error.last > day_ago);
    let errors = lately.count();
    if errors > 0 {
        hints.push(Hint::BrowserErrors { errors });
    }
    let reported = moderation::reported(&state.db).await?;
    let open = reported.iter().filter(|feed| !feed.blocked).count();
    if open > 0 {
        hints.push(Hint::ReportedFeeds { feeds: open });
    }
    let requests = Requests {
        all: Durations::from(&all.timing),
        answers: all.answers,
        limited: all.limited,
        routes: routes
            .into_iter()
            .take(ROUTES)
            .map(|(route, requests)| Route {
                route,
                took: Durations::from(&requests.timing),
                errors: requests.answers[4],
            })
            .collect(),
    };
    let recent = monitor.problems.list().into_iter().map(|problem| Problem {
        error: problem.severity == Severity::Error,
        target: problem.target,
        text: problem.text,
        count: problem.count,
        first: problem.first,
        last: problem.last,
    });
    Ok(Json(Status {
        version: version::VERSION,
        build: version::build(),
        started_at: monitor.started_at,
        uptime_s: monitor.started.elapsed().as_secs(),
        memory_bytes: memory(),
        hints,
        counts,
        requests,
        database,
        fetcher,
        finder: Finder {
            on: state.client.is_some(),
            took: Durations::from(&lookups.timing),
            found: lookups.found,
            none: lookups.none,
            failed: lookups.failed,
            limited: lookups.limited,
        },
        images,
        problems: Problems {
            errors_last_hour: problems.errors,
            warnings_last_hour: problems.warnings,
            recent: recent.collect(),
        },
        browsers,
        reported,
    }))
}

/// What wants looking at, the most pressing first.
fn hints(
    routes: &[(String, monitor::Requests)],
    all: &monitor::Requests,
    database: &Database,
    fetcher: &Fetcher,
    counts: &Counts,
    images: &Images,
    errors: u64,
) -> Vec<Hint> {
    let mut hints = Vec::new();
    if errors > 0 {
        hints.push(Hint::Errors { count: errors });
    }
    if database.pool_idle == 0 && database.pool_size >= database.pool_max {
        hints.push(Hint::PoolBusy {
            size: database.pool_size,
        });
    }
    if !fetcher.running {
        hints.push(Hint::NotFetching);
    } else if fetcher.overdue > 0 {
        hints.push(Hint::FetchBacklog {
            overdue: fetcher.overdue,
            most_overdue_s: fetcher.most_overdue_s.unwrap_or_default(),
        });
        if let Some(pace) = &fetcher.pace
            && fetcher.last_round_feeds == usize::try_from(pace.round).ok()
        {
            hints.push(Hint::FetchPaceFull {
                per_hour: pace.per_hour,
            });
        }
    }
    if let Some(seconds) = database.longest_query_s
        && seconds >= 30.0
    {
        hints.push(Hint::LongQuery { seconds });
    }
    for (route, requests) in routes {
        if requests.answers[4] > 0 {
            hints.push(Hint::ServerErrors {
                route: route.clone(),
                count: requests.answers[4],
            });
        }
    }
    // The routes come the slowest first. A stream of events is slow to open
    // on purpose: it says nothing until there is something to say.
    for (route, requests) in routes {
        if let Some(p95_ms) = requests.timing.quantile(0.95)
            && requests.timing.count >= ENOUGH
            && p95_ms >= SLOW_MS
        {
            hints.push(Hint::SlowRoute {
                route: route.clone(),
                p95_ms,
            });
        }
    }
    let failing: i64 = fetcher.failing.iter().map(|failing| failing.feeds).sum();
    // A fifth of the feeds or more.
    if failing > 0 && failing * 5 >= counts.subscribed_feeds {
        hints.push(Hint::FeedsFailing {
            failing,
            subscribed: counts.subscribed_feeds,
        });
    }
    if let Some(hit_ratio) = database.cache_hit_ratio
        && hit_ratio < 0.9
        && database.size_bytes > 0
    {
        hints.push(Hint::CacheMisses { hit_ratio });
    }
    if images.cache_max_bytes > 0 {
        let percent = images.cache_bytes as f64 * 100.0 / images.cache_max_bytes as f64;
        if percent >= 95.0 {
            hints.push(Hint::ImageCacheFull { percent });
        }
    }
    if all.limited > 0 {
        hints.push(Hint::Limited { count: all.limited });
    }
    hints
}

/// Why a fetch failed, from what was written to the feed: the texts of
/// [`crate::http_client::Error`], of the fetch and of reading a feed. The
/// tests hold this to those texts.
fn failure(message: &str) -> (Failure, Option<u16>) {
    if let Some(status) = message.strip_prefix("the site answers with ") {
        let code = status.get(..3).and_then(|code| code.parse().ok());
        return (Failure::Status, code);
    }
    let failure = if message == "took too long" {
        Failure::Timeout
    } else if message.contains("is not on the public internet")
        || message.starts_with("not an address rueder can fetch")
    {
        Failure::Blocked
    } else if message.ends_with(" has no address") {
        Failure::Dns
    } else if [
        "could not connect",
        "the answer broke off",
        "the request failed",
    ]
    .iter()
    .any(|start| message.starts_with(start))
    {
        Failure::Network
    } else if message.starts_with("larger than ") {
        Failure::TooLarge
    } else if message.starts_with("more than ") && message.ends_with(" redirects")
        || message == "a redirect without a usable target"
    {
        Failure::Redirects
    } else if message.starts_with("not a feed that can be read")
        || message == "the feed could not be read"
    {
        Failure::Unreadable
    } else {
        Failure::Other
    };
    (failure, None)
}

/// The memory the server holds, from Linux; None elsewhere.
fn memory() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmRSS:"))?;
    let kilobytes: u64 = line
        .trim_start_matches("VmRSS:")
        .trim()
        .trim_end_matches("kB")
        .trim()
        .parse()
        .ok()?;
    Some(kilobytes * 1024)
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use super::{Failure, failure};
    use crate::http_client::Error;

    #[test]
    fn failures_are_told_apart_by_their_text() {
        let internal = Error::Internal {
            host: "intranet.example".into(),
            addr: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            why: "private",
        };
        let cases = [
            (
                Error::Address("no host".into()).to_string(),
                Failure::Blocked,
            ),
            (internal.to_string(), Failure::Blocked),
            (Error::TooManyRedirects(5).to_string(), Failure::Redirects),
            (Error::BadRedirect.to_string(), Failure::Redirects),
            (Error::TooLarge(1000).to_string(), Failure::TooLarge),
            (Error::Timeout.to_string(), Failure::Timeout),
            (
                Error::Network("example.org has no address".into()).to_string(),
                Failure::Dns,
            ),
            (
                Error::Network("could not connect: connection refused".into()).to_string(),
                Failure::Network,
            ),
            (
                "not a feed that can be read: unexpected end".into(),
                Failure::Unreadable,
            ),
            ("the feed could not be read".into(), Failure::Unreadable),
            ("something new".into(), Failure::Other),
        ];
        for (message, expected) in cases {
            assert_eq!(failure(&message), (expected, None), "{message}");
        }
        assert_eq!(
            failure("the site answers with 403 Forbidden"),
            (Failure::Status, Some(403))
        );
    }
}
