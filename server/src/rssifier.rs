//! RSSifier, the program next to rueder that makes feeds of web pages without
//! one (A25, `rssifier/`). The feed search asks it when a page names no feed,
//! and the fetching asks it for the feeds of such pages (`feeds.source` is
//! `page`), always with the token the operator gave rueder.
//!
//! It is reached through a client of its own: one that may go into the
//! operator's network, where RSSifier runs, but only ever asks the address
//! the operator named, never one a reader chose, and follows no redirect.
//! What RSSifier answers counts as coming from outside, like any feed.

use std::time::Duration;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use url::Url;
use utoipa::ToSchema;

use crate::http_client::{self, HttpClient, Request, Response};

/// What rueder takes from RSSifier.
const ACCEPT: &str = "application/feed+json, application/problem+json;q=0.9";
/// The largest answer of RSSifier, in bytes: a feed of short texts.
const MAX_BODY: usize = 4 * 1024 * 1024;
/// How long one answer may take. RSSifier waits a while for a busy host
/// before it fetches the page itself.
const TIMEOUT: Duration = Duration::from_secs(60);
/// The longest `detail` of a problem that is taken, in characters.
const MAX_DETAIL: usize = 300;

/// RSSifier as rueder reaches it.
pub struct Rssifier {
    client: HttpClient,
    /// Where it answers, ending in `/`.
    base: Url,
    /// `Bearer` and the token.
    authorization: String,
}

/// A token's limits in RSSifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[ts(rename = "RssifierLimits")]
#[schema(as = RssifierLimits)]
pub struct Limits {
    pub per_minute: Option<u32>,
    pub fetches_per_hour: Option<u32>,
    pub at_once: Option<u32>,
}

/// What RSSifier says rueder's token used of its limits (`GET /v1/usage`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, ToSchema)]
#[ts(rename = "RssifierUsage")]
#[schema(as = RssifierUsage)]
pub struct Usage {
    /// The name of the token.
    pub name: String,
    pub limits: Limits,
    /// Requests in the last minute, as far as the limit counts them.
    pub requests: u32,
    /// Pages fetched in the last hour, as far as the limit counts them.
    pub fetches: u32,
    /// Requests under way now.
    pub under_way: u32,
}

/// The part of a feed of RSSifier the feed search needs.
#[derive(Deserialize)]
pub struct Page {
    /// The page as RSSifier writes it, which is the address of the feed.
    pub home_page_url: String,
    #[serde(default)]
    pub title: String,
}

#[derive(Deserialize)]
struct Problem {
    detail: String,
}

impl Rssifier {
    /// RSSifier as `RSSIFIER_URL` and `RSSIFIER_TOKEN` name it; `None`
    /// without them, an error with only one.
    pub fn from_env() -> anyhow::Result<Option<Self>> {
        let url = std::env::var("RSSIFIER_URL").ok().filter(|v| !v.is_empty());
        let token = std::env::var("RSSIFIER_TOKEN")
            .ok()
            .filter(|v| !v.is_empty());
        match (url, token) {
            (None, None) => Ok(None),
            (Some(url), Some(token)) => Self::new(&url, &token).map(Some),
            (Some(_), None) => anyhow::bail!("RSSIFIER_URL needs RSSIFIER_TOKEN"),
            (None, Some(_)) => anyhow::bail!("RSSIFIER_TOKEN needs RSSIFIER_URL"),
        }
    }

    /// RSSifier at `url`, asked with `token`.
    pub fn new(url: &str, token: &str) -> anyhow::Result<Self> {
        let mut base = Url::parse(url.trim()).context("RSSIFIER_URL is no address")?;
        if !matches!(base.scheme(), "http" | "https") || base.host_str().is_none() {
            anyhow::bail!("RSSIFIER_URL is no address of http or https");
        }
        if !base.path().ends_with('/') {
            base.set_path(&format!("{}/", base.path()));
        }
        base.set_query(None);
        base.set_fragment(None);
        let token = token.trim();
        if token.is_empty() || !token.bytes().all(|b| b.is_ascii_graphic()) {
            anyhow::bail!("RSSIFIER_TOKEN is no token");
        }
        // The one address is the operator's, so the client may go wherever it
        // is, the own network too.
        let everywhere = ["0.0.0.0/0", "::/0"].map(|net| net.parse().map_err(anyhow::Error::msg));
        let client = HttpClient::new(http_client::Config {
            agent: http_client::AGENT.to_owned(),
            allow: everywhere.into_iter().collect::<anyhow::Result<_>>()?,
            max_body: MAX_BODY,
            max_redirects: 0,
            timeout: TIMEOUT,
            ..Default::default()
        })?;
        Ok(Self {
            client,
            base,
            authorization: format!("Bearer {token}"),
        })
    }

