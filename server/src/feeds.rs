//! From the bytes of a feed to articles in the store.
//!
//! [`parse`] reads RSS, Atom and JSON Feed and turns every entry into an article
//! (Atom 0.3, which some sites still send, after [`atom_1`] has written it as
//! Atom 1.0):
//! with the id it has in its feed, the text as it came (kept as the version) and
//! the text cleaned for reading (see [`crate::cleaning`]). [`store`] then sorts
//! out what the feed already has: unknown ids are new articles, known ones that
//! read differently get a new version, the rest is left alone.

use std::collections::{HashMap, HashSet};

use anyhow::Context;
use feed_rs::model::{Entry, Link};
use quick_xml::{
    NsReader, Writer, XmlVersion,
    escape::{escape, unescape},
    events::{BytesEnd, BytesStart, BytesText, Event},
    name::{Namespace, ResolveResult},
};
use sha2::{Digest, Sha256};
use sqlx::PgConnection;
use time::OffsetDateTime;
use url::Url;

use crate::{
    articles::{self, NewArticle, Original, guid_hash},
    cleaning::{self, Base, Rules, text},
};

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
///
/// This takes as long as the feed is large and its texts are tangled; inside
/// the server it belongs on a thread that may be kept busy.
pub fn parse(
    body: &[u8],
    url: &str,
    fetched_at: OffsetDateTime,
    rules: &Rules,
) -> anyhow::Result<Parsed> {
    let parser = feed_rs::parser::Builder::new()
        .base_uri(Some(url))
        // The library would make up a random id where a feed gives none, and the
        // article would be new on every fetch. Leave it empty; see `guid`.
        .id_generator(|_, _, _| String::new())
        .build();
    let decoded = utf_8(body);
    let body = decoded.as_deref().unwrap_or(body);
    let written = atom_1(body);
    let body = written.as_deref().unwrap_or(body);
    let feed = parser.parse(body).context("not a feed that can be read")?;

    let listed = feed.entries.len();
    // Feeds list their newest entries first.
    let mut articles: Vec<NewArticle> = feed
        .entries
        .into_iter()
        .take(MAX_ARTICLES)
        .filter_map(|entry| article(entry, url, fetched_at, rules))
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
        site_url: link(&feed.links)
            .and_then(web)
            .map(|url| rules.clean_url(&url)),
        icon_url: feed.icon.or(feed.logo).and_then(|image| web(&image.uri)),
        articles,
        skipped,
    })
}

fn article(
    entry: Entry,
    feed_url: &str,
    fetched_at: OffsetDateTime,
    rules: &Rules,
) -> Option<NewArticle> {
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
    // The id is taken from the link as the feed gives it: that is what the old
    // rueder knows its articles by.
    let link = link(&entry.links);
    let guid = guid(&entry.id, link, &title, &content)?;
    let url = link.and_then(web).map(|url| rules.clean_url(&url));

    // A date in the future would keep the article on top of every list.
    let published_at = entry
        .published
        .or(entry.updated)
        .and_then(|at| OffsetDateTime::from_unix_timestamp(at.timestamp()).ok())
        .map_or(fetched_at, |at| at.min(fetched_at));

    let media = entry.media;
    let media_image = media
        .iter()
        .flat_map(|media| &media.thumbnails)
        .find_map(|thumbnail| web(&thumbnail.image.uri))
        .map(|url| rules.clean_url(&url))
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
                .map(|url| rules.clean_url(&url))
        });
    let mut enclosures: Vec<_> = media
        .iter()
        .flat_map(|media| &media.content)
        .filter_map(|content| {
            let url = rules.clean_url(&web(content.url.as_ref()?.as_str())?);
            let kind = content.content_type.as_ref().map(ToString::to_string);
            Some(serde_json::json!({ "url": url, "type": kind, "length": content.size }))
        })
        .collect();
    let base = Base {
        article: url.as_deref(),
        feed: Some(feed_url),
    };
    let cleaned = cleaning::clean(&content, base, rules);
    add_inline(&mut enclosures, cleaned.enclosures);
    // A feed that names no picture often has one in its text, and the list
    // shows the article by it.
    let thumbnail_url = media_image
        .clone()
        .or_else(|| cleaning::first_image(&cleaned.html));

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
        thumbnail_url,
        html: cleaned.html,
        enclosures: enclosures.into(),
        image_url: media_image,
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

