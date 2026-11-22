//! `rssifier serve` runs the server, `rssifier token …` keeps the tokens.

use std::{net::SocketAddr, path::PathBuf};

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use hardened_http::{Config, HttpClient, Net};
use rssifier::{
    AGENT, AppState, Sites, app, health,
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
        /// Networks that are fetched from although they are internal, comma
        /// separated; for tests against a server on this machine.
        #[arg(long, env = "FETCH_ALLOW_NETS", default_value = "")]
        allow: String,
        /// A file of recipes, written like `rules/recipes.toml`, that go
        /// before those that come with RSSifier.
        #[arg(long, env = "RSSIFIER_RECIPES")]
        recipes: Option<PathBuf>,
        /// Hosts never fetched from, each also for its subdomains, comma
        /// separated.
        #[arg(long, env = "RSSIFIER_BLOCKED_HOSTS", default_value = "")]
        blocked: String,
        /// One more token, named "env" and without limits, so that a setup
        /// needs no `rssifier token add`.
        #[arg(long, env = "RSSIFIER_TOKEN", hide_env_values = true)]
        token: Option<String>,
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
            allow,
            recipes,
            blocked,
            token,
        } => {
            let recipes = match recipes {
                Some(file) => Recipes::with_file(&file)?,
                None => Recipes::built_in()?,
            };
            let mut keeper = Keeper::new(cli.tokens.clone())?;
            if let Some(token) = token.filter(|token| !token.is_empty()) {
                keeper = keeper.with(tokens::chosen(token.trim())?);
            }
            serve(
                keeper,
                &cli.tokens,
                listen,
                &allow,
                Sites::new(recipes, &blocked),
            )
            .await
        }
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
    allow: &str,
    sites: Sites,
) -> anyhow::Result<()> {
    let allow = allow
        .split(',')
        .map(str::trim)
        .filter(|net| !net.is_empty())
        .map(|net| {
            net.parse::<Net>()
                .map_err(|err| anyhow::anyhow!("FETCH_ALLOW_NETS: {err}"))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let client = HttpClient::new(Config {
        agent: AGENT.to_owned(),
        allow,
        ..Config::default()
    })?;
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
