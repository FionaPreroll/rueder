//! `rssifier serve` runs the server, `rssifier token …` keeps the tokens,
//! `rssifier fetch` shows what the server would get from a page.

use std::{io::Write, net::SocketAddr, path::PathBuf};

use anyhow::{Context, bail};
use axum::http::HeaderValue;
use clap::{Parser, Subcommand};
use hardened_http::{Config, HttpClient, Net};
use rssifier::{
    AGENT, AppState, Sites, api, app, extract, health,
    limits::Pace,
    recipes::Recipes,
    tokens::{self, Keeper, Limits, Token},
};

#[derive(Parser)]
#[command(version, about = "Makes feeds of web pages that have none")]
struct Cli {
    /// The file with the tokens, written by `rssifier token`.
    #[arg(
        long,
        env = "RSSIFIER_TOKENS",
        default_value = "tokens.json",
        global = true
    )]
    tokens: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Runs the server.
    Serve {
        #[arg(long, env = "LISTEN_ADDR", default_value = "127.0.0.1:3100")]
        listen: SocketAddr,
        #[command(flatten)]
        reading: Reading,
        /// Hosts never fetched from, each also for its subdomains, comma
        /// separated.
        #[arg(long, env = "RSSIFIER_BLOCKED_HOSTS", default_value = "")]
        blocked: String,
        /// One more token, named "env" and without limits, so that a setup
        /// needs no `rssifier token add`.
        #[arg(long, env = "RSSIFIER_TOKEN", hide_env_values = true)]
        token: Option<String>,
    },
    /// Fetches a page as the server does, at the address a recipe names for
    /// it, without asking robots.txt. Prints the page and, on stderr, what
    /// the site answered and what RSSifier reads in it.
    Fetch {
        url: String,
        #[command(flatten)]
        reading: Reading,
    },
    /// Asks the server that runs on LISTEN_ADDR whether it is well: the
    /// health check of the container. Fails if it does not answer.
    Health {
        #[arg(long, env = "LISTEN_ADDR", default_value = "127.0.0.1:3100")]
        listen: String,
    },
    /// Makes, lists and ends tokens.
    #[command(subcommand)]
    Token(TokenCommand),
}

/// How pages are fetched and read, for `serve` and `fetch`.
#[derive(clap::Args)]
struct Reading {
    /// Networks that are fetched from although they are internal, comma
    /// separated; for tests against a server on this machine.
    #[arg(long, env = "FETCH_ALLOW_NETS", default_value = "")]
    allow: String,
    /// A file of recipes, written like `rules/recipes.toml`, that go before
    /// those that come with RSSifier.
    #[arg(long, env = "RSSIFIER_RECIPES")]
    recipes: Option<PathBuf>,
    /// Optional recipes to turn on, comma separated: "reddit".
    #[arg(long, env = "RSSIFIER_ENABLE", default_value = "")]
    enable: String,
    /// What RSSifier calls itself when it fetches. `robots.txt` is read for
    /// "rueder-rssifier" whatever this says.
    #[arg(long = "user-agent", env = "RSSIFIER_USER_AGENT", default_value = AGENT)]
    agent: String,
}

impl Reading {
    fn recipes(&self) -> anyhow::Result<Recipes> {
        let recipes = match &self.recipes {
            Some(file) => Recipes::with_file(file)?,
            None => Recipes::built_in()?,
        };
        recipes.enable(&self.enable)
    }

    /// Empty, as compose passes it when `.env` names none, is the default.
    fn agent(&self) -> &str {
        match self.agent.trim() {
            "" => AGENT,
            agent => agent,
        }
    }

