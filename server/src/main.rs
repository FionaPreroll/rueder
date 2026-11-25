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
    email,
    events::Events,
    fetch, health,
    http_client::{self, HttpClient},
    images::{self, ImageProxy},
    import, inactive, mail, moderation,
    monitor::{Monitor, Problems},
    oidc,
    oidc::Oidc,
    progress,
    rate::Rates,
    reclean, registration, reports, retention, router, second_factor, sessions,
    settings::{self, Settings},
    sizes::Sizes,
    transfer, tree, version,
};
use sqlx::{
    Connection, PgConnection, PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tokio::{
    net::TcpListener,
    signal::unix::{SignalKind, signal},
};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// rueder server. Configured through DATABASE_URL, LISTEN_ADDR and STATIC_DIR.
#[derive(Parser)]
// Not only the version: which commit it was built from and when, so that a
// report about a server says which rueder it is about.
#[command(version = version::LINE)]
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
    /// Say which accounts are admins: they see the status of the server, make
    /// accounts and say as what the feeds they read are fetched, in the
    /// browser.
    #[command(subcommand)]
    Admin(AdminCommand),
    /// Show what the server knows of a feed, say as what it is fetched, or
    /// block a feed readers reported: a blocked feed is not fetched, nobody
    /// sees its articles or subscribes to it anew, and its images go from the
    /// cache.
    #[command(subcommand)]
    Feed(FeedCommand),
    /// Show what the server knows of an article, and its versions as the feed
    /// sent them.
    #[command(subcommand)]
    Article(ArticleCommand),
    /// Say whether readers can make accounts for themselves or invite others,
    /// and how many. Nothing is open until it is said here.
    #[command(subcommand)]
    Registration(RegistrationCommand),
    /// Say how long the server keeps old articles, versions and feeds without
    /// subscribers, which stay until it is said here.
    #[command(subcommand)]
    Retention(RetentionCommand),
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
    /// Disable an account that is misused or no longer used: it cannot log
    /// in, its sessions end, and the feeds no other enabled account reads are
    /// not fetched. What it has stays, for `account enable`.
    Disable { name: String },
    /// Let a disabled account log in again, and fetch its feeds again.
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
    /// Switch off the second factor of an account and delete its passkeys and
    /// recovery codes, for somebody who lost the app, the passkeys and the
    /// codes. The account logs in with its password alone after that, until it
    /// sets one up again.
    SecondFactorOff { name: String },
}

/// A number on the command line from `least` on, or the word that stands for
/// none of it.
fn number_or(text: &str, word: &str, least: i32) -> Result<Option<i32>, String> {
    if text == word {
        return Ok(None);
    }
    match text.parse::<i32>() {
        Ok(number) if number >= least => Ok(Some(number)),
        _ => Err(format!("a number from {least} on, or `{word}`")),
    }
}

/// A limit on the command line: a number, or `none` for no limit.
#[derive(Clone, Copy)]
struct Limit(Option<i32>);

impl std::str::FromStr for Limit {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        number_or(text, "none", 0).map(Self)
    }
}

/// An account's quota of invitations: a number, or `default`.
#[derive(Clone, Copy)]
struct Quota(Option<i32>);

impl std::str::FromStr for Quota {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        number_or(text, "default", 0).map(Self)
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
        /// Whether registration asks for an address: `off`, `optional`, or
        /// `required`, where a new account logs in only once its address is
        /// confirmed and one from before is asked for it. Leaving `required`
        /// lets in the accounts that still wait. Not `off` needs SMTP_URL,
        /// MAIL_FROM and PUBLIC_URL.
        #[arg(long, value_name = "off|optional|required")]
        email: Option<email::Mode>,
        /// Whether registration without an invitation asks for a solved
        /// captcha, a moment of computing for the browser. On unless turned
        /// off.
        #[arg(long)]
        captcha: Option<bool>,
    },
}

