//! Metrics for Prometheus: `GET /metrics`, in its text format (0.0.4).
//!
//! It gives what the status page of the admins shows ([`crate::status`]) in a
//! form an operator's Prometheus can collect: how many requests each route
//! had and how long they took, how fetching, the feed search and the image
//! proxy went, the warnings and errors of the log, and what PostgreSQL says
//! about the data and itself.
//!
//! - **Off** unless the operator sets `METRICS_TOKEN`: without it the route
//!   answers 404, as if it were not there.
//! - **Logging in** is that token, as `Authorization: Bearer <token>`, and
//!   nothing else: neither the session of an admin nor an app password opens
//!   it. A token shorter than [`MIN_TOKEN`] stops the start. The server keeps
//!   only its SHA-256 and compares that, so the time it takes tells nothing
//!   about the token.
//! - **Counters** count since the process started: [`crate::monitor::Window`]
//!   keeps a total next to its parts of the hour. After a restart they begin
//!   at 0 again, which Prometheus takes for a restart. Times are histograms
//!   with the buckets of [`crate::monitor::HISTOGRAM`].
//! - **The numbers from PostgreSQL** are read at most every [`FRESH`] and
//!   within [`WITHIN`]; whoever asks more often gets the ones read last. When
//!   the database does not answer, the rest is there and `rueder_database_up`
//!   is 0.
//! - **Nothing names anybody.** Labels come from fixed sets: the route as its
//!   pattern, the class of the answer, the result, the reason a feed fails,
//!   the severity. No account, no feed, no address, no text from the log or
//!   from a browser.

use std::{
    fmt::{Display, Write as _},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::bail;
use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tokio::sync::Mutex;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    AppState,
    monitor::{Requests, Timing},
    status::{self, FromDatabase},
    version,
};

/// The fewest characters a token has: 32 random ones are far beyond guessing.
pub const MIN_TOKEN: usize = 32;
/// How long the numbers from PostgreSQL are used again.
pub const FRESH: Duration = Duration::from_secs(15);
/// How long reading them may take.
pub const WITHIN: Duration = Duration::from_secs(5);
/// The type of what the route answers.
pub const CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(metrics))
}

/// The metrics of a server that gives them: the token they need, and the
/// numbers from PostgreSQL read last.
pub struct Metrics {
    token: [u8; 32],
    read: Mutex<Option<(Instant, Option<Arc<FromDb>>)>>,
}

impl Metrics {
    /// Metrics for those who know this token. It has at least [`MIN_TOKEN`]
    /// characters, all of them visible ASCII, as a header can carry them.
    pub fn new(token: &str) -> anyhow::Result<Self> {
        let token = token.trim();
        if !token.bytes().all(|byte| byte.is_ascii_graphic()) {
            bail!("METRICS_TOKEN may only have letters, digits and visible ASCII signs");
        }
        if token.len() < MIN_TOKEN {
            bail!("METRICS_TOKEN needs at least {MIN_TOKEN} characters");
        }
        Ok(Self {
            token: Sha256::digest(token.as_bytes()).into(),
            read: Mutex::default(),
        })
    }

    /// The metrics as `METRICS_TOKEN` sets them: none where it is not set or
    /// empty.
    pub fn from_setting(setting: Option<&str>) -> anyhow::Result<Option<Self>> {
        match setting.map(str::trim) {
            None | Some("") => Ok(None),
            Some(token) => Self::new(token).map(Some),
        }
    }

    /// Whether the request names the token.
    fn admits(&self, headers: &HeaderMap) -> bool {
        let Some(given) = headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().split_once(' '))
            .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
            .map(|(_, token)| token.trim())
        else {
            return false;
        };
        let given: [u8; 32] = Sha256::digest(given.as_bytes()).into();
        given == self.token
    }

    /// The numbers from PostgreSQL, read again where they are older than
    /// [`FRESH`]; none where the database did not answer in time.
    async fn database(&self, db: &PgPool) -> Option<Arc<FromDb>> {
        // Held while reading, so that two at once read once.
        let mut read = self.read.lock().await;
        if let Some((at, numbers)) = read.as_ref()
            && at.elapsed() < FRESH
        {
            return numbers.clone();
        }
        let numbers = match tokio::time::timeout(WITHIN, FromDb::read(db)).await {
            Ok(Ok(numbers)) => Some(Arc::new(numbers)),
            Ok(Err(err)) => {
                tracing::warn!("the metrics could not read the database: {err:#}");
                None
            }
            Err(_) => {
                tracing::warn!("the metrics could not read the database in time");
                None
            }
        };
        *read = Some((Instant::now(), numbers.clone()));
        numbers
    }
}

