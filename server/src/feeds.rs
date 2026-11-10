//! From the bytes of a feed to articles in the store.
//!
//! [`parse`] reads RSS, Atom and JSON Feed and turns every entry into an article:
//! with the id it has in its feed, the text as it came (kept as the version) and
//! the text cleaned for reading. [`store`] then sorts out what the feed already
//! has: unknown ids are new articles, known ones with another checksum get a new
//! version, the rest is left alone.

use std::collections::{HashMap, HashSet};

use anyhow::Context;
use feed_rs::model::{Entry, Link};
use sha2::{Digest, Sha256};
use sqlx::PgConnection;
use time::OffsetDateTime;

use crate::articles::{self, NewArticle, Original, guid_hash};

/// How many articles one fetch takes from a feed, counted from its top.
pub const MAX_ARTICLES: usize = 500;
/// The longest text of one article, in bytes. Longer ones are left out.
pub const MAX_CONTENT: usize = 1024 * 1024;
/// The longest teaser, in characters.
const TEASER: usize = 300;
const MAX_TAGS: usize = 20;

/// What a feed says about itself and the articles it lists.
#[derive(Debug)]
pub struct Parsed {
    pub title: Option<String>,
    pub site_url: Option<String>,
    pub icon_url: Option<String>,
    /// Oldest first, the order they go into the store.
    pub articles: Vec<NewArticle>,
    /// Entries left out: without anything to tell them apart by, too long, or
    /// beyond [`MAX_ARTICLES`].
    pub skipped: usize,
}

/// Reads a feed. `url` is where it came from, for links relative to it.
pub fn parse(body: &[u8], url: &str, fetched_at: OffsetDateTime) -> anyhow::Result<Parsed> {
    let parser = feed_rs::parser::Builder::new()
        .base_uri(Some(url))
        // The library would make up a random id where a feed gives none, and the
        // article would be new on every fetch. Leave it empty; see `guid`.
        .id_generator(|_, _, _| String::new())
        .build();
    let feed = parser.parse(body).context("not a feed that can be read")?;

    let listed = feed.entries.len();
    // Feeds list their newest entries first.
    let mut articles: Vec<NewArticle> = feed
        .entries
        .into_iter()
        .take(MAX_ARTICLES)
        .filter_map(|entry| article(entry, fetched_at))
        .collect();
    let skipped = listed - articles.len();
    articles.reverse();
    // Where a feed lists them otherwise, their dates say what came first. Equal
    // dates keep the order of the feed.
    articles.sort_by_key(|article| article.published_at);

    Ok(Parsed {
        title: feed
            .title
            .map(|title| text(&title.content))
            .filter(|title| !title.is_empty()),
        site_url: link(&feed.links),
        icon_url: feed.icon.or(feed.logo).and_then(|image| web(&image.uri)),
        articles,
        skipped,
    })
}

fn article(entry: Entry, fetched_at: OffsetDateTime) -> Option<NewArticle> {
    let title = entry
        .title
        .as_ref()
        .map(|title| text(&title.content))
        .unwrap_or_default();
    let summary = entry.summary.map(|summary| summary.content);
    let content = entry
        .content
        .and_then(|content| content.body)
        .or_else(|| summary.clone())
        .unwrap_or_default();
    if content.len() > MAX_CONTENT {
        return None;
    }
    let url = link(&entry.links);
    let guid = guid(&entry.id, url.as_deref(), &title, &content)?;

    // A date in the future would keep the article on top of every list.
    let published_at = entry
        .published
        .or(entry.updated)
        .and_then(|at| OffsetDateTime::from_unix_timestamp(at.timestamp()).ok())
        .map_or(fetched_at, |at| at.min(fetched_at));

    let media = entry.media;
    let thumbnail_url = media
        .iter()
        .flat_map(|media| &media.thumbnails)
        .find_map(|thumbnail| web(&thumbnail.image.uri))
        .or_else(|| {
            let images = media.iter().flat_map(|media| &media.content);
            images
                .filter(|content| {
                    content
                        .content_type
                        .as_ref()
                        .is_some_and(|kind| kind.ty().as_str() == "image")
                })
                .find_map(|content| web(content.url.as_ref()?.as_str()))
        });
    let enclosures: Vec<_> = media
        .iter()
        .flat_map(|media| &media.content)
        .filter_map(|content| {
            let url = web(content.url.as_ref()?.as_str())?;
            let kind = content.content_type.as_ref().map(ToString::to_string);
            Some(serde_json::json!({ "url": url, "type": kind, "length": content.size }))
        })
        .collect();

    let mut tags: Vec<String> = Vec::new();
    for category in entry.categories {
        let tag = text(&category.label.unwrap_or(category.term));
        if !tag.is_empty() && !tags.contains(&tag) && tags.len() < MAX_TAGS {
            tags.push(tag);
        }
    }
    let authors = entry
        .authors
        .into_iter()
        .filter_map(|person| person.name)
        .map(|name| text(&name))
        .filter(|name| !name.is_empty())
        .collect();

    Some(NewArticle {
        guid,
        published_at,
        fetched_at,
        teaser: teaser(summary.as_deref().unwrap_or(&content)),
        thumbnail_url: thumbnail_url.clone(),
        // The cleaning with rueder's own rules (work package 2.3) builds on this:
        // only markup that is known to be harmless stays.
        html: ammonia::clean(&content),
        enclosures: enclosures.into(),
        image_url: thumbnail_url,
        image_title: None,
        original: Original {
            title,
            content,
            authors,
            tags,
            url,
        },
    })
}