#[derive(Subcommand)]
enum RetentionCommand {
    /// Show what is set and what the retention of articles looks after.
    Show,
    /// Change what is set. What is not named stays as it is, and a change
    /// counts at once, without a restart: what is older goes within the hour.
    Set {
        /// After how many months since rueder first fetched them articles go;
        /// `never` keeps them. Those a reader starred or labelled stay.
        #[arg(long, value_name = "MONTHS|never")]
        articles: Option<Keep>,
        /// What goes of an old article, which has to be said: `article` takes
        /// it from the lists, the unread counts and the search; `text` takes
        /// only its text and leaves its line in the lists.
        #[arg(long, value_name = "article|text")]
        remove: Option<retention::Remove>,
        /// After how many months versions go, by whole months; `never` keeps
        /// them.
        #[arg(long, value_name = "MONTHS|never")]
        versions: Option<Keep>,
        /// After how many days without subscribers a feed goes, with its
        /// articles; `never` keeps it. A blocked feed keeps its block.
        #[arg(long, value_name = "DAYS|never")]
        orphans: Option<Keep>,
        /// After how many months nobody used it an account goes, with all it
        /// has, after two mails to its address; `never` keeps it. Accounts
        /// without a confirmed address, admins and disabled ones stay.
        #[arg(long, value_name = "MONTHS|never")]
        inactive: Option<Keep>,
    },
    /// Clear now all that is old of the articles, instead of waiting for the
    /// server's hourly runs. Open tabs see it when they next load.
    Run,
}

/// How long something is kept: a number, or `never` for no end.
#[derive(Clone, Copy)]
struct Keep(Option<i32>);

impl std::str::FromStr for Keep {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        number_or(text, "never", 1).map(Self)
    }
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
    /// Show what a reader's "Feed info" shows and how many read the feed, by
    /// its number or its address.
    Show { feed: String },
    /// Say as what the feed with this number or at this address is fetched:
    /// as rueder, or as a browser for sites that turn rueder away. It holds
    /// for all who read it, and the feed is fetched at once.
    Agent {
        feed: String,
        #[arg(value_parser = ["rueder", "browser"])]
        agent: String,
    },
    /// Say how many seconds at least lie between two fetches of the feed with
    /// this number or at this address, for sites that find it asked too
    /// often, or `none`. It holds for all who read it.
    MinInterval { feed: String, seconds: String },
}

#[derive(Subcommand)]
enum ArticleCommand {
    /// Show what a reader's "Article info" shows, for the article with this
    /// number in the feed with this number.
    Show {
        feed: i32,
        seq: i32,
        /// Also print every version that is kept, as the feed sent it.
        #[arg(long)]
        versions: bool,
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
    /// Add many made-up readers to a database of made-up data, with
    /// subscriptions by popularity and a session each, for the load tests in
    /// web/load/. The sessions' tokens go to a file.
    Readers {
        #[arg(long, default_value_t = 10_000)]
        count: i32,
        /// Where the tokens go, one `account_id token` a line.
        #[arg(long, value_name = "FILE")]
        tokens: std::path::PathBuf,
        /// A feed every new reader subscribes to as well, for the load test
        /// of the live updates.
        #[arg(long, value_name = "URL")]
        shared: Option<String>,
    },
    /// Time the limits in memory with few keys and with a full table, and
    /// print a table. Needs no database.
    Limits,
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
        /// Feeds of one more name, which answers half a second late.
        #[arg(long, default_value_t = 0)]
        slow: u32,
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
        let listen = env::var("LISTEN_ADDR").unwrap_or_else(|_| settings::LISTEN_ADDR.into());
        return health::check(health::address(&listen).await?, health::WAIT).await;
    }
    if matches!(cli.command, Some(Command::Bench(BenchCommand::Limits))) {
        println!("{}", bench::limits::run()?);
        return Ok(());
    }