/// What the metrics read from PostgreSQL.
struct FromDb {
    status: FromDatabase,
    /// Reported feeds, by whether they are blocked.
    reported_open: i64,
    reported_blocked: i64,
    /// Different errors browsers reported in the last day, and how often.
    browser_errors: i64,
    browser_error_count: i64,
}

impl FromDb {
    async fn read(db: &PgPool) -> anyhow::Result<Self> {
        let status = status::from_database(db).await?;
        let reported = sqlx::query!(
            r#"SELECT count(*) FILTER (WHERE f.blocked_at IS NULL) AS "open!",
                      count(*) FILTER (WHERE f.blocked_at IS NOT NULL) AS "blocked!"
               FROM feeds f
               WHERE EXISTS (SELECT 1 FROM content_reports r WHERE r.feed_id = f.id)"#
        )
        .fetch_one(db)
        .await?;
        let browsers = sqlx::query!(
            r#"SELECT count(DISTINCT fingerprint) AS "errors!",
                      coalesce(sum(count), 0)::bigint AS "count!"
               FROM client_reports
               WHERE last_at > now() - interval '1 day'"#
        )
        .fetch_one(db)
        .await?;
        Ok(Self {
            status,
            reported_open: reported.open,
            reported_blocked: reported.blocked,
            browser_errors: browsers.errors,
            browser_error_count: browsers.count,
        })
    }
}

/// The metrics of the server for Prometheus, in its text format 0.0.4, for
/// whoever names the token in `METRICS_TOKEN`: requests by route, fetching,
/// the feed search, the image proxy, the warnings and errors of the log and
/// what PostgreSQL says. Counters count since the process started. Nothing in
/// it names an account, a feed or an address.
#[utoipa::path(
    get,
    path = "/metrics",
    operation_id = "metrics",
    tag = "Admin",
    security(("metrics_token" = [])),
    responses(
        (status = OK, description = "The metrics, in the text format of Prometheus 0.0.4.", body = String, content_type = "text/plain; version=0.0.4; charset=utf-8"),
        (status = UNAUTHORIZED, description = "No token, or not the one in METRICS_TOKEN."),
        (status = NOT_FOUND, description = "The server gives no metrics: METRICS_TOKEN is not set."),
    ),
)]
async fn metrics(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let Some(metrics) = state.metrics.clone() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !metrics.admits(&headers) {
        return (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"))],
            "no token, or not the right one",
        )
            .into_response();
    }
    let from_db = metrics.database(&state.db).await;
    let text = write(&state, from_db.as_deref());
    (
        [(header::CONTENT_TYPE, HeaderValue::from_static(CONTENT_TYPE))],
        text,
    )
        .into_response()
}