/// What tells the article apart within its feed: the id the feed gives, else
/// its link, else a checksum of title and text. `None` if there is nothing.
fn guid(id: &str, url: Option<&str>, title: &str, content: &str) -> Option<String> {
    if !id.trim().is_empty() {
        return Some(id.trim().to_owned());
    }
    if let Some(url) = url {
        return Some(url.to_owned());
    }
    if title.is_empty() && content.is_empty() {
        return None;
    }
    let digest = Sha256::new()
        .chain_update(title)
        .chain_update([0])
        .chain_update(content)
        .finalize();
    Some(
        digest
            .iter()
            .take(16)
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    )
}

/// The link to read at: the one marked as the page itself, else the first.
fn link(links: &[Link]) -> Option<String> {
    let page = links
        .iter()
        .find(|link| link.rel.as_deref().is_none_or(|rel| rel == "alternate"));
    page.or(links.first()).and_then(|link| web(&link.href))
}

/// The address, if it is one a browser may be sent to.
fn web(url: &str) -> Option<String> {
    let url = url.trim();
    let scheme = url.split_once(':')?.0.to_ascii_lowercase();
    matches!(scheme.as_str(), "http" | "https").then(|| url.to_owned())
}

/// The words of a piece of HTML, without markup, with single blanks.
fn text(html: &str) -> String {
    // Where a paragraph or a line ends, words must not run together.
    let mut spaced = html.to_owned();
    for block in [
        "<p",
        "</p",
        "<br",
        "<div",
        "</div",
        "<li",
        "<h",
        "<td",
        "<tr",
        "<blockquote",
    ] {
        spaced = spaced.replace(block, &format!(" {block}"));
    }
    let bare = ammonia::Builder::empty().clean(&spaced).to_string();
    let bare = bare
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&");
    bare.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn teaser(html: &str) -> String {
    let text = text(html);
    match text.char_indices().nth(TEASER) {
        None => text,
        Some((end, _)) => format!("{}…", text[..end].trim_end()),
    }
}

/// What [`store`] did with the articles of one fetch.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Stored {
    /// Articles the feed did not have.
    pub new: usize,
    /// Articles whose text changed and that got a new version.
    pub updated: usize,
    /// The number of the first new article.
    pub first_new: Option<i32>,
}

/// Puts the articles of one fetch into the store. Call it inside a
/// transaction: it locks the feed, so two fetches of the same feed cannot both
/// take an article for new.
pub async fn store(
    conn: &mut PgConnection,
    feed_id: i32,
    articles: &[NewArticle],
) -> anyhow::Result<Stored> {
    sqlx::query_scalar!("SELECT id FROM feeds WHERE id = $1 FOR UPDATE", feed_id)
        .fetch_optional(&mut *conn)
        .await?
        .with_context(|| format!("no feed {feed_id}"))?;

    let hashes: Vec<Vec<u8>> = articles
        .iter()
        .map(|article| guid_hash(&article.guid))
        .collect();
    let known: HashMap<Vec<u8>, (i32, Vec<u8>)> = sqlx::query!(
        "SELECT guid_hash, seq, checksum FROM article_content WHERE feed_id = $1 AND guid_hash = ANY($2)",
        feed_id,
        &hashes,
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(|row| (row.guid_hash, (row.seq, row.checksum)))
    .collect();

    let mut stored = Stored::default();
    let mut new = Vec::new();
    let mut seen = HashSet::new();
    for (article, hash) in articles.iter().zip(&hashes) {
        // A feed that lists an id twice means one article.
        if !seen.insert(hash) {
            continue;
        }
        match known.get(hash) {
            None => new.push(article.clone()),
            Some((seq, checksum)) if *checksum != article.original.checksum() => {
                if articles::add_version(conn, feed_id, *seq, article)
                    .await?
                    .is_some()
                {
                    stored.updated += 1;
                }
            }
            Some(_) => {}
        }
    }
    if !new.is_empty() {
        stored.first_new = Some(articles::insert(conn, feed_id, &new).await?);
        stored.new = new.len();
    }
    Ok(stored)
}