/// The link to read at, as the feed gives it: the one marked as the page
/// itself, else the first, if it names `http` or `https`.
fn link(links: &[Link]) -> Option<&str> {
    let page = links
        .iter()
        .find(|link| link.rel.as_deref().is_none_or(|rel| rel == "alternate"));
    let href = page.or(links.first())?.href.trim();
    let scheme = href.split_once(':')?.0.to_ascii_lowercase();
    matches!(scheme.as_str(), "http" | "https").then_some(href)
}

/// The address, if it is one a browser may be sent to: a whole `http` or
/// `https` address with a host.
fn web(url: &str) -> Option<String> {
    let url = url.trim();
    let parsed = Url::parse(url).ok()?;
    let is_web = matches!(parsed.scheme(), "http" | "https") && parsed.host_str().is_some();
    is_web.then(|| url.to_owned())
}

/// Adds the audio and video files that stood in an article's text to the
/// ones its feed lists, each file once. They are marked, so that cleaning the
/// text again can tell them from the feed's own.
pub(crate) fn add_inline(
    enclosures: &mut Vec<serde_json::Value>,
    inline: Vec<cleaning::Enclosure>,
) {
    for found in inline {
        let listed = enclosures
            .iter()
            .any(|enclosure| enclosure["url"].as_str() == Some(found.url.as_str()));
        if !listed {
            enclosures
                .push(serde_json::json!({ "url": found.url, "type": found.kind, "inline": true }));
        }
    }
}