    let database_url = env::var("DATABASE_URL").context("DATABASE_URL is not set")?;
    // Without JIT: PostgreSQL compiles a query to machine code once its estimated
    // cost is high, which takes longer than the queries of a reader ever run.
    let options = database_url
        .parse::<PgConnectOptions>()
        .context("reading DATABASE_URL")?
        .options([("jit", "off")]);
    // DATABASE_CONNECTIONS, HEAVY_AT_ONCE and MAILS_PER_DAY size the server for
    // a large instance; a value out of bounds stops the start.
    let sizes = Sizes::from_env()?;
    let db = PgPoolOptions::new()
        .max_connections(sizes.connections)
        .connect_with(options)
        .await
        .context("connecting to PostgreSQL")?;
    progress::migrate(&db).await?;

    match cli.command.unwrap_or(Command::Serve) {
        Command::Serve => serve(db, problems, sizes).await,
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
        Command::Article(command) => article(&db, command).await,
        Command::Registration(command) => registering(&db, command).await,
        Command::Retention(command) => retaining(&db, command).await,
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
            slow,
            seconds,
            port,
        }) => {
            let options = bench::fetching::Options {
                feeds,
                hosts,
                slow,
                seconds,
                port,
                pace: fetch::Pace::from_env()?,
            };
            println!("{}", bench::fetching::run(&db, options).await?);
            Ok(())
        }
        Command::Bench(BenchCommand::Readers {
            count,
            tokens,
            shared,
        }) => {
            let added = bench::readers::add(&db, count, &tokens, shared.as_deref()).await?;
            println!("{added}");
            Ok(())
        }
        Command::Bench(BenchCommand::Limits) => {
            unreachable!("answered before the database was opened")
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
            let imported =
                match transfer::import(db, &Rules::from_env()?, id, file, true, None).await {
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
            let alone = accounts::set_disabled(db, &name, true)
                .await?
                .with_context(|| format!("there is no account {}", name.trim()))?;
            println!(
                "Account {} is disabled. Its sessions are over, and the {alone} feeds \
                 nobody else reads are not fetched any more.",
                name.trim()
            );
        }
        AccountCommand::Enable { name } => {
            let alone = accounts::set_disabled(db, &name, false)
                .await?
                .with_context(|| format!("there is no account {}", name.trim()))?;
            println!(
                "Account {} can log in again, and the {alone} feeds nobody else reads \
                 are fetched.",
                name.trim()
            );
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
        email,
        captcha,
    } = command
    {
        if let Some(mode) = email {
            if mode != email::Mode::Off && mail::Mailer::from_env()?.is_none() {
                anyhow::bail!("addresses need mail: set SMTP_URL, MAIL_FROM and PUBLIC_URL first");
            }
            settings.email = mode;
        }
        settings.open = open.unwrap_or(settings.open);
        settings.captcha = captcha.unwrap_or(settings.captcha);
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
        // The captcha costs scripts little: only a limit per day holds how many
        // accounts anybody can make.
        if settings.open && settings.per_day.is_none() {
            anyhow::bail!("open registration needs a limit per day: add --per-day <number>");
        }
        registration::set_settings(db, &settings).await?;
    }
    let counts = registration::counts(&mut *db.acquire().await?).await?;
    let limit = |limit: Option<i32>| limit.map_or("no limit".to_owned(), |n| n.to_string());
    let open = if settings.open { "open" } else { "closed" };
    println!("Registration without an invitation: {open}");
    let captcha = if settings.captcha {
        "asked for"
    } else {
        "not asked for"
    };
    println!("A captcha, without an invitation: {captcha}");
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
    let email = match settings.email {
        email::Mode::Off => "not asked for",
        email::Mode::Optional => "asked for, and optional",
        email::Mode::Required => "required: a new account logs in once it is confirmed",
    };
    println!("An address: {email}");
    Ok(())
}

