//! What RSSifier answers: a JSON Feed 1.1, or a problem (RFC 9457).

use std::time::Duration;

use axum::{
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use time::format_description::well_known::Rfc3339;
use utoipa::ToSchema;

use crate::extract::Page;

/// A feed in JSON Feed 1.1 (<https://jsonfeed.org/version/1.1>).
#[derive(Debug, Serialize, ToSchema)]
pub struct JsonFeed {
    /// Always `https://jsonfeed.org/version/1.1`.
    pub version: &'static str,
    pub title: String,
    /// The page the feed is made of, as RSSifier writes it: a client keeps
    /// this one, so that ways of writing the same page are one feed.
    pub home_page_url: String,
    pub items: Vec<FeedItem>,
    /// How the entries were found: `json-ld` or `h-feed`.
    #[serde(rename = "_rssifier")]
    pub rssifier: Made,
}

/// How a feed was made, as an extension of JSON Feed.
#[derive(Debug, Serialize, ToSchema)]
pub struct Made {
    pub source: &'static str,
}

/// An entry of a feed.
#[derive(Debug, Serialize, ToSchema)]
pub struct FeedItem {
    /// The address of the entry, which is also its id.
    pub id: String,
    pub url: String,
    pub title: String,
    /// The short text of the entry as a paragraph, and its image; the text
    /// holds no markup of the page.
    pub content_html: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// RFC 3339, if the page says when.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_published: Option<String>,
}

/// Text for HTML, between tags or in a quoted attribute.
fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            c => escaped.push(c),
        }
    }
    escaped
}

impl JsonFeed {
    pub fn of(page: Page, address: &str) -> Self {
        let items = page
            .items
            .into_iter()
            .map(|item| {
                let mut content = String::new();
                if let Some(summary) = &item.summary {
                    content.push_str(&format!("<p>{}</p>", escape(summary)));
                }
                if let Some(image) = &item.image {
                    content.push_str(&format!(
                        "<p><img src=\"{}\" alt=\"\"></p>",
                        escape(image.as_str())
                    ));
                }
                FeedItem {
                    id: item.url.to_string(),
                    url: item.url.to_string(),
                    title: item.title,
                    content_html: content,
                    image: item.image.map(String::from),
                    date_published: item.published.and_then(|at| at.format(&Rfc3339).ok()),
                }
            })
            .collect();
        Self {
            version: "https://jsonfeed.org/version/1.1",
            title: page.title,
            home_page_url: address.to_owned(),
            items,
            rssifier: Made {
                source: page.source,
            },
        }
    }
}

/// A feed as it goes out: its bytes and their ETag.
#[derive(Debug, Clone)]
pub struct Written {
    pub body: Vec<u8>,
    pub etag: String,
}

impl Written {
    pub fn of(feed: &JsonFeed) -> Self {
        let body = serde_json::to_vec(feed).unwrap_or_default();
        let digest = Sha256::digest(&body);
        let etag = digest[..16]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        Self {
            body,
            etag: format!("\"{etag}\""),
        }
    }

    /// The answer, or 304 when the client has these bytes already.
    pub fn answer(&self, if_none_match: Option<&str>) -> Response {
        let etag = HeaderValue::from_str(&self.etag).unwrap_or(HeaderValue::from_static("\"\""));
        if if_none_match.is_some_and(|tags| tags.split(',').any(|tag| tag.trim() == self.etag)) {
            return (StatusCode::NOT_MODIFIED, [(header::ETAG, etag)]).into_response();
        }
        (
            [
                (
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/feed+json"),
                ),
                (header::ETAG, etag),
            ],
            self.body.clone(),
        )
            .into_response()
    }
}

/// Why there is no feed, as `application/problem+json`. `kind` is the fixed
/// word a client can tell problems by; `detail` is fit to be shown at a feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub status: StatusCode,
    pub kind: &'static str,
    pub detail: String,
    pub retry_after: Option<Duration>,
}

/// What a problem looks like as JSON.
#[derive(Serialize, ToSchema)]
#[schema(as = Problem)]
pub struct ProblemBody {
    /// `token`, `address`, `blocked-host`, `no-items`, `needs-javascript`,
    /// `too-large`, `quota`, `busy`, `upstream` or `timeout`.
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub status: u16,
    pub detail: String,
}

impl Problem {
    pub fn new(status: StatusCode, kind: &'static str, detail: impl Into<String>) -> Self {
        Self {
            status,
            kind,
            detail: detail.into(),
            retry_after: None,
        }
    }
}

impl IntoResponse for Problem {
    fn into_response(self) -> Response {
        let body = ProblemBody {
            kind: self.kind,
            status: self.status.as_u16(),
            detail: self.detail,
        };
        let mut response = (
            self.status,
            [(header::CONTENT_TYPE, "application/problem+json")],
            serde_json::to_vec(&body).unwrap_or_default(),
        )
            .into_response();
        if let Some(wait) = self.retry_after {
            // Whole seconds, at least one, so that nobody comes back at once.
            let seconds = wait.as_secs() + u64::from(wait.subsec_nanos() > 0);
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from(seconds.max(1)));
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_from_pages_cannot_become_markup() {
        assert_eq!(
            escape(r#"<b>"x" & y</b>"#),
            "&lt;b&gt;&quot;x&quot; &amp; y&lt;/b&gt;"
        );
    }
}