pub(crate) fn teaser(html: &str) -> String {
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
            Some((seq, checksum)) if *checksum != article.checksum() => {
                // It reads differently. If it was sent as before, only the
                // rules of the cleaning have changed since, and the text is
                // put in place again without a new version.
                let before = articles::current_original(conn, feed_id, *seq).await?;
                let as_before = before.is_some_and(|before| {
                    before.title == article.original.title
                        && before.content == article.original.content
                });
                if as_before {
                    articles::recleaned(conn, feed_id, *seq, article).await?;
                } else if articles::add_version(conn, feed_id, *seq, article)
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

/// The namespace of Atom 0.3, which some sites still send. The feed library
/// reads only Atom 1.0 and passes over every element of the older one.
const ATOM_03: &str = "http://purl.org/atom/ns#";
const ATOM_10: &str = "http://www.w3.org/2005/Atom";
const DUBLIN_CORE: &str = "http://purl.org/dc/elements/1.1/";

/// What a text of Atom 0.3 holds, by its `type` (a media type) and its `mode`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Text,
    /// HTML written as escaped text.
    Html,
    /// Markup as elements.
    Xhtml,
}

fn kind(media: Option<&str>, mode: Option<&str>) -> Kind {
    let media = media.unwrap_or_default().to_ascii_lowercase();
    match mode.unwrap_or("xml") {
        "escaped" if media.is_empty() || media.contains("html") => Kind::Html,
        "xml" if media.contains("html") => Kind::Xhtml,
        _ => Kind::Text,
    }
}

/// A feed in XML as UTF-8 and without its declaration, in the encoding its byte
/// order mark or its declaration names.
///
/// The library switches its decoding when it meets a declaration that names
/// another encoding, and panics if that comes later than the first 64 bytes:
/// a long declaration does it, and so does a second one further down. So it
/// gets a text in UTF-8 without them. Bytes that are not of the encoding become
/// replacement characters: where the library meets such a byte, it leaves it
/// out and reads on, and the letters around it may make up a declaration.
///
/// `None` where nothing needs to change: a feed in valid UTF-8 whose
/// declarations name no other encoding, and a JSON Feed.
fn utf_8(body: &[u8]) -> Option<Vec<u8>> {
    let detected = quick_xml::encoding::detect_encoding(body);
    let bom = detected.as_ref().map_or(0, |detected| detected.bom_len());
    let rest = body.get(bom..)?;
    if rest.trim_ascii_start().starts_with(b"{") {
        return None;
    }
    let found = detected
        .as_ref()
        .map_or(encoding_rs::UTF_8, |detected| detected.encoding());
    // A byte order mark says it all. Otherwise the declaration names the
    // encoding, as long as its own letters are ASCII.
    let encoding = if bom > 0 || !found.is_ascii_compatible() {
        found
    } else {
        let mut reader = quick_xml::Reader::from_reader(rest);
        match reader.read_event_into(&mut Vec::new()) {
            Ok(Event::Decl(declaration)) => declaration
                .encoder()
                .filter(|named| named.is_ascii_compatible())
                .unwrap_or(found),
            _ => found,
        }
    };
    let (text, broken) = encoding.decode_without_bom_handling(rest);

    // Where a declaration names another encoding than UTF-8, as the library
    // would meet it: also after markup it would read past.
    let mut reader = quick_xml::Reader::from_str(&text);
    reader.config_mut().check_end_names = false;
    let mut declarations = Vec::new();
    loop {
        let start = reader.buffer_position();
        let event = reader.read_event();
        let end = reader.buffer_position();
        match event {
            Ok(Event::Decl(declaration))
                if declaration
                    .encoder()
                    .is_some_and(|named| named != encoding_rs::UTF_8) =>
            {
                declarations.push(start..end);
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            // Further on, if it can; the library may read on, too.
            Err(_) if end > start => {}
            Err(_) => break,
        }
    }
    if bom == 0 && encoding == encoding_rs::UTF_8 && !broken && declarations.is_empty() {
        return None;
    }
    let mut written = Vec::with_capacity(text.len());
    let mut from = 0;
    for declaration in declarations {
        let (Ok(start), Ok(end)) = (
            usize::try_from(declaration.start),
            usize::try_from(declaration.end),
        ) else {
            return None;
        };
        written.extend_from_slice(text.as_bytes().get(from..start)?);
        from = end;
    }
    written.extend_from_slice(text.as_bytes().get(from..)?);
    Some(written)
}

/// A feed in Atom 0.3 written as Atom 1.0: its namespace, the names that
/// changed (`issued`, `modified`, `tagline`, `copyright`), the kind of its texts,
/// and `dc:date`, the only date some of these feeds have, as `updated`. Escaped
/// HTML that is wrapped in a CDATA marker of its own loses the wrapper.
///
/// `None` for every other feed, and for one that cannot be read through as
/// XML: then the library gets the bytes as they came.
fn atom_1(body: &[u8]) -> Option<Vec<u8>> {
    let mut reader = NsReader::from_reader(body);
    let mut writer = Writer::new(Vec::with_capacity(body.len() + 1024));
    let mut buf = Vec::new();
    // The prefix Atom's elements have, empty where it is the default namespace.
    let mut atom: Option<String> = None;
    // Inside an escaped HTML text: what it says so far, still escaped.
    let mut html: Option<String> = None;
    loop {
        let (ns, event) = reader.read_resolved_event_into(&mut buf).ok()?;
        let is = |uri: &str| matches!(ns, ResolveResult::Bound(Namespace(n)) if n == uri);
        let (in_atom, in_dc) = (is(ATOM_03), is(DUBLIN_CORE));
        match event {
            Event::Start(ref start) | Event::Empty(ref start) => {
                if let Some(text) = html.take() {
                    writer
                        .write_event(Event::Text(BytesText::from_escaped(text)))
                        .ok()?;
                }
                let empty = matches!(event, Event::Empty(_));
                let local = start.local_name().into_inner();
                let prefix = match start.name().prefix() {
                    Some(prefix) => format!("{}:", prefix.into_inner()),
                    None => String::new(),
                };
                if atom.is_none() {
                    // Only a feed of Atom 0.3 is written anew.
                    if !(in_atom && local == "feed") {
                        return None;
                    }
                    atom = Some(prefix.clone());
                }
                let name = match (in_atom, in_dc, local) {
                    (true, _, _) => format!("{prefix}{}", renamed(local)),
                    (_, true, "date") => format!("{}updated", atom.as_deref().unwrap_or_default()),
                    _ => start.name().into_inner().to_owned(),
                };
                let mut written = BytesStart::new(name);
                let (mut media, mut mode) = (None, None);
                for attribute in start.attributes() {
                    let attribute = attribute.ok()?;
                    let key = attribute.key.into_inner();
                    let value = attribute.normalized_value(XmlVersion::Implicit1_0).ok()?;
                    let declares = key == "xmlns" || key.starts_with("xmlns:");
                    if declares && value == ATOM_03 {
                        written.push_attribute((key, ATOM_10));
                    } else if in_atom && key == "type" {
                        media = Some(value.into_owned());
                    } else if in_atom && key == "mode" {
                        mode = Some(value.into_owned());
                    } else {
                        written.push_attribute(attribute);
                    }
                }
                let text = in_atom && TEXTS.contains(&local);
                if text {
                    let kind = kind(media.as_deref(), mode.as_deref());
                    let kind = match kind {
                        Kind::Text => "text",
                        Kind::Html => "html",
                        Kind::Xhtml => "xhtml",
                    };
                    written.push_attribute(("type", kind));
                    if kind == "html" && !empty {
                        html = Some(String::new());
                    }
                } else if let Some(media) = media {
                    // A link's or a source's type stays.
                    written.push_attribute(("type", media.as_str()));
                }
                let written = if empty {
                    Event::Empty(written)
                } else {
                    Event::Start(written)
                };
                writer.write_event(written).ok()?;
            }
            Event::End(ref end) => {
                if let Some(text) = html.take() {
                    writer.write_event(Event::Text(unwrapped(&text)?)).ok()?;
                }
                let local = end.local_name().into_inner();
                let name = match (in_atom, in_dc, local) {
                    (true, _, _) => {
                        let prefix = match end.name().prefix() {
                            Some(p) => format!("{}:", p.into_inner()),
                            None => String::new(),
                        };
                        format!("{prefix}{}", renamed(local))
                    }
                    (_, true, "date") => format!("{}updated", atom.as_deref().unwrap_or_default()),
                    _ => end.name().into_inner().to_owned(),
                };
                writer.write_event(Event::End(BytesEnd::new(name))).ok()?;
            }
            Event::Text(ref text) if html.is_some() => {
                html.as_mut()?.push_str(text);
            }
            Event::GeneralRef(ref reference) if html.is_some() => {
                html.as_mut()?.push_str(&format!("&{};", &**reference));
            }
            Event::CData(ref data) if html.is_some() => {
                html.as_mut()?.push_str(&escape(&**data));
            }
            Event::Eof => break,
            other => writer.write_event(other).ok()?,
        }
        buf.clear();
    }
    atom.is_some().then(|| writer.into_inner())
}

/// The elements of Atom 0.3 that hold a text.
const TEXTS: [&str; 6] = [
    "title",
    "tagline",
    "copyright",
    "summary",
    "content",
    "info",
];

/// An element's name in Atom 1.0.
fn renamed(local: &str) -> &str {
    match local {
        "issued" => "published",
        "modified" => "updated",
        "tagline" => "subtitle",
        "copyright" => "rights",
        other => other,
    }
}

/// Escaped HTML as a text again, without a CDATA marker that stands around all
/// of it as text: tagesschau.de wraps its articles in one before escaping them.
fn unwrapped(escaped: &str) -> Option<BytesText<'static>> {
    let html = unescape(escaped).ok()?;
    let trimmed = html.trim();
    let inner = trimmed
        .strip_prefix("<![CDATA[")
        .and_then(|rest| rest.strip_suffix("]]>"));
    Some(match inner {
        Some(inner) => BytesText::from_escaped(escape(inner).into_owned()),
        None => BytesText::from_escaped(escaped.to_owned()),
    })
}
