//! How many feeds the server fetches in an hour, measured against a source of
//! made-up feeds on this machine: `rueder bench fetch`.
//!
//! Fetching takes one feed of a host at a time, so the source answers under
//! many names: one address each out of 127.0.0.0/8, which on Linux all lead
//! to this machine. The client reaches them as it reaches any network the
//! operator opens (`FETCH_ALLOW_NETS`), here opened for the run alone.
//!
//! Every feed has 20 articles to begin with and one more a minute, each with
//! a paragraph and a picture for the cleaning to work on. It answers "not
//! modified" while nothing is new, as most feeds do. Before every round all
//! feeds are made due again, so each round takes as many as the pace allows;
//! what comes out is how far the machine gets at that pace.

use std::{
    fmt,
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, bail};
use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use sqlx::PgPool;
use tokio::{net::TcpListener, task::JoinSet};

use crate::{
    articles,
    cleaning::Rules,
    events::Events,
    fetch::{self, Pace},
    http_client::{self, HttpClient},
    monitor::Monitor,
};

/// The most names the source answers under.
pub const MAX_HOSTS: u32 = 1000;
/// The most feeds a run sets up.
pub const MAX_FEEDS: u32 = 100_000;
/// The account that reads the made-up feeds.
const READER: &str = "bench-fetch";

/// What to measure.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub feeds: u32,
    pub hosts: u32,
    pub seconds: u64,
    /// The port the source answers on, under every name. A fixed one keeps the
    /// addresses of the feeds, so a second run finds the feeds of the first.
    pub port: u16,
    pub pace: Pace,
}

/// The `n`th name of the source: 127.0.1.1, 127.0.1.2, … 127.0.4.250.
fn host(n: u32) -> Ipv4Addr {
    Ipv4Addr::new(127, 0, (1 + n / 250) as u8, (1 + n % 250) as u8)
}

fn feed_url(options: &Options, n: u32) -> String {
    format!(
        "http://{}:{}/feed/{n}",
        host(n % options.hosts),
        options.port
    )
}

/// When the source started: the feeds grow by the minute from then on.
#[derive(Clone)]
struct Source {
    started: Instant,
}

impl Source {
    /// How many articles feed `n` has now; the feeds do not all grow at once.
    fn articles(&self, n: u32) -> u64 {
        let seconds = self.started.elapsed().as_secs() + u64::from(n % 60);
        20 + seconds / 60
    }
}

async fn feed(State(source): State<Source>, Path(n): Path<u32>, headers: HeaderMap) -> Response {
    let count = source.articles(n);
    let etag = format!("\"{count}\"");
    if headers
        .get(header::IF_NONE_MATCH)
        .is_some_and(|sent| sent.as_bytes() == etag.as_bytes())
    {
        return StatusCode::NOT_MODIFIED.into_response();
    }
    let mut items = String::new();
    // The newest 20, as feeds keep only their latest.
    for seq in (count.saturating_sub(20) + 1..=count).rev() {
        items.push_str(&format!(
            "<item><guid>feed-{n}-{seq}</guid><title>Report {seq} of feed {n}</title>\
             <link>https://feed-{n}.example/{seq}?utm_source=rss</link>\
             <description><![CDATA[<p>The council met again on {seq} and talked for a long \
             time about the budget, the roads and the new school. Nothing was decided, but \
             everybody agreed to meet again next week.</p>\
             <p><img src=\"https://feed-{n}.example/pictures/{seq}.jpg\" width=\"640\"></p>\
             <p>More <a href=\"https://feed-{n}.example/{seq}\">on the site</a>.</p>]]>\
             </description></item>"
        ));
    }
    let body = format!(
        "<?xml version=\"1.0\"?><rss version=\"2.0\"><channel><title>Feed {n}</title>\
         <link>https://feed-{n}.example/</link>{items}</channel></rss>"
    );
    (
        [
            (header::CONTENT_TYPE, "application/rss+xml".to_owned()),
            (header::ETAG, etag),
        ],
        body,
    )
        .into_response()
}

/// Starts the source under `hosts` names; it answers until the run ends.
#[allow(
    clippy::disallowed_methods,
    reason = "the made-up sites of the measurement, not a route of rueder"
)]
async fn serve(options: &Options) -> anyhow::Result<JoinSet<()>> {
    let app = Router::new()
        .route("/feed/{n}", get(feed))
        .with_state(Source {
            started: Instant::now(),
        });
    let mut servers = JoinSet::new();
    for n in 0..options.hosts {
        let at = SocketAddr::from((host(n), options.port));
        let listener = TcpListener::bind(at)
            .await
            .with_context(|| format!("listening on {at}"))?;
        let app = app.clone();
        servers.spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
    }
    Ok(servers)
}

/// Makes the reader and the feeds it reads, where a run before has not.
async fn set_up(db: &PgPool, options: &Options) -> anyhow::Result<()> {
    let reader: i32 = sqlx::query_scalar(
        "INSERT INTO accounts (username) VALUES ($1)
         ON CONFLICT (lower(username)) DO UPDATE SET username = EXCLUDED.username
         RETURNING id",
    )
    .bind(READER)
    .fetch_one(db)
    .await?;
    let urls: Vec<String> = (0..options.feeds).map(|n| feed_url(options, n)).collect();
    sqlx::query(
        "INSERT INTO feeds (feed_url, site_url, title)
         SELECT url, url, 'Made-up feed' FROM unnest($1::text[]) url
         WHERE NOT EXISTS (SELECT 1 FROM feeds f WHERE f.feed_url = url)",
    )
    .bind(&urls)
    .execute(db)
    .await?;
    sqlx::query(
        "INSERT INTO subscriptions (account_id, feed_id, position)
         SELECT $1, f.id, f.id FROM feeds f
         WHERE f.feed_url = ANY($2)
           AND NOT EXISTS (SELECT 1 FROM subscriptions s
                           WHERE s.account_id = $1 AND s.feed_id = f.id)",
    )
    .bind(reader)
    .bind(&urls)
    .execute(db)
    .await?;
    Ok(())
}

