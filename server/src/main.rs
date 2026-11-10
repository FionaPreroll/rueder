use std::{
    env,
    io::{self, IsTerminal},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use clap::{Parser, Subcommand};
use rueder::{
    AppState, articles, bench,
    cleaning::Rules,
    fetch,
    http_client::{self, HttpClient},
    images::{self, ImageProxy},
    import, reclean, router,
};
use sqlx::{
    Connection, PgConnection, PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tokio::{
    net::TcpListener,
    signal::unix::{SignalKind, signal},
};
use tracing_subscriber::EnvFilter;

/// rueder server. Configured through DATABASE_URL, LISTEN_ADDR and STATIC_DIR.
#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the web server (default).
    Serve,
    /// Fill an empty database with made-up feeds and articles and time the
    /// queries the reader lives on.
    #[command(subcommand)]
    Bench(BenchCommand),
    /// Clean the text of all articles again, from the versions as they were
    /// fetched. For after the rules of the cleaning or its lists have changed.
    Reclean {
        /// Only the articles of this feed.
        #[arg(long)]
        feed: Option<i32>,
    },
    /// Bring over the feeds, articles, accounts, folders and subscriptions of the
    /// old rueder. Adds what is not there yet, so it can run more than once.
    ImportLegacy {
        /// The old rueder's database, e.g. postgres://user:password@host/rueder.
        /// It is only read.
        #[arg(long, env = "LEGACY_DATABASE_URL", hide_env_values = true)]
        from: String,
    },
}

#[derive(Subcommand)]
enum BenchCommand {
    /// Make up accounts, feeds, articles and what the accounts have read.
    Generate {
        #[arg(long, default_value_t = 10)]
        accounts: i32,
        #[arg(long, default_value_t = 300)]
        feeds: i32,
        #[arg(long, default_value_t = 10_000_000)]
        articles: i64,
    },
    /// Time the queries and print a table.
    Measure {
        /// How often each query runs.
        #[arg(long, default_value_t = 500)]
        runs: usize,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rueder=info,tower_http=info".into()),
        )
        // No color codes in `docker compose logs` and other redirected output.
        .with_ansi(io::stdout().is_terminal())
        .init();
    let cli = Cli::parse();

    let database_url = env::var("DATABASE_URL").context("DATABASE_URL is not set")?;
    // Without JIT: PostgreSQL compiles a query to machine code once its estimated
    // cost is high, which takes longer than the queries of a reader ever run.
    let options = database_url
        .parse::<PgConnectOptions>()
        .context("reading DATABASE_URL")?
        .options([("jit", "off")]);
    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect_with(options)
        .await
        .context("connecting to PostgreSQL")?;
    sqlx::migrate!()
        .run(&db)
        .await
        .context("running migrations")?;

    match cli.command.unwrap_or(Command::Serve) {
        Command::Serve => serve(db).await,
        Command::Bench(BenchCommand::Generate {
            accounts,
            feeds,
            articles,
        }) => {
            let size = bench::Size {
                accounts,
                feeds,
                articles,
            };
            bench::generate(&db, size).await
        }
        Command::Reclean { feed } => {
            let report = reclean::run(&db, &Rules::from_env()?, feed).await?;
            println!("{report}");
            Ok(())
        }
        Command::ImportLegacy { from } => {
            let mut legacy = PgConnection::connect(&from)
                .await
                .context("connecting to the old database")?;
            let report = import::run(&db, &mut legacy, &Rules::from_env()?).await?;
            println!("{report}");
            Ok(())
        }
        Command::Bench(BenchCommand::Measure { runs }) => {
            let report = bench::measure(&db, runs).await?;
            println!("{report}");
            Ok(())
        }
    }
}

async fn serve(db: PgPool) -> anyhow::Result<()> {
    let listen_addr = env::var("LISTEN_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let static_dir =
        PathBuf::from(env::var("STATIC_DIR").unwrap_or_else(|_| "../web/build".into()));

    // New versions of articles need the partition of their month.
    articles::ensure_current_partitions(&db).await?;
    let partitions = db.clone();
    tokio::spawn(async move {
        let mut every = tokio::time::interval(Duration::from_secs(6 * 60 * 60));
        every.tick().await;
        loop {
            every.tick().await;
            if let Err(err) = articles::ensure_current_partitions(&partitions).await {
                tracing::error!("creating partitions: {err:#}");
            }
        }
    });

    // Feeds and images are fetched by the server itself, through the one
    // client that may reach out. FETCH_ALLOW_NETS opens internal networks for
    // it, e.g. "192.168.1.0/24, fd00::/8".
    let allow = env::var("FETCH_ALLOW_NETS").unwrap_or_default();
    let allow = allow
        .split(',')
        .map(str::trim)
        .filter(|net| !net.is_empty())
        .map(|net| {
            net.parse()
                .map_err(|err: String| anyhow::anyhow!("FETCH_ALLOW_NETS: {err}"))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let client = HttpClient::new(http_client::Config {
        allow,
        ..Default::default()
    })?;

    // FETCH=off keeps the server from fetching feeds.
    if env::var("FETCH").is_ok_and(|value| value == "off") {
        tracing::info!("not fetching feeds (FETCH=off)");
    } else {
        // CLEANING_RULES_DIR puts the operator's own lists in place of the ones
        // that come with rueder.
        let rules = Arc::new(Rules::from_env()?);
        tokio::spawn(fetch::run(db.clone(), client.clone(), rules));
    }

    // Images reach readers through the proxy. IMAGE_CACHE_DIR is where it keeps
    // them, IMAGE_CACHE_MB how much of them; IMAGE_PROXY_KEY is the secret for
    // its addresses, which it makes itself if there is none.
    let mut images =
        images::Config::new(env::var("IMAGE_CACHE_DIR").unwrap_or_else(|_| "image-cache".into()));
    if let Ok(megabytes) = env::var("IMAGE_CACHE_MB") {
        let megabytes: u64 = megabytes
            .parse()
            .context("IMAGE_CACHE_MB is not a number")?;
        images.cache_bytes = megabytes.saturating_mul(1024 * 1024);
    }
    images.key = env::var("IMAGE_PROXY_KEY")
        .ok()
        .filter(|key| !key.is_empty())
        .map(String::into_bytes);
    let images = Arc::new(ImageProxy::open(images, client)?);
    // Once an hour, what the cache holds too much of goes.
    let cache = images.clone();
    tokio::spawn(async move {
        let mut every = tokio::time::interval(Duration::from_secs(60 * 60));
        loop {
            every.tick().await;
            let cache = cache.clone();
            let _ = tokio::task::spawn_blocking(move || cache.cache().sweep()).await;
        }
    });

    let app = router(AppState::new(db).with_images(images), &static_dir);
    let listener = TcpListener::bind(&listen_addr).await?;
    tracing::info!("listening on http://{listen_addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown())
        .await?;
    Ok(())
}

/// Resolves on Ctrl+C or SIGTERM (what `docker stop` sends).
async fn shutdown() {
    let mut terminate = signal(SignalKind::terminate()).expect("installing the SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = terminate.recv() => {}
    }
}
