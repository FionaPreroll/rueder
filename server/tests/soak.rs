//! A soak test: the server program as it runs in production, for a while
//! under the steady load of readers, with feeds that keep growing and change
//! their texts, and with images through the proxy. Every request has to get
//! the answer it should, the unread counts have to agree with the lists, the
//! log has to stay free of errors and panics, and the server has to stop
//! cleanly when told to. On a longer run, what the server holds (memory, file
//! handles, threads, connections to the database) and how fast it answers
//! have to stay level as well.
//!
//! `SOAK_SECONDS` says how long the load lasts, 20 seconds unless set: long
//! enough for every kind of request and a few rounds of fetching. A run of
//! [`LONG`] or more checks the levels too. For a measurement, with the table
//! of samples:
//!
//! ```sh
//! SOAK_SECONDS=3600 cargo test --release --test soak -- --nocapture
//! ```
//!
//! Everything is made up and stays on the machine: the feeds and images come
//! from a small server of the test's own on 127.0.0.1.

use std::{
    env, fs,
    io::Cursor,
    net::SocketAddr,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use axum::{
    Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use image::{ImageFormat, Rgb, RgbImage};
use reqwest::Method;
use rueder::{accounts, health};
use serde_json::{Value, json};
use sqlx::PgPool;
use time::{OffsetDateTime, format_description::well_known::Rfc2822};
use tokio::{net::TcpListener, task::JoinHandle};

const READERS: usize = 6;
const FEEDS: usize = 24;
const PASSWORD: &str = "made-up password of the soak test";
/// The name the server's connections carry in `pg_stat_activity`.
const APPLICATION: &str = "rueder-soak";
/// From this long on, the levels are checked too.
const LONG: Duration = Duration::from_secs(5 * 60);
/// How many articles a feed of the source lists, the newest first.
const LISTED: usize = 60;
/// The images the source serves, by width and height: some fit every size,
/// some are made smaller by the proxy.
const IMAGES: [(u32, u32); 8] = [
    (16, 16),
    (64, 48),
    (200, 120),
    (640, 480),
    (900, 300),
    (1200, 900),
    (300, 2400),
    (2400, 1600),
];

#[sqlx::test]
async fn the_server_holds_steady_under_load(db: PgPool) {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let duration = Duration::from_secs(
        env::var("SOAK_SECONDS")
            .ok()
            .and_then(|seconds| seconds.parse().ok())
            .unwrap_or(20),
    );

    let source = Source::start().await;
    let readers = made_up_readers(&db, &source).await;
    let server = Server::start(&db).await;
    println!(
        "soak test: {READERS} readers, {FEEDS} feeds, {}s, server pid {}",
        duration.as_secs(),
        server.pid()
    );

    let stats = Arc::new(Stats::default());
    let stop = Arc::new(AtomicBool::new(false));
    let http = browser();

    // Fetching is due every quarter of an hour at most; here it is due on
    // every round of the server, every 30 seconds.
    let due = {
        let (db, stop) = (db.clone(), stop.clone());
        tokio::spawn(async move {
            while !stop.load(Ordering::Relaxed) {
                sqlx::query("UPDATE feeds SET next_fetch_at = now()")
                    .execute(&db)
                    .await
                    .unwrap();
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        })
    };
    let reading: Vec<JoinHandle<Reader>> = readers
        .into_iter()
        .enumerate()
        .map(|(n, name)| {
            let mut reader = Reader::new(n, name, &server, http.clone(), stats.clone());
            let stop = stop.clone();
            tokio::spawn(async move {
                reader.login().await;
                while !stop.load(Ordering::Relaxed) {
                    reader.visit().await;
                }
                reader
            })
        })
        .collect();

    // The samples, until the time is up.
    let every = (duration / 60).clamp(Duration::from_secs(2), Duration::from_secs(30));
    let started = Instant::now();
    let mut samples = Vec::new();
    while started.elapsed() < duration {
        tokio::time::sleep(every.min(duration.saturating_sub(started.elapsed()))).await;
        samples.push(server.sample(&db, &stats, started.elapsed()).await);
        if let Some(last) = samples.last() {
            println!("{last}");
        }
    }

    stop.store(true, Ordering::Relaxed);
    let mut readers = Vec::new();
    for reader in reading {
        readers.push(reader.await.unwrap());
    }
    due.await.unwrap();

    // What the readers were told matches what is there, once nothing changes
    // any more.
    source.freeze();
    sqlx::query("UPDATE feeds SET next_fetch_at = now() + interval '1 day'")
        .execute(&db)
        .await
        .unwrap();
    settle(&db).await;
    for reader in &readers {
        reader.check_counts().await;
    }
    for reader in &mut readers {
        reader.end().await;
    }

    let summary = stats.summary();
    println!("{summary}");
    println!("slowest:");
    for (time, what) in stats.slow.lock().unwrap().iter() {
        let what: String = what.chars().take(90).collect();
        println!("{:>9.1} ms  {what}", time.as_secs_f64() * 1000.0);
    }
    let log = server.stop().await;

    let errors = stats.errors.lock().unwrap().clone();
    assert!(errors.is_empty(), "unexpected answers: {errors:#?}");
    assert!(summary.requests > 0, "no requests were made");
    assert!(summary.events > 0, "no events reached the readers");
    assert!(summary.images > 0, "no images came through the proxy");
    let articles: i64 = sqlx::query_scalar("SELECT count(*) FROM articles")
        .fetch_one(&db)
        .await
        .unwrap();
    assert!(articles > 0, "nothing was fetched");
    let failing: Vec<String> = log
        .lines()
        .filter(|line| line.contains("panicked") || line.contains(" ERROR "))
        .map(str::to_owned)
        .collect();
    assert!(failing.is_empty(), "the server logged: {failing:#?}");
    for sample in &samples {
        assert!(sample.connections <= 10, "{sample}");
    }
    if duration >= LONG {
        assert_level(&samples);
    }
}

/// What the readers send their requests with, as a browser would.
// The readers are browsers: they talk to the server under test on 127.0.0.1
// and to nothing else. The rule is for what the server itself fetches.
#[allow(clippy::disallowed_methods)]
fn browser() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap()
}

/// The articles of the source stop changing: the last fetch may still be on
/// its way, so wait until the numbers stay.
async fn settle(db: &PgPool) {
    let mut last = -1;
    for _ in 0..30 {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let seqs: i64 = sqlx::query_scalar("SELECT coalesce(sum(last_seq), 0)::int8 FROM feeds")
            .fetch_one(db)
            .await
            .unwrap();
        if seqs == last {
            return;
        }
        last = seqs;
    }
    panic!("the feeds kept changing after the source stopped");
}

/// After the warm-up, what the server holds stays where it was, and so does
/// how fast it answers: the median of the last tenth of the samples against
/// the one of the samples between a quarter and a third of the time.
fn assert_level(samples: &[Sample]) {
    let warm: Vec<&Sample> = samples
        .iter()
        .skip(samples.len() / 4)
        .take((samples.len() / 12).max(1))
        .collect();
    let end: Vec<&Sample> = samples
        .iter()
        .skip(samples.len() - (samples.len() / 10).max(1))
        .collect();
    let median = |samples: &[&Sample], of: fn(&Sample) -> u64| {
        let mut values: Vec<u64> = samples.iter().map(|sample| of(sample)).collect();
        values.sort_unstable();
        values[values.len() / 2]
    };
    let compare = |what: &str, of: fn(&Sample) -> u64, allowed: fn(u64) -> u64| {
        let (before, after) = (median(&warm, of), median(&end, of));
        assert!(
            after <= allowed(before),
            "{what} grew from {before} to {after}"
        );
    };
    // A quarter more memory, at least 32 MB, is what the allocator may keep.
    compare(
        "memory (kB)",
        |sample| sample.rss_kb,
        |kb| kb + (kb / 4).max(32 * 1024),
    );
    compare("open files", |sample| sample.files, |files| files + 16);
    // Threads come and go with the work that blocks, such as decoding images.
    compare("threads", |sample| sample.threads, |threads| threads + 16);
    compare(
        "99th percentile (µs)",
        |sample| sample.p99.as_micros() as u64,
        |micros| (micros * 3).max(250_000),
    );
}

/// The feeds and images, made up as the time goes by.
#[derive(Clone)]
struct Source {
    address: SocketAddr,
    started: Instant,
    /// Where the time stands still once the test is over.
    frozen: Arc<Mutex<Option<Duration>>>,
    images: Arc<Vec<Vec<u8>>>,
}

impl Source {
    #[allow(
        clippy::disallowed_methods,
        reason = "the made-up site of the test, not a route of rueder"
    )]
    async fn start() -> Self {
        let images = IMAGES
            .iter()
            .map(|&(width, height)| {
                let image = RgbImage::from_fn(width, height, |x, y| {
                    Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8])
                });
                let mut bytes = Vec::new();
                image
                    .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
                    .unwrap();
                bytes
            })
            .collect();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let source = Self {
            address: listener.local_addr().unwrap(),
            started: Instant::now(),
            frozen: Arc::default(),
            images: Arc::new(images),
        };
        let app = Router::new()
            .route("/feeds/{n}", get(feed))
            .route("/images/{n}", get(picture))
            .with_state(source.clone());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        source
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.address)
    }

    fn elapsed(&self) -> Duration {
        let frozen = *self.frozen.lock().unwrap();
        frozen.unwrap_or_else(|| self.started.elapsed())
    }

    fn freeze(&self) {
        *self.frozen.lock().unwrap() = Some(self.started.elapsed());
    }
}