async fn retaining(db: &PgPool, command: RetentionCommand) -> anyhow::Result<()> {
    let mut settings = retention::articles(db).await?;
    match command {
        RetentionCommand::Show => {}
        RetentionCommand::Set {
            articles,
            remove,
            versions,
            orphans,
            inactive,
        } => {
            if let Some(Keep(months)) = articles {
                settings.articles_months = months;
                if months.is_none() {
                    settings.articles_what = None;
                }
            }
            if remove.is_some() {
                settings.articles_what = remove;
            }
            if let Some(Keep(months)) = versions {
                settings.versions_months = months;
            }
            if let Some(Keep(days)) = orphans {
                settings.orphans_days = days;
            }
            retention::set_articles(db, &settings).await?;
            if let Some(Keep(months)) = inactive {
                inactive::set_months(db, months).await?;
                let mode = email::mode(&mut *db.acquire().await?).await?;
                if months.is_some() && mode == email::Mode::Off {
                    println!(
                        "Note: accounts have no address while `rueder registration set --email` is off, so none goes."
                    );
                }
            }
        }
        RetentionCommand::Run => {
            let swept = retention::sweep(db, None, OffsetDateTime::now_utc(), None).await?;
            println!("Articles that went: {}", swept.articles);
            println!("Articles whose text went: {}", swept.texts);
            let months = if swept.months.is_empty() {
                "none".to_owned()
            } else {
                swept.months.join(", ")
            };
            println!("Months of versions that went: {months}");
            println!(
                "Feeds without subscribers that went: {}, with {} articles",
                swept.feeds, swept.feed_articles
            );
            if swept.more {
                println!(
                    "Some is left: a month of versions had to wait for the table. Run it again."
                );
            }
            println!();
        }
    }
    let after = |value: Option<i32>, unit: &str| {
        value.map_or("never".to_owned(), |value| format!("after {value} {unit}"))
    };
    let what = match settings.articles_what {
        Some(retention::Remove::Article) => " (whole)",
        Some(retention::Remove::Text) => " (only the text)",
        None => "",
    };
    println!(
        "Articles go: {}{what}",
        after(settings.articles_months, "months since they arrived")
    );
    println!("Versions go: {}", after(settings.versions_months, "months"));
    println!(
        "Feeds without subscribers go: {}",
        after(settings.orphans_days, "days")
    );
    println!("Starred and labelled articles stay.");
    let inactive_months = inactive::months(db).await?;
    println!(
        "Accounts nobody uses go: {}",
        inactive_months.map_or("never".to_owned(), |months| format!(
            "after {months} months, after a mail {} and {} days before, by the running server",
            inactive::FIRST_DAYS,
            inactive::SECOND_DAYS
        ))
    );
    let kept = retention::kept(db).await?;
    println!();
    println!(
        "Ids of articles that went, kept so they do not come back: {}",
        kept.tombstones
    );
    println!("Articles whose text went: {}", kept.dropped_texts);
    println!("Feeds without subscribers: {}", kept.orphans);
    let months = match (kept.months.first(), kept.months.last()) {
        (Some(first), Some(last)) => format!("{first} to {last}"),
        _ => "none".to_owned(),
    };
    println!("Months of versions: {months}");
    if let Some(months) = inactive_months {
        let standing = inactive::standing(db, months, OffsetDateTime::now_utc()).await?;
        println!("Accounts that had the first mail: {}", standing.warned);
        println!(
            "Accounts nobody used for {months} months that stay, without a confirmed address: {}",
            standing.without_address
        );
    }
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
            let config = Settings::images_from_env()?;
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
        FeedCommand::Show { feed } => {
            let found = moderation::named(db, &feed).await?;
            if found.is_empty() {
                anyhow::bail!("there is no feed {}", feed.trim());
            }
            for feed_id in found {
                let Some(info) = tree::feed_info(db, feed_id, None).await? else {
                    continue;
                };
                let readers = moderation::readers(db, feed_id).await?;
                let next = if info.blocked {
                    "blocked".to_owned()
                } else {
                    when(Some(info.next_fetch_at))
                };
                println!("Feed {}", info.id);
                println!("  Title:        {}", info.title);
                println!("  Address:      {}", info.feed_url);
                println!(
                    "  Site:         {}",
                    info.site_url.as_deref().unwrap_or("-")
                );
                println!("  Fetched as:   {}", info.fetch_agent);
                println!("  Every:        {} s", info.fetch_interval_s);
                match info.min_interval_s {
                    Some(seconds) => println!("  At least:     {seconds} s"),
                    None => println!("  At least:     -"),
                }
                println!("  Last fetch:   {}", when(info.fetched_at));
                println!("  Last success: {}", when(info.last_success_at));
                println!("  Last error:   {}", when(info.last_error_at));
                println!("  Error:        {}", info.error.as_deref().unwrap_or("-"));
                println!("  Next fetch:   {next}");
                println!("  Articles:     {}", info.last_seq);
                println!("  Readers:      {readers}");
            }
        }
        FeedCommand::Agent { feed, agent } => {
            let found = moderation::named(db, &feed).await?;
            if found.is_empty() {
                anyhow::bail!("there is no feed {}", feed.trim());
            }
            moderation::set_agent(db, &found, &agent).await?;
            let numbers: Vec<String> = found.iter().map(i32::to_string).collect();
            println!(
                "Feed {} is fetched as {agent} from now on, the first time as soon as it may.",
                numbers.join(", ")
            );
        }
        FeedCommand::MinInterval { feed, seconds } => {
            let range = fetch::MIN_INTERVAL..=fetch::MAX_INTERVAL;
            let seconds = match seconds.trim() {
                "none" => None,
                given => match given.parse::<i32>() {
                    Ok(seconds) if range.contains(&seconds) => Some(seconds),
                    _ => anyhow::bail!(
                        "the seconds are `none` or between {} and {}",
                        range.start(),
                        range.end()
                    ),
                },
            };
            let found = moderation::named(db, &feed).await?;
            if found.is_empty() {
                anyhow::bail!("there is no feed {}", feed.trim());
            }
            moderation::set_min_interval(db, &found, seconds).await?;
            let numbers: Vec<String> = found.iter().map(i32::to_string).collect();
            match seconds {
                Some(seconds) => println!(
                    "Feed {} is fetched at most every {seconds} s from now on.",
                    numbers.join(", ")
                ),
                None => println!(
                    "Feed {} is fetched as often as it publishes from now on.",
                    numbers.join(", ")
                ),
            }
        }
    }
    Ok(())
}

