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
    import, moderation,
    monitor::{Monitor, Problems},
    oidc,
    oidc::Oidc,
    reclean, registration, reports, router, second_factor, sessions, transfer,
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
    /// Block a feed readers reported, or lift the block: a blocked feed is not
    /// fetched, nobody sees its articles or subscribes to it anew, and its
    /// images go from the cache.
    #[command(subcommand)]
    Feed(FeedCommand),
    /// Say whether readers can make accounts for themselves or invite others,
    /// and how many. Nothing is open until it is said here.
    #[command(subcommand)]
    Registration(RegistrationCommand),
    /// Bring over the feeds, articles, accounts, folders and subscriptions of the
    /// old rueder and merge them with the feeds and accounts that are here.
    /// Stop the server first and take a backup. Adds only what is not there yet,
    /// so a run that stopped halfway carries on when it is started again.
    ImportLegacy {
        /// The old rueder's database, e.g. postgres://user:password@host/rueder.
        /// It is only read.
        #[arg(long, env = "LEGACY_DATABASE_URL", hide_env_values = true)]
        from: String,
        /// An old user that goes into an account of another name, as
        /// <old name>=<new name>. May be given more than once. Old users without
        /// one go into the account of the same name, or get a new account.
        #[arg(long = "account", value_name = "OLD=NEW", value_parser = import::account_pair)]
        accounts: Vec<(String, String)>,
        /// Only say which feeds and accounts would be merged and which added,
        /// and write nothing.
        #[arg(long)]
        dry_run: bool,
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
    /// Delete an account and everything that is its own: sessions,
    /// subscriptions, what was read, stars, labels, rules, settings, reports
    /// of errors, invitations and identities at the OpenID Connect provider.
    /// Feeds stay for their other subscribers. This cannot be undone.
    Remove { name: String },
    /// Disable an account that is misused: it cannot log in, and its sessions
    /// end. What it has stays, for `account enable`.
    Disable { name: String },
    /// Let a disabled account log in again.
    Enable { name: String },
    /// Write what an account has made of rueder (settings, folders, feeds,
    /// what was read, stars, labels and rules) as JSON to the standard output,
    /// for moving it to another rueder with `account import`.
    Export { name: String },
    /// Take a file of `account export`, from this rueder or another one, into
    /// an account. Adds to what the account has and replaces its settings.
    Import { name: String, file: PathBuf },
    /// Say how many accounts this one may invite in all: a number, or
    /// `default` for what `rueder registration set --invites` gives everybody.
    Invites { name: String, quota: Quota },
    /// Switch off the second factor of an account and delete its recovery
    /// codes, for somebody who lost the app and the codes. The account logs in
    /// with its password alone after that, until it sets one up again.
    SecondFactorOff { name: String },
}

/// A limit on the command line: a number, or `none` for no limit.
#[derive(Clone, Copy)]
struct Limit(Option<i32>);

impl std::str::FromStr for Limit {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "none" => Ok(Self(None)),
            _ => match text.parse::<i32>() {
                Ok(number) if number >= 0 => Ok(Self(Some(number))),
                _ => Err("a number from 0 on, or `none`".into()),
            },
        }
    }
}

/// An account's quota of invitations: a number, or `default`.
#[derive(Clone, Copy)]
struct Quota(Option<i32>);

impl std::str::FromStr for Quota {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "default" => Ok(Self(None)),
            _ => match text.parse::<i32>() {
                Ok(number) if number >= 0 => Ok(Self(Some(number))),
                _ => Err("a number from 0 on, or `default`".into()),
            },
        }
    }
}