    fn client(&self) -> anyhow::Result<HttpClient> {
        let allow = self
            .allow
            .split(',')
            .map(str::trim)
            .filter(|net| !net.is_empty())
            .map(|net| {
                net.parse::<Net>()
                    .map_err(|err| anyhow::anyhow!("FETCH_ALLOW_NETS: {err}"))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        if HeaderValue::from_str(self.agent()).is_err() {
            bail!(
                "RSSIFIER_USER_AGENT is not a header value: {:?}",
                self.agent
            );
        }
        HttpClient::new(Config {
            agent: self.agent().to_owned(),
            allow,
            ..Config::default()
        })
    }
}

#[derive(clap::Args)]
struct LimitArgs {
    /// Requests per minute.
    #[arg(long)]
    per_minute: Option<u32>,
    /// Pages fetched per hour, past the cache.
    #[arg(long)]
    fetches_per_hour: Option<u32>,
    /// Requests under way at the same time.
    #[arg(long)]
    at_once: Option<u32>,
}

impl From<LimitArgs> for Limits {
    fn from(args: LimitArgs) -> Self {
        Self {
            per_minute: args.per_minute,
            fetches_per_hour: args.fetches_per_hour,
            at_once: args.at_once,
        }
    }
}

#[derive(Subcommand)]
enum TokenCommand {
    /// Makes a token and shows it once. Without limits it has none.
    Add {
        name: String,
        #[command(flatten)]
        limits: LimitArgs,
    },
    /// Lists the tokens with their limits.
    List,
    /// Sets the limits of a token; a limit not named no longer holds.
    Limit {
        name: String,
        #[command(flatten)]
        limits: LimitArgs,
    },
    /// Ends a token; the server stops taking it with the next request.
    Revoke { name: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let cli = Cli::parse();
    match cli.command {
        Command::Serve {
            listen,
            reading,
            blocked,
            token,
        } => {
            let mut keeper = Keeper::new(cli.tokens.clone())?;
            if let Some(token) = token.filter(|token| !token.is_empty()) {
                keeper = keeper.with(tokens::chosen(token.trim())?);
            }
            let sites = Sites::new(reading.recipes()?, &blocked);
            serve(keeper, &cli.tokens, listen, reading.client()?, sites).await
        }
        Command::Fetch { url, reading } => fetch(&url, &reading).await,
        Command::Health { listen } => {
            health::check(health::address(&listen).await?, health::WAIT).await
        }
        Command::Token(command) => token(&cli.tokens, command),
    }
}

async fn serve(
    keeper: Keeper,
    tokens: &std::path::Path,
    listen: SocketAddr,
    client: HttpClient,
    sites: Sites,
) -> anyhow::Result<()> {
    let state = AppState::new(client, keeper, Pace::default(), sites);
    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .with_context(|| format!("listening on {listen}"))?;
    tracing::info!(
        "RSSifier listens on {listen}, tokens from {}",
        tokens.display()
    );
    axum::serve(listener, app(state))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

async fn fetch(address: &str, reading: &Reading) -> anyhow::Result<()> {
    let client = reading.client()?;
    let recipes = reading.recipes()?;
    let url = extract::link(&client.parse(address)?, address).context("no web address")?;
    let url = recipes.address(&url);
    eprintln!("fetching {url} as {}", reading.agent());
    let response = client.get(api::page_request(url.as_str())).await?;
    eprintln!("{} from {}", response.status, response.url);
    let kind = response.header("content-type").unwrap_or("no content type");
    eprintln!("{kind}, {} bytes", response.body.len());
    std::io::stdout().write_all(&response.body)?;
    let text = String::from_utf8_lossy(&response.body);
    let page = extract::page(&text, &response.url, &recipes);
    let source = if page.source.is_empty() {
        "nothing"
    } else {
        page.source
    };
    let scripted = if page.scripted {
        ", made by scripts"
    } else {
        ""
    };
    eprintln!(
        "\nread: {} entries from {source}{scripted}",
        page.items.len()
    );
    for item in &page.items {
        eprintln!("  {} {}", item.url, item.title);
    }
    Ok(())
}

fn token(path: &std::path::Path, command: TokenCommand) -> anyhow::Result<()> {
    let mut all = tokens::load(path)?;
    match command {
        TokenCommand::Add { name, limits } => {
            if all.iter().any(|token| token.name == name) {
                bail!("there is a token named {name} already");
            }
            let secret = tokens::new_secret()?;
            all.push(Token {
                name,
                hash: tokens::hash(&secret),
                limits: limits.into(),
            });
            tokens::save(path, &all)?;
            println!("{secret}");
            eprintln!("Shown only now: the file keeps its hash.");
        }
        TokenCommand::List => {
            for token in &all {
                let limit = |limit: Option<u32>| limit.map_or("-".to_owned(), |n| n.to_string());
                println!(
                    "{}\tper minute {}\tfetches per hour {}\tat once {}",
                    token.name,
                    limit(token.limits.per_minute),
                    limit(token.limits.fetches_per_hour),
                    limit(token.limits.at_once),
                );
            }
        }
        TokenCommand::Limit { name, limits } => {
            let Some(token) = all.iter_mut().find(|token| token.name == name) else {
                bail!("there is no token named {name}");
            };
            token.limits = limits.into();
            tokens::save(path, &all)?;
        }
        TokenCommand::Revoke { name } => {
            let before = all.len();
            all.retain(|token| token.name != name);
            if all.len() == before {
                bail!("there is no token named {name}");
            }
            tokens::save(path, &all)?;
        }
    }
    Ok(())
}