/// The metrics as text.
fn write(state: &AppState, from_db: Option<&FromDb>) -> String {
    let mut out = Text::default();
    let monitor = &state.monitor;

    let build = version::build();
    out.family(
        "build_info",
        "gauge",
        "The version and commit of the running server.",
    );
    out.sample(
        "build_info",
        &[("version", build.version), ("commit", build.commit)],
        1,
    );
    out.gauge(
        "start_time_seconds",
        "When the process started, in seconds since 1970.",
        monitor.started_at.unix_timestamp(),
    );
    if let Some(bytes) = status::memory() {
        out.gauge(
            "resident_memory_bytes",
            "The memory the process holds.",
            bytes,
        );
    }
    out.gauge(
        "live_connections",
        "Open streams of events: about how many tabs are open.",
        state.events.all_streams(),
    );

    requests(&mut out, &monitor.requests_total());

    let fetches = monitor.fetches_total();
    out.family(
        "fetches_total",
        "counter",
        "Fetches of feeds, by how they ended.",
    );
    for (result, count) in [
        ("fetched", fetches.fetched),
        ("unchanged", fetches.unchanged),
        ("failed", fetches.failed),
        ("broken", fetches.broken),
    ] {
        out.sample("fetches_total", &[("result", result)], count);
    }
    out.counter(
        "fetch_new_articles_total",
        "Articles the fetches brought that were new.",
        fetches.new_articles,
    );
    out.histogram(
        "fetch_duration_seconds",
        "How long a fetch took, from asking the site to having stored it.",
        &[(&[], &fetches.timing)],
    );
    let pace = monitor.fetching();
    out.gauge(
        "fetch_running",
        "Whether the server fetches feeds.",
        u8::from(pace.is_some()),
    );
    if let Some(pace) = pace {
        out.gauge(
            "fetch_round_feeds",
            "The most feeds in a round (FETCH_ROUND).",
            pace.round,
        );
        out.gauge(
            "fetch_at_once",
            "The most fetches at once (FETCH_AT_ONCE).",
            pace.at_once,
        );
        out.gauge(
            "fetch_interval_seconds",
            "How often a round begins (FETCH_EVERY).",
            pace.every.as_secs(),
        );
    }
    if let Some(round) = monitor.last_round() {
        out.gauge(
            "fetch_last_round_timestamp_seconds",
            "When the last round of fetching ended, in seconds since 1970.",
            round.at.unix_timestamp(),
        );
        out.gauge(
            "fetch_last_round_feeds",
            "How many feeds the last round of fetching had.",
            round.feeds,
        );
        out.gauge(
            "fetch_last_round_duration_seconds",
            "How long the last round of fetching took.",
            round.took.as_secs_f64(),
        );
    }

    let lookups = monitor.lookups_total();
    out.family(
        "feed_lookups_total",
        "counter",
        "Addresses readers had looked at for feeds, by how it went.",
    );
    for (result, count) in [
        ("found", lookups.found),
        ("none", lookups.none),
        ("failed", lookups.failed),
        ("limited", lookups.limited),
    ] {
        out.sample("feed_lookups_total", &[("result", result)], count);
    }
    out.histogram(
        "feed_lookup_duration_seconds",
        "How long looking at an address for feeds took.",
        &[(&[], &lookups.timing)],
    );

    if let Some(images) = state.images.as_deref() {
        let served = images.served_total();
        out.family(
            "images_total",
            "counter",
            "Images the proxy was asked for, by what it answered.",
        );
        for (result, count) in [
            ("cached", served.cached),
            ("fetched", served.fetched),
            ("not_found", served.not_found),
            ("source_failed", served.source_failed),
            ("not_image", served.not_image),
            ("too_large", served.too_large),
        ] {
            out.sample("images_total", &[("result", result)], count);
        }
        out.histogram(
            "image_duration_seconds",
            "How long images took that were not in the cache.",
            &[(&[], &served.timing)],
        );
        out.gauge(
            "image_cache_bytes",
            "What the image cache holds.",
            images.cache().used(),
        );
        out.gauge(
            "image_cache_max_bytes",
            "The most the image cache holds (IMAGE_CACHE_MB).",
            images.cache().max_bytes(),
        );
        out.gauge(
            "image_failing_sources",
            "Sources of images left alone for now because they failed.",
            images.failing_sources(),
        );
    }

    let problems = monitor.problems.total();
    out.family(
        "log_events_total",
        "counter",
        "Warnings and errors the server logged.",
    );
    out.sample(
        "log_events_total",
        &[("severity", "warning")],
        problems.warnings,
    );
    out.sample(
        "log_events_total",
        &[("severity", "error")],
        problems.errors,
    );

    let pool = &state.db;
    let idle = pool.num_idle();
    out.family(
        "db_pool_connections",
        "gauge",
        "The server's connections to the database, by whether they are in use.",
    );
    out.sample(
        "db_pool_connections",
        &[("state", "busy")],
        (pool.size() as usize).saturating_sub(idle),
    );
    out.sample("db_pool_connections", &[("state", "idle")], idle);
    out.gauge(
        "db_pool_max_connections",
        "The most connections the server opens to the database.",
        pool.options().get_max_connections(),
    );

    out.gauge(
        "database_up",
        "Whether the database answered the last time the metrics asked it.",
        u8::from(from_db.is_some()),
    );
    if let Some(from_db) = from_db {
        database(&mut out, from_db);
    }
    out.0
}