/// How the run went.
#[derive(Debug, Clone)]
pub struct Report {
    pub options: Options,
    pub rounds: u64,
    pub took: Duration,
    pub fetched: u64,
    pub unchanged: u64,
    pub failed: u64,
    pub broken: u64,
    pub new_articles: u64,
    pub round_mean_ms: f64,
    pub round_max_ms: f64,
    pub fetch_median_ms: Option<f64>,
    pub fetch_p95_ms: Option<f64>,
}

impl Report {
    pub fn fetches(&self) -> u64 {
        self.fetched + self.unchanged + self.failed + self.broken
    }

    /// The fetches of the run, as many an hour. A round stands for the time of
    /// the pace, or for the time it took where that was longer.
    pub fn per_hour(&self) -> f64 {
        let paced = self.options.pace.every.as_secs_f64() * self.rounds as f64;
        let time = self.took.as_secs_f64().max(paced).max(f64::EPSILON);
        self.fetches() as f64 * 3600.0 / time
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let o = &self.options;
        let ms = |value: Option<f64>| value.map_or("-".to_owned(), |ms| format!("{ms:.0} ms"));
        writeln!(
            f,
            "{} feeds under {} names, {} s; pace: {} feeds every {} s, {} at once \
             (at most {} an hour)",
            o.feeds,
            o.hosts,
            self.took.as_secs(),
            o.pace.round,
            o.pace.every.as_secs(),
            o.pace.at_once,
            o.pace.per_hour()
        )?;
        writeln!(
            f,
            "{} rounds, {:.0} ms each on average, the longest {:.0} ms",
            self.rounds, self.round_mean_ms, self.round_max_ms
        )?;
        writeln!(
            f,
            "{} fetches: {} with the feed, {} unchanged, {} failed, {} failed at the database; \
             {} new articles",
            self.fetches(),
            self.fetched,
            self.unchanged,
            self.failed,
            self.broken,
            self.new_articles
        )?;
        writeln!(
            f,
            "a fetch: {} in the median, {} in the 95th percentile",
            ms(self.fetch_median_ms),
            ms(self.fetch_p95_ms)
        )?;
        write!(f, "{:.0} fetches an hour", self.per_hour())
    }
}

/// Sets up the feeds, starts the source and fetches for as long as asked.
/// Wants a database of its own: it refuses one where somebody reads other feeds.
pub async fn run(db: &PgPool, options: Options) -> anyhow::Result<Report> {
    if options.hosts == 0 || options.hosts > MAX_HOSTS {
        bail!("--hosts must be from 1 to {MAX_HOSTS}");
    }
    if options.feeds == 0 || options.feeds > MAX_FEEDS {
        bail!("--feeds must be from 1 to {MAX_FEEDS}");
    }
    let prefix = "http://127.0.";
    let others: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM feeds f
         WHERE NOT starts_with(f.feed_url, $1)
           AND EXISTS (SELECT 1 FROM subscriptions s WHERE s.feed_id = f.id)",
    )
    .bind(prefix)
    .fetch_one(db)
    .await?;
    if others > 0 {
        bail!(
            "{others} other feeds have readers here; measure on a database of its own, \
             so nothing else is fetched"
        );
    }
    set_up(db, &options).await?;
    // As the server does when it starts, so fetches do not make them at once.
    articles::ensure_current_partitions(db).await?;
    let mut source = serve(&options).await?;

    let client = HttpClient::new(http_client::Config {
        allow: vec![
            "127.0.0.0/8"
                .parse()
                .map_err(|err: String| anyhow::anyhow!(err))?,
        ],
        ..Default::default()
    })?;
    let rules = Arc::new(Rules::built_in()?);
    let (events, monitor) = (Events::new(), Arc::new(Monitor::default()));
    let mut every = tokio::time::interval(options.pace.every);
    every.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let (began, mut rounds, mut round_ms) = (Instant::now(), 0u64, Vec::new());
    let end = began + Duration::from_secs(options.seconds);
    while Instant::now() < end {
        every.tick().await;
        // Every round finds all it may take.
        sqlx::query(
            "UPDATE feeds SET next_fetch_at = now()
             WHERE starts_with(feed_url, $1)",
        )
        .bind(prefix)
        .execute(db)
        .await?;
        let round = Instant::now();
        fetch::fetch_due(db, &client, &rules, &events, &monitor, &options.pace).await?;
        round_ms.push(round.elapsed().as_secs_f64() * 1000.0);
        rounds += 1;
    }
    let took = began.elapsed();
    source.abort_all();

    let fetches = monitor.fetches(Instant::now());
    Ok(Report {
        options,
        rounds,
        took,
        fetched: fetches.fetched,
        unchanged: fetches.unchanged,
        failed: fetches.failed,
        broken: fetches.broken,
        new_articles: fetches.new_articles,
        round_mean_ms: round_ms.iter().sum::<f64>() / round_ms.len().max(1) as f64,
        round_max_ms: round_ms.iter().copied().fold(0.0, f64::max),
        fetch_median_ms: fetches.timing.quantile(0.5),
        fetch_p95_ms: fetches.timing.quantile(0.95),
    })
}