async fn article(db: &PgPool, command: ArticleCommand) -> anyhow::Result<()> {
    let ArticleCommand::Show {
        feed,
        seq,
        versions,
    } = command;
    let info = articles::info(db, None, feed, seq)
        .await?
        .with_context(|| format!("there is no article {seq} in feed {feed}"))?;
    let what = match info.updated_what {
        Some(articles::Changed::Title) => " (title)",
        Some(articles::Changed::Text) => " (text)",
        Some(articles::Changed::Both) => " (title and text)",
        None => "",
    };
    let changed = match info.updated_at {
        Some(at) => format!("{}{what}", when(Some(at))),
        None => "never".to_owned(),
    };
    println!(
        "Article {} of feed {} ({})",
        info.seq, info.feed_id, info.feed_title
    );
    println!("  Title:         {}", info.title);
    println!("  Id:            {}", info.guid);
    println!("  Link:          {}", info.url.as_deref().unwrap_or("-"));
    println!("  Published:     {}", when(Some(info.published_at)));
    println!("  First fetched: {}", when(Some(info.first_fetched_at)));
    println!("  Changed:       {changed}");
    println!(
        "  Version:       {}, {} kept",
        info.version, info.versions_kept
    );
    if info.text_dropped {
        println!("  Text:          dropped by the retention");
    }
    if versions {
        for version in articles::versions(db, None, feed, seq).await? {
            let original = version.original;
            println!();
            println!(
                "Version {}, fetched {}: {}",
                version.version,
                when(Some(version.fetched_at)),
                original.title
            );
            println!("{}", original.content);
        }
    }
    Ok(())
}