/// The requests by route: how many with which class of answer, how many were
/// turned away, and how long they took.
fn requests(out: &mut Text, routes: &[(String, Requests)]) {
    out.family(
        "http_requests_total",
        "counter",
        "Requests by route and the class of their answer. An image whose source did \
         not deliver it counts as 4xx.",
    );
    for (route, requests) in routes {
        for (class, count) in ["1xx", "2xx", "3xx", "4xx", "5xx"]
            .into_iter()
            .zip(requests.answers)
        {
            if count > 0 {
                out.sample(
                    "http_requests_total",
                    &[("route", route), ("class", class)],
                    count,
                );
            }
        }
    }
    out.family(
        "http_requests_limited_total",
        "counter",
        "Requests by route that were answered with 429 Too Many Requests.",
    );
    for (route, requests) in routes {
        out.sample(
            "http_requests_limited_total",
            &[("route", route)],
            requests.limited,
        );
    }
    let timings: Vec<([(&str, &str); 1], &Timing)> = routes
        .iter()
        .map(|(route, requests)| ([("route", route.as_str())], &requests.timing))
        .collect();
    let timings: Vec<(&[(&str, &str)], &Timing)> = timings
        .iter()
        .map(|(labels, timing)| (labels.as_slice(), *timing))
        .collect();
    out.histogram(
        "http_request_duration_seconds",
        "How long requests took until their answer began, by route.",
        &timings,
    );
}

/// What PostgreSQL says.
fn database(out: &mut Text, from_db: &FromDb) {
    let FromDatabase {
        counts,
        articles,
        database,
        fetching,
        failing,
    } = &from_db.status;
    out.gauge("accounts", "Accounts.", counts.accounts);
    out.gauge("feeds", "Feeds the server knows.", counts.feeds);
    out.gauge(
        "subscribed_feeds",
        "Feeds somebody subscribes to, which are the ones fetched.",
        counts.subscribed,
    );
    out.gauge(
        "articles_estimated",
        "About how many articles there are, as PostgreSQL estimates.",
        *articles,
    );
    out.gauge(
        "database_size_bytes",
        "The size of the database.",
        database.size,
    );
    out.gauge(
        "database_connections",
        "All connections to the database, the server's and others'.",
        database.connections,
    );
    out.family(
        "database_blocks_total",
        "counter",
        "Blocks PostgreSQL found in memory or read from the disk, since its statistics began.",
    );
    out.sample("database_blocks_total", &[("from", "memory")], database.hit);
    out.sample("database_blocks_total", &[("from", "disk")], database.read);
    out.counter(
        "database_deadlocks_total",
        "Deadlocks, since the statistics of PostgreSQL began.",
        database.deadlocks,
    );
    out.gauge(
        "database_longest_query_seconds",
        "How long the query that has been running longest has; 0 when none runs.",
        database.longest.unwrap_or_default().max(0.0),
    );
    out.gauge(
        "feeds_waiting",
        "Subscribed feeds not fetched yet at all.",
        fetching.waiting,
    );
    out.gauge(
        "feeds_overdue",
        "Subscribed feeds more than ten minutes past their time.",
        fetching.overdue,
    );
    out.gauge(
        "fetch_behind_seconds",
        "How long the feed that waits longest past its time has waited; 0 when none is due.",
        fetching.behind.unwrap_or_default().max(0.0),
    );
    out.family(
        "feeds_failing",
        "gauge",
        "Subscribed feeds whose last fetch failed, by why.",
    );
    let mut reasons: Vec<(&str, i64)> = Vec::new();
    for failing in failing {
        let reason = failing.failure.name();
        match reasons.iter_mut().find(|(known, _)| *known == reason) {
            Some((_, feeds)) => *feeds += failing.feeds,
            None => reasons.push((reason, failing.feeds)),
        }
    }
    reasons.sort();
    for (reason, feeds) in reasons {
        out.sample("feeds_failing", &[("reason", reason)], feeds);
    }
    out.family(
        "feeds_reported",
        "gauge",
        "Feeds readers reported, by whether they are blocked.",
    );
    out.sample(
        "feeds_reported",
        &[("blocked", "false")],
        from_db.reported_open,
    );
    out.sample(
        "feeds_reported",
        &[("blocked", "true")],
        from_db.reported_blocked,
    );
    out.gauge(
        "browser_errors",
        "Different errors browsers reported in the last day.",
        from_db.browser_errors,
    );
    out.gauge(
        "browser_error_reports",
        "How often browsers reported errors in the last day.",
        from_db.browser_error_count,
    );
}

/// The text format of Prometheus, written line by line. Every name gets the
/// prefix `rueder_`.
#[derive(Default)]
struct Text(String);

