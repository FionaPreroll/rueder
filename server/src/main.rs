use std::{
    env,
    io::{self, IsTerminal},
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use clap::{Parser, Subcommand};
use rueder::{
    AppState, accounts, articles, bench,
    cleaning::Rules,
    events::Events,
    fetch, health,
    http_client::{self, HttpClient},
    images::{self, ImageProxy},
    import,
    monitor::{Monitor, Problems},
    oidc,
    oidc::Oidc,
    reclean, reports, router, sessions, transfer,
};
use sqlx::{
    Connection, PgConnection, PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tokio::{
    net::TcpListener,
    signal::unix::{SignalKind, signal},
};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Where the server listens if LISTEN_ADDR does not say.
const LISTEN_ADDR: &str = "127.0.0.1:3000";

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
    /// Ask the server that runs on LISTEN_ADDR whether it is well: the health
    /// check of the container. Fails if it does not answer.
    Health,
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
    /// Make accounts and set their passwords. There is no other way to get one.
    #[command(subcommand)]
    Account(AccountCommand),
    /// Say which accounts are admins: they see the status of the server and
    /// make accounts in the browser.
    #[command(subcommand)]
    Admin(AdminCommand),
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
enum AccountCommand {
    /// Make an account. Asks for its password, or reads it as one line when
    /// the input is not a terminal.
    Add {
        name: String,
        /// The account may manage the server.
        #[arg(long)]
        admin: bool,
    },
    /// Give an account a new password, which ends its sessions.
    Password { name: String },
    /// Let an account log in through the OpenID Connect provider of
    /// OIDC_ISSUER, as the one the provider calls by this subject.
    Link { name: String, subject: String },
    /// List the accounts.
    List,
    /// Write what an account has made of rueder (settings, folders, feeds,
    /// what was read, stars, labels and rules) as JSON to the standard output,
    /// for moving it to another rueder with `account import`.
    Export { name: String },
    /// Take a file of `account export`, from this rueder or another one, into
    /// an account. Adds to what the account has and replaces its settings.
    Import { name: String, file: PathBuf },
}

#[derive(Subcommand)]
enum AdminCommand {
    /// Make an account an admin.
    Add { name: String },
    /// Make an admin a reader again. Their sessions stay.
    Remove { name: String },
    /// List the admins.
    List,
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
    // The warnings and errors of the log are kept for the status page, too.
    let problems = Arc::new(Problems::default());
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rueder=info,tower_http=info".into()),
        )
        // No color codes in `docker compose logs` and other redirected output.
        .with(tracing_subscriber::fmt::layer().with_ansi(io::stdout().is_terminal()))
        .with(problems.layer())
        .init();
    let cli = Cli::parse();

    // Asks the server that is running, so it opens no database of its own.
    if matches!(cli.command, Some(Command::Health)) {
        let listen = env::var("LISTEN_ADDR").unwrap_or_else(|_| LISTEN_ADDR.into());
        return health::check(health::address(&listen).await?, health::WAIT).await;
    }

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
        Command::Serve => serve(db, problems).await,
        Command::Health => unreachable!("answered before the database was opened"),
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
        Command::Account(command) => account(&db, command).await,
        Command::Admin(command) => admin(&db, command).await,
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

async fn account(db: &PgPool, command: AccountCommand) -> anyhow::Result<()> {
    match command {
        AccountCommand::Add { name, admin } => {
            accounts::check_name(&name)?;
            let password = ask_password()?;
            accounts::create(db, &name, Some(&password), admin).await??;
            println!("Account {} is there.", name.trim());
        }
        AccountCommand::Password { name } => {
            let password = ask_password()?;
            if !accounts::set_password(db, &name, &password).await?? {
                anyhow::bail!("there is no account {}", name.trim());
            }
            println!(
                "Account {} has a new password. Its sessions are over.",
                name.trim()
            );
        }
        AccountCommand::Link { name, subject } => {
            let config = oidc::Config::from_env()?.context("OIDC_ISSUER is not set")?;
            if !oidc::link(db, &name, &config.issuer, &subject).await? {
                anyhow::bail!(
                    "there is no account {}, or this identity has an account already",
                    name.trim()
                );
            }
            println!(
                "Account {} can log in through {}.",
                name.trim(),
                config.issuer
            );
        }
        AccountCommand::Export { name } => {
            let id = accounts::id_of(db, &name)
                .await?
                .with_context(|| format!("there is no account {}", name.trim()))?;
            let file = transfer::export(db, id).await?;
            serde_json::to_writer_pretty(io::stdout().lock(), &file)?;
            println!();
        }
        AccountCommand::Import { name, file } => {
            let id = accounts::id_of(db, &name)
                .await?
                .with_context(|| format!("there is no account {}", name.trim()))?;
            let text =
                std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
            let file: transfer::Export =
                serde_json::from_slice(&text).context("this is not a file of a rueder account")?;
            let imported = match transfer::import(db, &Rules::from_env()?, id, file).await {
                Ok(imported) => imported,
                Err(rueder::AppError::BadRequest(reason)) => anyhow::bail!("{reason}"),
                Err(rueder::AppError::Internal(err)) => return Err(err),
                Err(_) => anyhow::bail!("the import failed"),
            };
            println!("{imported}");
        }
        AccountCommand::List => {
            for account in accounts::list(db).await? {
                println!(
                    "{}\t{}\t{}",
                    account.username,
                    if account.is_admin { "admin" } else { "reader" },
                    if account.has_password {
                        "password"
                    } else {
                        "no password"
                    },
                );
            }
        }
    }
    Ok(())
}

async fn admin(db: &PgPool, command: AdminCommand) -> anyhow::Result<()> {
    match command {
        AdminCommand::Add { name } => {
            if !accounts::set_admin(db, &name, true).await? {
                anyhow::bail!("there is no account {}", name.trim());
            }
            println!("Account {} is an admin.", name.trim());
        }
        AdminCommand::Remove { name } => {
            if !accounts::set_admin(db, &name, false).await? {
                anyhow::bail!("there is no account {}", name.trim());
            }
            println!("Account {} is no admin any more.", name.trim());
            if !accounts::list(db)
                .await?
                .iter()
                .any(|account| account.is_admin)
            {
                println!("There is no admin left; `rueder admin add` makes one.");
            }
        }
        AdminCommand::List => {
            for account in accounts::list(db).await? {
                if account.is_admin {
                    println!("{}", account.username);
                }
            }
        }
    }
    Ok(())
}

/// A password from the terminal, asked for twice and not shown, or the first
/// line of the input when a script pipes it in.
fn ask_password() -> anyhow::Result<String> {
    if io::stdin().is_terminal() {
        let password = rpassword::prompt_password("Password: ")?;
        accounts::check_password(&password)?;
        if rpassword::prompt_password("Once more: ")? != password {
            anyhow::bail!("the two passwords differ");
        }
        return Ok(password);
    }
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let password = line.trim_end_matches(['\r', '\n']).to_owned();
    accounts::check_password(&password)?;
    Ok(password)
}

async fn serve(db: PgPool, problems: Arc<Problems>) -> anyhow::Result<()> {
    let monitor = Arc::new(Monitor::new(problems));
    let listen_addr = env::var("LISTEN_ADDR").unwrap_or_else(|_| LISTEN_ADDR.into());
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

    // CLEANING_RULES_DIR puts the operator's own lists in place of the ones
    // that come with rueder.
    let rules = Arc::new(Rules::from_env()?);
    // What fetching finds and what readers change reaches open tabs at once.
    let events = Events::new();
    // FETCH=off keeps the server from fetching feeds.
    if env::var("FETCH").is_ok_and(|value| value == "off") {
        tracing::info!("not fetching feeds (FETCH=off)");
    } else {
        tokio::spawn(fetch::run(
            db.clone(),
            client.clone(),
            rules.clone(),
            events.clone(),
            monitor.clone(),
        ));
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
    let images = Arc::new(ImageProxy::open(images, client.clone())?);
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

    // Once an hour, the sessions that have run out go, and the reports of
    // errors in browsers that are older than 30 days.
    let sessions = db.clone();
    tokio::spawn(async move {
        let mut every = tokio::time::interval(Duration::from_secs(60 * 60));
        loop {
            every.tick().await;
            if let Err(err) = sessions::sweep(&sessions).await {
                tracing::error!("removing old sessions: {err:#}");
            }
            if let Err(err) = reports::sweep(&sessions).await {
                tracing::error!("removing old reports of errors: {err:#}");
            }
        }
    });

    let mut state = AppState::new(db)
        .with_images(images)
        .with_rules(rules)
        .with_events(events)
        .with_monitor(monitor)
        .with_client(client.clone());
    // OIDC_ISSUER, OIDC_CLIENT_ID, OIDC_CLIENT_SECRET and OIDC_REDIRECT_URL add
    // the login through an OpenID Connect provider.
    if let Some(config) = oidc::Config::from_env()? {
        tracing::info!("login through {} is on", config.issuer);
        state = state.with_oidc(Arc::new(Oidc::new(config, client)?));
    }
    // Behind a reverse proxy every connection comes from the proxy.
    // CLIENT_IP_HEADER names the header it puts the sender's address in, e.g.
    // X-Forwarded-For. Only set it if a proxy really sets that header.
    if let Ok(name) = env::var("CLIENT_IP_HEADER")
        && !name.is_empty()
    {
        let name = name
            .parse()
            .context("CLIENT_IP_HEADER is not the name of a header")?;
        state = state.with_client_ip_header(name);
    }
    // CLIENT_REPORTS=off takes no reports of errors from browsers, whoever
    // agreed to send them.
    if env::var("CLIENT_REPORTS").is_ok_and(|value| value == "off") {
        tracing::info!("taking no reports of errors from browsers (CLIENT_REPORTS=off)");
        state = state.without_reports();
    }
    let app = router(state, &static_dir);
    let listener = TcpListener::bind(&listen_addr).await?;
    tracing::info!("listening on http://{listen_addr}");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
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