/// Feed `n` has a new article every `10 + 3n` seconds, and every fifth of its
/// articles is written anew every minute and a half. Feed 1 fails every other
/// minute.
async fn feed(State(source): State<Source>, Path(n): Path<usize>) -> Response {
    let elapsed = source.elapsed();
    if n == 1 && (elapsed.as_secs() / 60) % 2 == 1 {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    let every = 10 + 3 * n as u64;
    let count = 10 + elapsed.as_secs() / every;
    let revision = elapsed.as_secs() / 90;
    let now = OffsetDateTime::now_utc();
    let items: String = (count.saturating_sub(LISTED as u64) + 1..=count)
        .rev()
        .map(|k| {
            let published = now - time::Duration::seconds(((count - k) * every) as i64);
            let revision = if k % 5 == 0 { revision } else { 0 };
            let image = source.url(&format!("/images/{}", k as usize % IMAGES.len()));
            let text = format!(
                "<p>Article {k} of feed {n}, revision {revision}.</p>\
                 <p><img src=\"{image}\" alt=\"picture {k}\"></p>\
                 <p><a href=\"https://example.org/{n}/{k}?utm_source=soak\">Read on</a></p>\
                 <script>track({k})</script><iframe src=\"https://ads.example/\"></iframe>"
            );
            format!(
                "<item><guid>feed-{n}-{k}</guid><title>Article {k} of feed {n}</title>\
                 <link>https://example.org/{n}/{k}</link>\
                 <pubDate>{}</pubDate><category>topic-{}</category>\
                 <description><![CDATA[{text}]]></description></item>",
                published.format(&Rfc2822).unwrap(),
                k % 4,
            )
        })
        .collect();
    let body = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><rss version=\"2.0\"><channel>\
         <title>Feed {n}</title><link>https://example.org/{n}/</link>{items}</channel></rss>"
    );
    ([(header::CONTENT_TYPE, "application/rss+xml")], body).into_response()
}

