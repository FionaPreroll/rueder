//! RSSifier makes feeds of web pages that have none, for rueder and other feed
//! readers. It is a program of its own next to rueder, without a database:
//! the two only talk HTTP to each other, and RSSifier answers a JSON Feed
//! that the reader treats like any feed from outside.
//!
//! - [`api`]: the HTTP interface and how a request goes.
//! - [`tokens`]: who may ask, [`limits`]: how much.
//! - [`extract`]: what a page lists, read by [`html`] within limits.
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
pub mod html;
pub mod limits;
pub mod tokens;

/// What RSSifier calls itself when it fetches a page; `robots.txt` names it
/// as `rueder-rssifier`.
pub const AGENT: &str = concat!(
    "rueder-rssifier/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/FionaPreroll/rueder)"
);

/// What every request can reach.
#[derive(Clone)]
pub struct AppState {
    pub client: HttpClient,
    pub tokens: Arc<tokens::Keeper>,
    pub usage: Arc<limits::Tokens>,
    pub gates: Arc<limits::Gates>,
    pub cache: Arc<cache::Cache>,
}

impl AppState {
    pub fn new(client: HttpClient, tokens: tokens::Keeper, pace: limits::Pace) -> Self {
        Self {
            client,
            tokens: Arc::new(tokens),
            usage: Arc::default(),
            gates: Arc::new(limits::Gates::new(pace)),
            cache: Arc::new(cache::Cache::new(pace.cache_for)),
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