/// A time for the command line, to the second; "never" for none.
fn when(at: Option<OffsetDateTime>) -> String {
    at.and_then(|at| at.replace_nanosecond(0).ok())
        .and_then(|at| at.format(&Rfc3339).ok())
        .unwrap_or_else(|| "never".to_owned())
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

async fn serve(db: PgPool, problems: Arc<Problems>, sizes: Sizes) -> anyhow::Result<()> {
    let settings = Settings::from_env()?;
    settings.log();
    let monitor = Arc::new(Monitor::new(problems));

    // The command line asks for a limit per day when it opens registration;
    // one opened before it did is said here until it has one.
    let registering = registration::settings(&db).await?;
    if registering.open && registering.per_day.is_none() {
        tracing::warn!(
            "registration is open without a limit per day: set one with `rueder registration set --per-day <number>`"
        );
    }

    // New versions of articles need the partition of their month.
    articles::ensure_current_partitions(&db).await?;
    periodically(6 * HOUR, false, db.clone(), |db| async move {
        if let Err(err) = articles::ensure_current_partitions(&db).await {
            tracing::error!("creating partitions: {err:#}");
        }
    });

    // Feeds and images are fetched by the server itself, through the one
    // client that may reach out.
    let client = HttpClient::new(http_client::Config {
        agent: http_client::AGENT.to_owned(),
        allow: settings.allow,
        ..Default::default()
    })?;
    // RSSIFIER_URL and RSSIFIER_TOKEN: where RSSifier answers and rueder's
    // token there, for web pages without a feed. Without them rueder finds
    // and fetches none of those.
    let rssifier = rueder::rssifier::Rssifier::from_env()?.map(Arc::new);
    if rssifier.is_some() {
        tracing::info!("web pages without a feed come through RSSifier");
    }
    // CLEANING_RULES_DIR puts the operator's own lists in place of the ones
    // that come with rueder.
    let rules = Arc::new(Rules::from_env()?);
    // What fetching finds and what readers change reaches open tabs at once.
    let events = Events::new();
    // FETCH_ROUND, FETCH_AT_ONCE and FETCH_EVERY set how fast feeds are
    // fetched; a value out of bounds stops the start.
    let pace = fetch::Pace::from_env()?;
    if settings.fetch {
        let fetcher = fetch::Fetcher {
            db: db.clone(),
            client: client.clone(),
            rssifier: rssifier.clone(),
            rules: rules.clone(),
            events: events.clone(),
            monitor: monitor.clone(),
        };
        tokio::spawn(fetcher.run(pace));
    }

    // Images reach readers through the proxy, which fetches nothing that
    // belongs to a blocked feed. What its cache holds too much of goes once an
    // hour.
    let images =
        Arc::new(ImageProxy::open(settings.images, client.clone())?.with_blocked_from(db.clone()));
    periodically(HOUR, true, images.clone(), |images| async move {
        let _ = tokio::task::spawn_blocking(move || images.cache().sweep()).await;
    });
    // Once an hour, the sessions that have run out go, the links nobody
    // used, and the reports of errors in browsers that are old.
    periodically(HOUR, true, db.clone(), |db| async move {
        if let Err(err) = sessions::sweep(&db).await {
            tracing::error!("removing old sessions: {err:#}");
        }
        if let Err(err) = rueder::email::sweep(&db).await {
            tracing::error!("removing old links and unconfirmed accounts: {err:#}");
        }
        if let Err(err) = rueder::recovery::sweep(&db).await {
            tracing::error!("removing old links to set a password: {err:#}");
        }
        if let Err(err) = reports::sweep(&db).await {
            tracing::error!("removing old reports of errors: {err:#}");
        }
    });
    // Once an hour, what the retention says is old goes.
    periodically(HOUR, true, (db.clone(), events.clone()), |(db, events)| {
        retention::hourly(db, events)
    });

    let mut state = AppState {
        images: Some(images),
        rules: Some(rules),
        events,
        monitor,
        client: Some(client.clone()),
        rssifier,
        rates: Arc::new(Rates::new(sizes.limits())),
        client_ip_header: settings.client_ip_header,
        switches: settings.switches,
        database_warn_bytes: settings.database_warn_bytes,
        ..AppState::new(db)
    };
    if sizes != Sizes::default() {
        tracing::info!(
            connections = sizes.connections,
            heavy_at_once = sizes.heavy_at_once,
            mails_per_day = sizes.mails_per_day,
            "sized for a large instance"
        );
    }
    // OIDC_ISSUER, OIDC_CLIENT_ID, OIDC_CLIENT_SECRET and OIDC_REDIRECT_URL add
    // the login through an OpenID Connect provider.
    if let Some(config) = oidc::Config::from_env()? {
        tracing::info!("login through {} is on", config.issuer);
        state.oidc = Some(Arc::new(Oidc::new(config, client)?));
    }
    // PUBLIC_URL and PASSKEY_ANDROID_APPS: passkeys, for the site and its apps.
    state.passkeys = rueder::passkeys::Passkeys::from_env()?.map(Arc::new);
    // SMTP_URL, MAIL_FROM and PUBLIC_URL: the relay mail goes through (see
    // `mail`). Where registration asks for addresses there has to be one:
    // otherwise nobody who registers could ever confirm theirs.
    match mail::Mailer::from_env()? {
        Some(mailer) => {
            tracing::info!("mail goes through the relay in SMTP_URL");
            let mailer = Arc::new(mailer);
            state.mail = Some(mailer.clone());
            // Once an hour, the accounts nobody uses get their mails, and go
            // when their time is over, where the retention says so (E13).
            let looked_after = (state.db.clone(), mailer, state.rates.clone());
            periodically(HOUR, true, looked_after, |(db, mailer, rates)| {
                inactive::hourly(db, mailer, rates)
            });
        }
        None => {
            let mode = rueder::email::mode(&mut *state.db.acquire().await?).await?;
            if mode != rueder::email::Mode::Off {
                anyhow::bail!(
                    "registration asks for addresses (--email {mode}), but SMTP_URL is not set"
                );
            }
        }
    }
    // LEGAL_IMPRINT_URL, LEGAL_PRIVACY_URL, LEGAL_TERMS_URL and TERMS_VERSION:
    // the operator's legal texts, and the terms an account has to agree to.
    state.legal = Arc::new(rueder::legal::Legal::from_env()?);
    // METRICS_TOKEN: the metrics for Prometheus under /metrics, for whoever
    // sends this token as "Authorization: Bearer". Without it there are none;
    // one that is too short stops the start.
    if let Some(metrics) =
        rueder::metrics::Metrics::from_setting(env::var("METRICS_TOKEN").ok().as_deref())?
    {
        tracing::info!("metrics for Prometheus under /metrics (METRICS_TOKEN)");
        state.metrics = Some(Arc::new(metrics));
    }
    let app = router(state, &settings.static_dir);
    let listener = TcpListener::bind(&settings.listen_addr).await?;
    tracing::info!("listening on http://{}", settings.listen_addr);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown())
    .await?;
    Ok(())
}

/// An hour, how often most of what the server tidies runs.
const HOUR: Duration = Duration::from_secs(60 * 60);

/// Runs `task` with a copy of `with` every `period` for as long as the
/// server runs, the first time at once or after the first period. Each task
/// says in the log what went wrong; the next time runs all the same.
fn periodically<T, F, Fut>(period: Duration, at_once: bool, with: T, task: F)
where
    T: Clone + Send + 'static,
    F: Fn(T) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send,
{
    tokio::spawn(async move {
        let mut every = tokio::time::interval(period);
        if !at_once {
            every.tick().await;
        }
        loop {
            every.tick().await;
            task(with.clone()).await;
        }
    });
}

/// Resolves on Ctrl+C or SIGTERM (what `docker stop` sends).
async fn shutdown() {
    let mut terminate = signal(SignalKind::terminate()).expect("installing the SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = terminate.recv() => {}
    }
}