async fn picture(State(source): State<Source>, Path(n): Path<String>) -> Response {
    let n: usize = n.parse().unwrap_or(0);
    match source.images.get(n) {
        Some(bytes) => ([(header::CONTENT_TYPE, "image/png")], bytes.clone()).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Accounts with two folders each and most of the feeds, which nobody has
/// fetched yet.
async fn made_up_readers(db: &PgPool, source: &Source) -> Vec<String> {
    let mut feeds = Vec::new();
    for n in 0..FEEDS {
        let id: i32 = sqlx::query_scalar("INSERT INTO feeds (feed_url) VALUES ($1) RETURNING id")
            .bind(source.url(&format!("/feeds/{n}")))
            .fetch_one(db)
            .await
            .unwrap();
        feeds.push(id);
    }
    let mut names = Vec::new();
    for r in 0..READERS {
        let name = format!("reader{r}");
        let account_id = accounts::create(db, &name, Some(PASSWORD), false)
            .await
            .unwrap()
            .unwrap();
        let mut folders = Vec::new();
        for (position, title) in ["News", "Blogs"].iter().enumerate() {
            let id: i32 = sqlx::query_scalar(
                "INSERT INTO folders (account_id, title, position) VALUES ($1, $2, $3) RETURNING id",
            )
            .bind(account_id)
            .bind(title)
            .bind(position as f64)
            .fetch_one(db)
            .await
            .unwrap();
            folders.push(id);
        }
        for (n, feed_id) in feeds.iter().enumerate() {
            if (n + r) % 3 == 0 {
                continue;
            }
            let folder = match n % 3 {
                0 => None,
                k => Some(folders[k - 1]),
            };
            sqlx::query(
                "INSERT INTO subscriptions (account_id, feed_id, folder_id, position)
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(account_id)
            .bind(feed_id)
            .bind(folder)
            .bind(n as f64)
            .execute(db)
            .await
            .unwrap();
        }
        names.push(name);
    }
    names
}

/// The server program, on a port of its own and the test's database.
struct Server {
    child: Child,
    base: String,
    log: PathBuf,
    dir: PathBuf,
}

impl Server {
    async fn start(db: &PgPool) -> Self {
        let dir = env::temp_dir().join(format!(
            "rueder-soak-{}-{}",
            std::process::id(),
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        fs::create_dir_all(dir.join("static")).unwrap();
        fs::write(
            dir.join("static/index.html"),
            "<!doctype html><title>rueder</title>",
        )
        .unwrap();
        let log = dir.join("server.log");

        let mut url = url::Url::parse(&env::var("DATABASE_URL").unwrap()).unwrap();
        url.set_path(db.connect_options().get_database().unwrap());
        url.query_pairs_mut()
            .clear()
            .append_pair("application_name", APPLICATION);
        let port = {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.local_addr().unwrap().port()
        };
        let out = fs::File::create(&log).unwrap();
        let child = Command::new(env!("CARGO_BIN_EXE_rueder"))
            .arg("serve")
            .env("DATABASE_URL", url.as_str())
            .env("LISTEN_ADDR", format!("127.0.0.1:{port}"))
            .env("STATIC_DIR", dir.join("static"))
            .env("IMAGE_CACHE_DIR", dir.join("images"))
            .env(
                "IMAGE_PROXY_KEY",
                "made-up key of the soak test, 32 bytes or more",
            )
            .env("FETCH_ALLOW_NETS", "127.0.0.0/8")
            .env("CLIENT_IP_HEADER", "X-Forwarded-For")
            .env("RUST_LOG", "rueder=info")
            .stdout(out.try_clone().unwrap())
            .stderr(out)
            .stdin(Stdio::null())
            .spawn()
            .unwrap();
        let server = Self {
            child,
            base: format!("http://127.0.0.1:{port}"),
            log,
            dir,
        };
        let address = format!("127.0.0.1:{port}").parse().unwrap();
        for _ in 0..600 {
            if health::check(address, Duration::from_millis(500))
                .await
                .is_ok()
            {
                return server;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!(
            "the server did not start: {}",
            fs::read_to_string(&server.log).unwrap_or_default()
        );
    }

    fn pid(&self) -> u32 {
        self.child.id()
    }

    /// What the server holds now, and how it answered since the last sample.
    async fn sample(&self, db: &PgPool, stats: &Stats, at: Duration) -> Sample {
        let status = fs::read_to_string(format!("/proc/{}/status", self.pid())).unwrap_or_default();
        let field = |name: &str| {
            status
                .lines()
                .find_map(|line| line.strip_prefix(name))
                .and_then(|rest| rest.split_whitespace().next())
                .and_then(|value| value.parse().ok())
                .unwrap_or(0)
        };
        let files = fs::read_dir(format!("/proc/{}/fd", self.pid()))
            .map(|entries| entries.count() as u64)
            .unwrap_or(0);
        let connections: i64 =
            sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE application_name = $1")
                .bind(APPLICATION)
                .fetch_one(db)
                .await
                .unwrap();
        let articles: i64 = sqlx::query_scalar("SELECT count(*) FROM articles")
            .fetch_one(db)
            .await
            .unwrap();
        let (requests, p50, p99) = stats.window();
        Sample {
            at,
            rss_kb: field("VmRSS:"),
            threads: field("Threads:"),
            files,
            connections,
            requests,
            p50,
            p99,
            articles,
            errors: stats.errors.lock().unwrap().len(),
        }
    }

    /// Stops the server as `docker stop` does and returns its log.
    async fn stop(mut self) -> String {
        let killed = Command::new("kill")
            .args(["-TERM", &self.pid().to_string()])
            .status()
            .unwrap();
        assert!(killed.success());
        let mut waited = Duration::ZERO;
        let status = loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                break status;
            }
            assert!(
                waited < Duration::from_secs(30),
                "the server did not stop when told to"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
            waited += Duration::from_millis(100);
        };
        let log = fs::read_to_string(&self.log).unwrap_or_default();
        assert!(status.success(), "the server ended with {status}: {log}");
        let _ = fs::remove_dir_all(&self.dir);
        log
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

struct Sample {
    at: Duration,
    rss_kb: u64,
    threads: u64,
    files: u64,
    connections: i64,
    requests: usize,
    p50: Duration,
    p99: Duration,
    articles: i64,
    errors: usize,
}

impl std::fmt::Display for Sample {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:>6}s  {:>7.1} MB  {:>3} threads  {:>3} files  {:>2} db  {:>6} requests  \
             p50 {:>6.1} ms  p99 {:>7.1} ms  {:>7} articles  {} errors",
            self.at.as_secs(),
            self.rss_kb as f64 / 1024.0,
            self.threads,
            self.files,
            self.connections,
            self.requests,
            self.p50.as_secs_f64() * 1000.0,
            self.p99.as_secs_f64() * 1000.0,
            self.articles,
            self.errors,
        )
    }
}

/// How the server answered: the times since the last sample, and what went
/// wrong.
#[derive(Default)]
struct Stats {
    window: Mutex<Vec<Duration>>,
    all: Mutex<Vec<Duration>>,
    errors: Mutex<Vec<String>>,
    events: AtomicU64,
    images: AtomicU64,
    /// The slowest requests, for the report.
    slow: Mutex<Vec<(Duration, String)>>,
}

impl Stats {
    fn took(&self, time: Duration, what: String) {
        self.window.lock().unwrap().push(time);
        let mut slow = self.slow.lock().unwrap();
        if slow.len() < 10 || slow.last().is_some_and(|(slowest, _)| *slowest < time) {
            slow.push((time, what));
            slow.sort_by_key(|(time, _)| std::cmp::Reverse(*time));
            slow.truncate(10);
        }
    }

    fn error(&self, what: String) {
        let mut errors = self.errors.lock().unwrap();
        if errors.len() < 50 {
            errors.push(what);
        }
    }

    /// How many requests since the last time, and their median and 99th
    /// percentile.
    fn window(&self) -> (usize, Duration, Duration) {
        let mut window = std::mem::take(&mut *self.window.lock().unwrap());
        self.all.lock().unwrap().extend_from_slice(&window);
        window.sort_unstable();
        (
            window.len(),
            percentile(&window, 50),
            percentile(&window, 99),
        )
    }

    fn summary(&self) -> Summary {
        let mut all = self.all.lock().unwrap().clone();
        all.extend_from_slice(&self.window.lock().unwrap());
        all.sort_unstable();
        Summary {
            requests: all.len(),
            p50: percentile(&all, 50),
            p99: percentile(&all, 99),
            max: all.last().copied().unwrap_or_default(),
            events: self.events.load(Ordering::Relaxed),
            images: self.images.load(Ordering::Relaxed),
        }
    }
}

fn percentile(sorted: &[Duration], percent: usize) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    sorted[(sorted.len() * percent / 100).min(sorted.len() - 1)]
}

struct Summary {
    requests: usize,
    p50: Duration,
    p99: Duration,
    max: Duration,
    events: u64,
    images: u64,
}

impl std::fmt::Display for Summary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} requests, p50 {:.1} ms, p99 {:.1} ms, slowest {:.1} ms, {} events, {} images",
            self.requests,
            self.p50.as_secs_f64() * 1000.0,
            self.p99.as_secs_f64() * 1000.0,
            self.max.as_secs_f64() * 1000.0,
            self.events,
            self.images,
        )
    }
}