#[derive(Subcommand)]
enum RegistrationCommand {
    /// Show what is set and how many accounts there are.
    Show,
    /// Change what is set. What is not named stays as it is, and a change
    /// counts at once, without a restart.
    Set {
        /// Whether anybody can make an account without an invitation.
        #[arg(long)]
        open: Option<bool>,
        /// The most accounts there can be, not counting those admins invited;
        /// `none` for no limit. Registration and readers' invitations stop there.
        #[arg(long, value_name = "N|none")]
        max_accounts: Option<Limit>,
        /// The most accounts registration and readers' invitations make in 24
        /// hours; `none` for no limit.
        #[arg(long, value_name = "N|none")]
        per_day: Option<Limit>,
        /// How many accounts every reader may invite in all. `rueder account
        /// invites` gives one account a number of its own.
        #[arg(long, value_name = "N", value_parser = clap::value_parser!(i32).range(0..))]
        invites: Option<i32>,
        /// How many days an invitation link is good for.
        #[arg(long, value_name = "DAYS", value_parser = clap::value_parser!(i32).range(1..=3650))]
        invite_days: Option<i32>,
        /// The most accounts admins' invitations make in all; `none` for no
        /// limit. They count here and nowhere else.
        #[arg(long, value_name = "N|none")]
        admin_invite_accounts: Option<Limit>,
    },
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
enum FeedCommand {
    /// Block the feed at this address (as the status page shows it).
    Block { address: String },
    /// Lift the block of the feed at this address; it is fetched again at once.
    Unblock { address: String },
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
    /// Fetch made-up feeds from a source on this machine at the pace of
    /// FETCH_ROUND, FETCH_AT_ONCE and FETCH_EVERY, and say how many an hour
    /// that makes. Wants a database of its own.
    Fetch {
        #[arg(long, default_value_t = 5000)]
        feeds: u32,
        /// Under how many names the source answers; one fetch runs per name.
        #[arg(long, default_value_t = 250)]
        hosts: u32,
        #[arg(long, default_value_t = 120, value_parser = clap::value_parser!(u64).range(1..=3600))]
        seconds: u64,
        #[arg(long, default_value_t = 39_871)]
        port: u16,
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
        Command::Feed(command) => feed(&db, command).await,
        Command::Registration(command) => registering(&db, command).await,
        Command::Reclean { feed } => {
            let report = reclean::run(&db, &Rules::from_env()?, feed).await?;
            println!("{report}");
            Ok(())
        }
        Command::ImportLegacy {
            from,
            accounts,
            dry_run,
        } => {
            let mut legacy = PgConnection::connect(&from)
                .await
                .context("connecting to the old database")?;
            let options = import::Options { accounts, dry_run };
            let report = import::run(&db, &mut legacy, &Rules::from_env()?, &options).await?;
            println!("{report}");
            Ok(())
        }
        Command::Bench(BenchCommand::Fetch {
            feeds,
            hosts,
            seconds,
            port,
        }) => {
            let options = bench::fetching::Options {
                feeds,
                hosts,
                seconds,
                port,
                pace: fetch::Pace::from_env()?,
            };
            println!("{}", bench::fetching::run(&db, options).await?);
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
            let imported = match transfer::import(db, &Rules::from_env()?, id, file, None).await {
                Ok(imported) => imported,
                Err(rueder::AppError::BadRequest(reason)) => anyhow::bail!("{reason}"),
                Err(rueder::AppError::Internal(err)) => return Err(err),
                Err(_) => anyhow::bail!("the import failed"),
            };
            println!("{imported}");
        }
        AccountCommand::SecondFactorOff { name } => {
            if !second_factor::switch_off_named(db, &name).await? {
                anyhow::bail!("there is no account {}", name.trim());
            }
            println!(
                "Account {} has no second factor any more; it can set one up again.",
                name.trim()
            );
        }
        AccountCommand::Invites { name, quota } => {
            if !registration::set_quota(db, &name, quota.0).await? {
                anyhow::bail!("there is no account {}", name.trim());
            }
            match quota.0 {
                Some(quota) => println!("Account {} may invite {quota} in all.", name.trim()),
                None => println!("Account {} may invite as many as everybody.", name.trim()),
            }
        }
        AccountCommand::Remove { name } => {
            let id = accounts::id_of(db, &name)
                .await?
                .with_context(|| format!("there is no account {}", name.trim()))?;
            accounts::remove(db, id).await?;
            println!(
                "Account {} is deleted, with all that was its own.",
                name.trim()
            );
        }
        AccountCommand::Disable { name } => {
            if !accounts::set_disabled(db, &name, true).await? {
                anyhow::bail!("there is no account {}", name.trim());
            }
            println!(
                "Account {} is disabled. Its sessions are over.",
                name.trim()
            );
        }
        AccountCommand::Enable { name } => {
            if !accounts::set_disabled(db, &name, false).await? {
                anyhow::bail!("there is no account {}", name.trim());
            }
            println!("Account {} can log in again.", name.trim());
        }
        AccountCommand::List => {
            for account in accounts::list(db).await? {
                println!(
                    "{}\t{}\t{}{}",
                    account.username,
                    if account.is_admin { "admin" } else { "reader" },
                    if account.has_password {
                        "password"
                    } else {
                        "no password"
                    },
                    if account.disabled { "\tdisabled" } else { "" },
                );
            }
        }
    }
    Ok(())
}

async fn registering(db: &PgPool, command: RegistrationCommand) -> anyhow::Result<()> {
    let mut settings = registration::settings(db).await?;
    if let RegistrationCommand::Set {
        open,
        max_accounts,
        per_day,
        invites,
        invite_days,
        admin_invite_accounts,
    } = command
    {
        settings.open = open.unwrap_or(settings.open);
        if let Some(Limit(limit)) = max_accounts {
            settings.max_accounts = limit;
        }
        if let Some(Limit(limit)) = per_day {
            settings.per_day = limit;
        }
        settings.invites_per_account = invites.unwrap_or(settings.invites_per_account);
        settings.invite_days = invite_days.unwrap_or(settings.invite_days);
        if let Some(Limit(limit)) = admin_invite_accounts {
            settings.admin_invite_accounts = limit;
        }
        registration::set_settings(db, &settings).await?;
    }
    let counts = registration::counts(&mut *db.acquire().await?).await?;
    let limit = |limit: Option<i32>| limit.map_or("no limit".to_owned(), |n| n.to_string());
    let open = if settings.open { "open" } else { "closed" };
    println!("Registration without an invitation: {open}");
    println!(
        "Accounts, without those admins invited: {} of {}",
        counts.general,
        limit(settings.max_accounts)
    );
    println!(
        "New accounts in the last 24 hours, by registration and readers' invitations: {} of {}",
        counts.today,
        limit(settings.per_day)
    );
    println!(
        "Accounts a reader may invite in all: {}",
        settings.invites_per_account
    );
    println!("An invitation is good for: {} days", settings.invite_days);
    println!(
        "Accounts admins invited: {} of {}",
        counts.by_admins,
        limit(settings.admin_invite_accounts)
    );
    Ok(())
}

async fn feed(db: &PgPool, command: FeedCommand) -> anyhow::Result<()> {
    match command {
        FeedCommand::Block { address } => {
            let blocked = moderation::block(db, &address).await?;
            if blocked.is_empty() {
                anyhow::bail!("there is no feed at {}", address.trim());
            }
            // The images go from the cache this machine keeps, as the server
            // would find it: in the container, the same volume.
            let config = image_config()?;
            let cache = images::Cache::open(&config.cache_dir, config.cache_bytes)?;
            let mut files = 0;
            for feed_id in &blocked {
                for url in moderation::images_of(db, *feed_id).await? {
                    files += cache.forget(&url);
                }
            }
            println!(
                "Blocked {}: it is not fetched, nobody sees its articles or subscribes to it, \
                 and {files} cached images of it are gone.",
                address.trim()
            );
        }
        FeedCommand::Unblock { address } => {
            if moderation::unblock(db, &address).await? == 0 {
                anyhow::bail!("there is no blocked feed at {}", address.trim());
            }
            println!(
                "{} is not blocked any more and is fetched again.",
                address.trim()
            );
        }
    }
    Ok(())
}

/// Where the image proxy keeps its images and its key, from the environment:
/// IMAGE_CACHE_DIR, IMAGE_CACHE_MB and IMAGE_PROXY_KEY.
fn image_config() -> anyhow::Result<images::Config> {
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
    Ok(images)
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
    // FETCH_ROUND, FETCH_AT_ONCE and FETCH_EVERY set how fast feeds are
    // fetched; a value out of bounds stops the start.
    let pace = fetch::Pace::from_env()?;
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
            pace,
        ));
    }

    // Images reach readers through the proxy. IMAGE_CACHE_DIR is where it keeps
    // them, IMAGE_CACHE_MB how much of them; IMAGE_PROXY_KEY is the secret for
    // its addresses, which it makes itself if there is none.
    let images = Arc::new(ImageProxy::open(image_config()?, client.clone())?);
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
    // APP_API=off lets no app of another maker log in (see `greader`).
    if env::var("APP_API").is_ok_and(|value| value == "off") {
        tracing::info!("apps of other makers cannot log in (APP_API=off)");
        state = state.without_app_api();
    }
    // CONTENT_REPORTS=on lets readers report feeds and articles to the admins.
    if env::var("CONTENT_REPORTS").is_ok_and(|value| value == "on") {
        tracing::info!("readers may report feeds and articles (CONTENT_REPORTS=on)");
        state = state.with_content_reports();
    }
    // CLIENT_REPORTS=off takes no reports of errors from browsers, whoever
    // agreed to send them.
    if env::var("CLIENT_REPORTS").is_ok_and(|value| value == "off") {
        tracing::info!("taking no reports of errors from browsers (CLIENT_REPORTS=off)");
        state = state.without_reports();
    }
    // ADMIN_SECOND_FACTOR=required lets admins be admins only in a session
    // that began with a second factor. Anything else is a mistake, not "off".
    match env::var("ADMIN_SECOND_FACTOR").as_deref() {
        Err(_) | Ok("" | "off") => {}
        Ok("required") => {
            tracing::info!("admins need a second factor (ADMIN_SECOND_FACTOR=required)");
            state = state.with_admin_second_factor();
        }
        Ok(other) => anyhow::bail!("ADMIN_SECOND_FACTOR must be required or off, not {other:?}"),
    }
    // LEGAL_IMPRINT_URL, LEGAL_PRIVACY_URL, LEGAL_TERMS_URL and TERMS_VERSION:
    // the operator's legal texts, and the terms an account has to agree to.
    state = state.with_legal(rueder::legal::Legal::from_env()?);
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
