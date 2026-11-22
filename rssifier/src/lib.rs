//! RSSifier makes feeds of web pages that have none, for rueder and other feed
//! readers. It is a program of its own next to rueder, without a database:
//! the two only talk HTTP to each other, and RSSifier answers a JSON Feed
//! that the reader treats like any feed from outside.
//!
//! - [`api`]: the HTTP interface and how a request goes.
//! - [`tokens`]: who may ask, [`limits`]: how much.
//! - [`extract`]: what a page lists, read by [`html`] within limits, with
//!   the [`recipes`] of the operator, written with [`select`]ors, and the
//!   [`guess`].
//! - [`robots`]: what a site allows, [`Sites`]: what the operator allows.
//! - [`cache`]: answers kept for a while, [`feed`]: what goes out.
//!
//! Everything goes out to the internet through the hardened client of the
//! crate `hardened-http`, which rueder uses too.

use std::sync::Arc;

use axum::{
    Router,
    http::{HeaderValue, header},
    response::Response,
};
use hardened_http::HttpClient;

pub mod api;
pub mod cache;
pub mod extract;
pub mod feed;
pub mod guess;
pub mod health;
pub mod html;
pub mod limits;
pub mod recipes;
pub mod robots;
pub mod select;
pub mod tokens;

/// What RSSifier calls itself when it fetches a page; `robots.txt` names it
/// as [`robots::NAME`].
pub const AGENT: &str = concat!(
    "rueder-rssifier/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/FionaPreroll/rueder)"
);

/// What the operator sets about sites: how to read some, and which not to
/// fetch from at all.
#[derive(Debug, Default)]
pub struct Sites {
    pub recipes: recipes::Recipes,
    /// Hosts, each also for its subdomains, written in lower case.
    blocked: Vec<String>,
}

impl Sites {
    /// `blocked` is a list of hosts separated by commas, as
    /// `RSSIFIER_BLOCKED_HOSTS` has it.
    pub fn new(recipes: recipes::Recipes, blocked: &str) -> Self {
        let blocked = blocked
            .split(',')
            .map(|host| {
                host.trim()
                    .trim_start_matches("*.")
                    .trim_end_matches('.')
                    .to_ascii_lowercase()
            })
            .filter(|host| !host.is_empty())
            .collect();
        Self { recipes, blocked }
    }

    /// Whether the operator does not fetch from this host.
    pub fn blocks(&self, host: &str) -> bool {
        let host = host.trim_end_matches('.').to_ascii_lowercase();
        self.blocked.iter().any(|blocked| {
            host.strip_suffix(blocked.as_str())
                .is_some_and(|before| before.is_empty() || before.ends_with('.'))
        })
    }
}

/// What every request can reach.
#[derive(Clone)]
pub struct AppState {
    pub client: HttpClient,
    pub tokens: Arc<tokens::Keeper>,
    pub usage: Arc<limits::Tokens>,
    pub gates: Arc<limits::Gates>,
    pub cache: Arc<cache::Cache>,
    pub sites: Arc<Sites>,
    pub robots: Arc<robots::Robots>,
}

impl AppState {
    pub fn new(
        client: HttpClient,
        tokens: tokens::Keeper,
        pace: limits::Pace,
        sites: Sites,
    ) -> Self {
        Self {
            client,
            tokens: Arc::new(tokens),
            usage: Arc::default(),
            gates: Arc::new(limits::Gates::new(pace)),
            cache: Arc::new(cache::Cache::new(pace.cache_for)),
            sites: Arc::new(sites),
            robots: Arc::default(),
        }
    }
}

/// The whole server. Every answer belongs to one token and is not kept by a
/// cache on the way; the client asks again with `If-None-Match`.
pub fn app(state: AppState) -> Router {
    let (router, _) = api::routes().split_for_parts();
    router
        .layer(axum::middleware::map_response(
            |mut response: Response| async move {
                let headers = response.headers_mut();
                headers.insert(
                    header::X_CONTENT_TYPE_OPTIONS,
                    HeaderValue::from_static("nosniff"),
                );
                headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                response
            },
        ))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blocked_host_blocks_its_subdomains_and_nothing_else() {
        let sites = Sites::new(
            recipes::Recipes::default(),
            " Example.org., *.tracker.net,, ",
        );
        for host in [
            "example.org",
            "www.EXAMPLE.org",
            "a.tracker.net",
            "tracker.net.",
        ] {
            assert!(sites.blocks(host), "{host}");
        }
        for host in ["badexample.org", "example.org.evil.net", "net"] {
            assert!(!sites.blocks(host), "{host}");
        }
    }
}