    fn address(&self, path: &str) -> Result<Url, http_client::Error> {
        self.base
            .join(path)
            .map_err(|err| http_client::Error::Address(err.to_string()))
    }

    /// RSSifier's feed of a page, or "not modified" (304) if it is the one of
    /// `etag`. A problem is an answer too: [`problem`] says what it is.
    pub async fn feed(
        &self,
        page: &str,
        etag: Option<&str>,
    ) -> Result<Response, http_client::Error> {
        let mut address = self.address("v1/feed")?;
        address.query_pairs_mut().append_pair("url", page);
        let request = Request {
            accept: Some(ACCEPT),
            etag,
            authorization: Some(&self.authorization),
            ..Request::new(address.as_str())
        };
        self.client.get(request).await
    }

    /// What rueder's token used, for the status page.
    pub async fn usage(&self) -> anyhow::Result<Usage> {
        let address = self.address("v1/usage")?;
        let request = Request {
            accept: Some("application/json"),
            authorization: Some(&self.authorization),
            ..Request::new(address.as_str())
        };
        let response = self.client.get(request).await?;
        if !response.status.is_success() {
            anyhow::bail!("{}", problem(&response));
        }
        serde_json::from_slice(&response.body).context("RSSifier answers no usage")
    }
}

/// What a problem RSSifier answers says, fit to stand at a feed: its
/// `detail`, or the status if there is none.
pub fn problem(response: &Response) -> String {
    match serde_json::from_slice::<Problem>(&response.body) {
        Ok(problem) if !problem.detail.trim().is_empty() => problem
            .detail
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .take(MAX_DETAIL)
            .collect(),
        _ => format!("RSSifier answers with {}", response.status),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_address_and_the_token_have_to_make_sense() {
        let rssifier = Rssifier::new(" http://rssifier:8080/base?x#y ", "rsf_abc").unwrap();
        assert_eq!(rssifier.base.as_str(), "http://rssifier:8080/base/");
        assert_eq!(
            rssifier.address("v1/feed").unwrap().as_str(),
            "http://rssifier:8080/base/v1/feed"
        );
        assert_eq!(rssifier.authorization, "Bearer rsf_abc");
        for (url, token) in [
            ("rssifier:8080", "rsf_abc"),
            ("ftp://rssifier/", "rsf_abc"),
            ("http://rssifier/", ""),
            ("http://rssifier/", "rsf abc"),
            ("http://rssifier/", "rsf_\tx"),
        ] {
            assert!(Rssifier::new(url, token).is_err(), "{url} {token:?}");
        }
    }

    #[test]
    fn a_problem_says_its_detail_or_its_status() {
        let answer = |body: &str| Response {
            url: Url::parse("http://rssifier/v1/feed").unwrap(),
            status: reqwest::StatusCode::UNPROCESSABLE_ENTITY,
            headers: reqwest::header::HeaderMap::new(),
            body: body.as_bytes().to_vec(),
        };
        let detail = r#"{"type":"no-items","status":422,"detail":"the page lists\nnothing"}"#;
        assert_eq!(problem(&answer(detail)), "the page lists nothing");
        let long = format!(r#"{{"detail":"{}"}}"#, "x".repeat(1000));
        assert_eq!(problem(&answer(&long)).len(), MAX_DETAIL);
        for body in ["", "<html>", r#"{"detail":" "}"#] {
            assert_eq!(
                problem(&answer(body)),
                "RSSifier answers with 422 Unprocessable Entity"
            );
        }
    }
}