impl Text {
    /// Begins a family of samples: its help and its type.
    fn family(&mut self, name: &str, kind: &str, help: &str) {
        let _ = writeln!(self.0, "# HELP rueder_{name} {help}");
        let _ = writeln!(self.0, "# TYPE rueder_{name} {kind}");
    }

    fn sample(&mut self, name: &str, labels: &[(&str, &str)], value: impl Display) {
        let _ = write!(self.0, "rueder_{name}");
        if !labels.is_empty() {
            self.0.push('{');
            for (at, (label, text)) in labels.iter().enumerate() {
                if at > 0 {
                    self.0.push(',');
                }
                let _ = write!(self.0, "{label}=\"{}\"", escaped(text));
            }
            self.0.push('}');
        }
        let _ = writeln!(self.0, " {value}");
    }

    fn gauge(&mut self, name: &str, help: &str, value: impl Display) {
        self.family(name, "gauge", help);
        self.sample(name, &[], value);
    }

    fn counter(&mut self, name: &str, help: &str, value: impl Display) {
        self.family(name, "counter", help);
        self.sample(name, &[], value);
    }

    /// A histogram for each set of labels.
    fn histogram(&mut self, name: &str, help: &str, timings: &[(&[(&str, &str)], &Timing)]) {
        self.family(name, "histogram", help);
        let bucket = format!("{name}_bucket");
        for (labels, timing) in timings {
            for (seconds, count) in timing.histogram() {
                let le = seconds.to_string();
                let mut with_le = labels.to_vec();
                with_le.push(("le", &le));
                self.sample(&bucket, &with_le, count);
            }
            let mut with_le = labels.to_vec();
            with_le.push(("le", "+Inf"));
            self.sample(&bucket, &with_le, timing.count);
            self.sample(&format!("{name}_sum"), labels, timing.sum_seconds());
            self.sample(&format!("{name}_count"), labels, timing.count);
        }
    }
}

/// A label's value as the text format wants it: backslash, double quote and
/// line feed escaped.
fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue, header};

    use super::{MIN_TOKEN, Metrics, Text, escaped};

    const TOKEN: &str = "0123456789abcdefghijklmnopqrstuvwxyz";

    fn with(authorization: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(authorization).unwrap(),
        );
        headers
    }

    #[test]
    fn a_token_is_long_and_fits_a_header() {
        assert!(Metrics::new(&"a".repeat(MIN_TOKEN - 1)).is_err());
        assert!(Metrics::new(&"a".repeat(MIN_TOKEN)).is_ok());
        assert!(Metrics::new(&format!("{TOKEN} with blanks")).is_err());
        assert!(Metrics::new(&format!("{TOKEN}ä")).is_err());
        assert!(Metrics::from_setting(None).unwrap().is_none());
        assert!(Metrics::from_setting(Some("  ")).unwrap().is_none());
        assert!(Metrics::from_setting(Some("short")).is_err());
        assert!(Metrics::from_setting(Some(TOKEN)).unwrap().is_some());
    }

    #[test]
    fn only_the_token_as_bearer_opens() {
        let metrics = Metrics::new(TOKEN).unwrap();
        assert!(metrics.admits(&with(&format!("Bearer {TOKEN}"))));
        assert!(metrics.admits(&with(&format!("bearer  {TOKEN} "))));
        assert!(!metrics.admits(&HeaderMap::new()));
        assert!(!metrics.admits(&with(TOKEN)));
        assert!(!metrics.admits(&with(&format!("Basic {TOKEN}"))));
        assert!(!metrics.admits(&with(&format!("Bearer {TOKEN}x"))));
        assert!(!metrics.admits(&with(&format!("Bearer {}", &TOKEN[1..]))));
        assert!(!metrics.admits(&with("Bearer ")));
    }

    #[test]
    fn labels_are_escaped() {
        assert_eq!(escaped(r#"a\b"c"#), r#"a\\b\"c"#);
        assert_eq!(escaped("a\nb"), "a\\nb");
        let mut text = Text::default();
        text.family("x_total", "counter", "Things.");
        text.sample("x_total", &[("route", "GET /\"{id}\"")], 3);
        assert_eq!(
            text.0,
            "# HELP rueder_x_total Things.\n# TYPE rueder_x_total counter\n\
             rueder_x_total{route=\"GET /\\\"{id}\\\"\"} 3\n"
        );
    }
}
