//! The one way out to the internet: feeds, web pages for the feed search and
//! images are all fetched through the hardened client of the crate
//! `hardened-http` next to the server, so the rules against fetching from
//! inside the own network (SSRF) are in one place. RSSifier uses the same
//! crate.

pub use hardened_http::*;

/// What rueder calls itself when it fetches: `Config::agent` of the server's
/// client.
pub const AGENT: &str = concat!(
    "rueder/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/FionaPreroll/rueder)"
);