/// A made-up reader who logs in, keeps a tab open for the events, and reads
/// what is new, with a short pause between two requests.
struct Reader {
    name: String,
    /// Where the reader's requests come from, in `X-Forwarded-For`: each
    /// reader has its own, as behind a reverse proxy.
    address: String,
    base: String,
    http: reqwest::Client,
    stats: Arc<Stats>,
    token: String,
    events: Option<JoinHandle<()>>,
    random: u64,
}

impl Reader {
    fn new(
        n: usize,
        name: String,
        server: &Server,
        http: reqwest::Client,
        stats: Arc<Stats>,
    ) -> Self {
        Self {
            name,
            address: format!("10.0.0.{}", n + 1),
            base: server.base.clone(),
            http,
            stats,
            token: String::new(),
            events: None,
            random: 0x9e37_79b9_7f4a_7c15 ^ (n as u64 + 1),
        }
    }

    /// A number below `n`, from xorshift.
    fn below(&mut self, n: usize) -> usize {
        self.random ^= self.random << 13;
        self.random ^= self.random >> 7;
        self.random ^= self.random << 17;
        (self.random % n.max(1) as u64) as usize
    }

    fn one_in(&mut self, n: usize) -> bool {
        self.below(n) == 0
    }

    fn request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        self.http
            .request(method, format!("{}{path}", self.base))
            .header("X-Forwarded-For", &self.address)
            .header("Sec-Fetch-Site", "same-origin")
            .header(
                header::COOKIE,
                format!("{}={}", rueder::sessions::COOKIE, self.token),
            )
    }

    /// Sends a request, notes how long it took, and returns the answer if it
    /// is one of `expected`.
    async fn send(
        &mut self,
        method: Method,
        path: &str,
        body: Option<Value>,
        expected: &[u16],
    ) -> Option<Vec<u8>> {
        let mut request = self.request(method.clone(), path);
        if let Some(body) = body {
            request = request
                .header(header::CONTENT_TYPE, "application/json")
                .body(body.to_string());
        }
        let started = Instant::now();
        let answer = request.send().await;
        let result = match answer {
            Ok(answer) => {
                let status = answer.status().as_u16();
                let bytes = answer.bytes().await.map(|bytes| bytes.to_vec());
                match bytes {
                    Ok(bytes) if expected.contains(&status) => Some(bytes),
                    Ok(bytes) => {
                        self.stats.error(format!(
                            "{} {method} {path}: {status} {}",
                            self.name,
                            String::from_utf8_lossy(&bytes[..bytes.len().min(200)])
                        ));
                        None
                    }
                    Err(err) => {
                        self.stats
                            .error(format!("{} {method} {path}: {err}", self.name));
                        None
                    }
                }
            }
            Err(err) => {
                self.stats
                    .error(format!("{} {method} {path}: {err}", self.name));
                None
            }
        };
        self.stats
            .took(started.elapsed(), format!("{method} {path}"));
        // A reader takes a moment between two requests, which keeps each of
        // them well below the limit for an address.
        let pause = 20 + self.below(60) as u64;
        tokio::time::sleep(Duration::from_millis(pause)).await;
        result
    }

    async fn json(&mut self, path: &str) -> Option<Value> {
        let bytes = self.send(Method::GET, path, None, &[200]).await?;
        serde_json::from_slice(&bytes).ok()
    }

    async fn login(&mut self) {
        let answer = self
            .request(Method::POST, "/api/login")
            .header(header::CONTENT_TYPE, "application/json")
            .body(json!({ "username": self.name, "password": PASSWORD }).to_string())
            .send()
            .await
            .unwrap();
        assert_eq!(answer.status(), StatusCode::OK, "{} logs in", self.name);
        let cookie = answer.headers()[header::SET_COOKIE].to_str().unwrap();
        self.token = cookie
            .split(';')
            .next()
            .and_then(|pair| pair.split_once('='))
            .map(|(_, token)| token.to_owned())
            .unwrap();
        self.listen();
    }

    /// The open tab: the events as they come, counted.
    fn listen(&mut self) {
        if let Some(old) = self.events.take() {
            old.abort();
        }
        let request = self.request(Method::GET, "/api/events");
        let stats = self.stats.clone();
        let name = self.name.clone();
        self.events = Some(tokio::spawn(async move {
            // As a browser does, a stream that ends is opened again; the
            // server ends each after half an hour.
            loop {
                let Some(request) = request.try_clone() else {
                    return;
                };
                let mut answer = match request.send().await {
                    Ok(answer) if answer.status().is_success() => answer,
                    Ok(answer) => {
                        stats.error(format!("{name} GET /api/events: {}", answer.status()));
                        return;
                    }
                    Err(err) => {
                        stats.error(format!("{name} GET /api/events: {err}"));
                        return;
                    }
                };
                while let Ok(Some(chunk)) = answer.chunk().await {
                    let events = String::from_utf8_lossy(&chunk).matches("data:").count();
                    stats.events.fetch_add(events as u64, Ordering::Relaxed);
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }));
    }

    async fn logout(&mut self) {
        // The tab goes first, or it would open its stream again with the
        // session that ends.
        if let Some(events) = self.events.take() {
            events.abort();
        }
        self.send(Method::POST, "/api/logout", None, &[204]).await;
    }

    async fn end(&mut self) {
        self.logout().await;
    }

    /// One visit: a look at the tree, a list with what is new in a feed, a
    /// few articles opened with their images and marked as read, and now and
    /// then something else a reader does.
    async fn visit(&mut self) {
        let Some(tree) = self.json("/api/tree").await else {
            return;
        };
        let feeds: Vec<(i64, i64, i64)> = tree["feeds"]
            .as_array()
            .map(|feeds| {
                feeds
                    .iter()
                    .map(|feed| {
                        let number = |key: &str| feed[key].as_i64().unwrap_or(0);
                        (number("id"), number("unread"), number("last_seq"))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let folders: Vec<i64> = tree["folders"]
            .as_array()
            .map(|folders| folders.iter().filter_map(|f| f["id"].as_i64()).collect())
            .unwrap_or_default();
        if feeds.is_empty() {
            return;
        }
        let with_unread: Vec<_> = feeds.iter().filter(|feed| feed.1 > 0).collect();
        let &(feed_id, _, last_seq) = if with_unread.is_empty() {
            &feeds[self.below(feeds.len())]
        } else {
            with_unread[self.below(with_unread.len())]
        };

        let path = format!("/api/feeds/{feed_id}/articles?unread=true&limit=20");
        let listed = self.json(&path).await;
        let seqs: Vec<i64> = listed
            .as_ref()
            .and_then(|list| list["articles"].as_array())
            .map(|articles| articles.iter().filter_map(|a| a["seq"].as_i64()).collect())
            .unwrap_or_default();
        for &seq in seqs.iter().take(3) {
            let path = format!("/api/feeds/{feed_id}/articles/{seq}");
            let Some(article) = self.json(&path).await else {
                continue;
            };
            let html = article["html"].as_str().unwrap_or_default().to_owned();
            for image in html.split("src=\"").skip(1) {
                let address = image.split('"').next().unwrap_or_default().to_owned();
                if self
                    .send(Method::GET, &address, None, &[200])
                    .await
                    .is_some()
                {
                    self.stats.images.fetch_add(1, Ordering::Relaxed);
                }
            }
            let state = if self.one_in(6) {
                json!({ "read": true, "starred": true })
            } else {
                json!({ "read": true })
            };
            self.send(Method::PUT, &format!("{path}/state"), Some(state), &[204])
                .await;
            if self.one_in(5) {
                self.send(Method::GET, &format!("{path}/versions"), None, &[200])
                    .await;
            }
            if self.one_in(30) {
                let unread = json!({ "read": false });
                self.send(Method::PUT, &format!("{path}/state"), Some(unread), &[204])
                    .await;
            }
        }

        if self.one_in(6) && !folders.is_empty() {
            let folder = folders[self.below(folders.len())];
            let list = self
                .json(&format!(
                    "/api/folders/{folder}/articles?unread=true&limit=50"
                ))
                .await;
            let newest = list
                .as_ref()
                .and_then(|list| list["articles"][0]["id"].as_i64());
            if let Some(newest) = newest
                && self.one_in(3)
            {
                let upto = json!({ "upto_id": newest });
                self.send(
                    Method::POST,
                    &format!("/api/folders/{folder}/read"),
                    Some(upto),
                    &[204],
                )
                .await;
            }
        }
        if self.one_in(8) {
            // All articles, and a page further down.
            let list = self.json("/api/articles?limit=50").await;
            let articles = list.as_ref().and_then(|list| list["articles"].as_array());
            let newest = articles
                .and_then(|articles| articles.first())
                .and_then(|article| article["id"].as_i64());
            let last = articles
                .and_then(|articles| articles.last())
                .and_then(|article| article["id"].as_i64());
            if let Some(last) = last {
                self.json(&format!("/api/articles?limit=50&before={last}"))
                    .await;
            }
            self.json("/api/articles?limit=50&after=0").await;
            self.json("/api/starred?limit=50").await;
            if let Some(newest) = newest
                && self.one_in(3)
            {
                // What is older than an hour, as far as the list went.
                let older = json!({ "upto_id": newest, "older_than": 3600 });
                self.send(Method::POST, "/api/read", Some(older), &[204])
                    .await;
            }
        }
        if self.one_in(12) && last_seq > 1 {
            let upto = json!({ "upto": last_seq / 2 });
            self.send(
                Method::POST,
                &format!("/api/feeds/{feed_id}/read"),
                Some(upto),
                &[204],
            )
            .await;
        }
        if self.one_in(15) {
            // A rule that hides some of it, looked at and taken back.
            let rule = json!({
                "scope": { "kind": "all" }, "field": "title", "match": "contains",
                "pattern": format!("Article {}", self.below(9) + 1), "action": "hide",
            });
            let made = self
                .send(Method::POST, "/api/rules", Some(rule), &[201])
                .await
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
            self.json("/api/rules/hits").await;
            self.json("/api/hidden?limit=50").await;
            if let Some(id) = made.and_then(|made| made["id"].as_i64()) {
                self.send(Method::DELETE, &format!("/api/rules/{id}"), None, &[200])
                    .await;
            }
        }
        if self.one_in(40) {
            self.logout().await;
            self.login().await;
        }
    }

    /// The unread counts of the tree are what the lists of unread articles
    /// hold, feed by feed and all together.
    async fn check_counts(&self) {
        let get = |path: String| async move {
            let answer = self.request(Method::GET, &path).send().await.unwrap();
            assert_eq!(answer.status(), StatusCode::OK, "{path}");
            serde_json::from_slice::<Value>(&answer.bytes().await.unwrap()).unwrap()
        };
        let count = |path: String| async move {
            let mut count = 0;
            let mut before = None;
            loop {
                let page = match before {
                    Some(id) => format!("{path}&before={id}"),
                    None => path.clone(),
                };
                let list = get(page).await;
                let articles = list["articles"].as_array().unwrap().clone();
                count += articles.len();
                match articles.last() {
                    Some(last) if articles.len() == 200 => before = last["id"].as_i64(),
                    _ => return count,
                }
            }
        };
        let tree = get("/api/tree".into()).await;
        let mut total = 0;
        for feed in tree["feeds"].as_array().unwrap() {
            let id = feed["id"].as_i64().unwrap();
            let unread = feed["unread"].as_u64().unwrap() as usize;
            let listed = count(format!("/api/feeds/{id}/articles?unread=true&limit=200")).await;
            assert_eq!(unread, listed, "{}: feed {id}", self.name);
            total += unread;
        }
        let all = count("/api/articles?unread=true&limit=200".into()).await;
        assert_eq!(all, total, "{}: all feeds", self.name);
    }
}
