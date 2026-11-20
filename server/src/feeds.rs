//! From the bytes of a feed to articles in the store.
//!
//! [`parse`] reads RSS, Atom and JSON Feed and turns every entry into an article
//! (Atom 0.3, which some sites still send, after [`atom_1`] has written it as
//! Atom 1.0):
//! with the id it has in its feed, the text as it came (kept as the version) and
//! the text cleaned for reading (see [`crate::cleaning`]). [`store`] then sorts
//! out what the feed already has: unknown ids are new articles, unless an
//! article of the feed has the same title and date; known ones that read
//! differently get a new version, the rest is left alone.

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt,
};

use anyhow::Context;
use feed_rs::model::{Entry, Link};
use quick_xml::{
    NsReader, Reader, Writer, XmlVersion,
    escape::{escape, unescape},
    events::{BytesEnd, BytesStart, BytesText, Event},
    name::{Namespace, ResolveResult},
};
use serde::de::{self, DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor};
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
/// How many elements and attributes of a feed in XML, or values of a JSON
/// Feed, are read. The library makes something of nearly every element it knows, up to a
/// kilobyte for an enclosure, and holds all of it at once; see [`limited`].
pub const MAX_ELEMENTS: usize = 50_000;

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
    let (body, left_out) = limited(body).context("not a feed that can be read")?;
    let written = atom_1(&body);
    let body = written.as_deref().unwrap_or(&body);
    let feed = parser.parse(body).context("not a feed that can be read")?;

    let listed = feed.entries.len() + left_out;
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
        .map(|name| author(&text(&name)))
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

/// The name of an author as a reader would write it. RSS writes an author as
/// `desk@example.org (Jonas Erbe)`; feed-rs takes the address out and leaves
/// the name in its parentheses, or angle brackets in `Name <address>`.
fn author(name: &str) -> String {
    let bare = name
        .strip_prefix('(')
        .and_then(|inner| inner.strip_suffix(')'))
        .or_else(|| {
            name.strip_prefix('<')
                .and_then(|inner| inner.strip_suffix('>'))
        })
        .unwrap_or(name);
    bare.trim().to_owned()
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
    let known: HashMap<Vec<u8>, (i32, Vec<u8>, bool)> = sqlx::query!(
        "SELECT guid_hash, seq, checksum, text_dropped FROM article_content
         WHERE feed_id = $1 AND guid_hash = ANY($2)",
        feed_id,
        &hashes,
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .map(|row| (row.guid_hash, (row.seq, row.checksum, row.text_dropped)))
    .collect();
    // Articles the retention deleted whole (`crate::retention`). A feed that
    // still lists one, as podcasts list all their episodes, does not bring it
    // back as new.
    let gone: HashSet<Vec<u8>> = sqlx::query_scalar!(
        "SELECT guid_hash FROM article_tombstones WHERE feed_id = $1 AND guid_hash = ANY($2)",
        feed_id,
        &hashes,
    )
    .fetch_all(&mut *conn)
    .await?
    .into_iter()
    .collect();
    let unknown: Vec<_> = articles
        .iter()
        .zip(&hashes)
        .filter(|(_, hash)| !known.contains_key(*hash) && !gone.contains(*hash))
        .map(|(article, _)| article)
        .collect();
    let mut seen_as = same_in_feed(conn, feed_id, &unknown).await?;

    let mut stored = Stored::default();
    let mut new = Vec::new();
    let mut seen = HashSet::new();
    for (article, hash) in articles.iter().zip(&hashes) {
        // A feed that lists an id twice means one article.
        if !seen.insert(hash) || gone.contains(hash) {
            continue;
        }
        match known.get(hash) {
            // Unless it is an article the feed already has under another id,
            // or one this fetch brings twice. That one is left as it is: the
            // links in its text change with the id.
            None => {
                if same_as(article).is_none_or(|same| seen_as.insert(same)) {
                    new.push(article.clone());
                }
            }
            // Its text went with the retention: it stays as the list has it.
            Some((_, _, true)) => {}
            Some((seq, checksum, false)) if *checksum != article.checksum() => {
                // It reads differently. If it was sent as before, only the
                // rules of the cleaning have changed since, and the text is
                // put in place again without a new version.
                let as_before = match articles::current_original(conn, feed_id, *seq).await? {
                    Some(before) => {
                        before.title == article.original.title
                            && before.content == article.original.content
                    }
                    // The retention took its versions, so what was sent before
                    // is not known. A new headline is a change; a text that
                    // reads differently may be the rules alike, and is put in
                    // place without claiming the article changed.
                    None => {
                        articles::title_of(conn, feed_id, *seq).await?.as_deref()
                            == Some(article.original.title.as_str())
                    }
                };
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

/// What tells an article apart where its id does not: its title and its date.
/// Google News gives an article a new id with every fetch. Two articles of one
/// feed with the same title at the same second are one, but only where the
/// feed gives both: without a date, every entry of a fetch has its time.
fn same_as(article: &NewArticle) -> Option<(String, i128)> {
    let dated = article.published_at != article.fetched_at;
    (dated && !article.original.title.is_empty()).then(|| {
        (
            article.original.title.clone(),
            article.published_at.unix_timestamp_nanos(),
        )
    })
}

/// Title and date, as [`same_as`] gives them, of the feed's articles that have
/// the date of one of these.
pub async fn same_in_feed(
    conn: &mut PgConnection,
    feed_id: i32,
    articles: &[&NewArticle],
) -> anyhow::Result<HashSet<(String, i128)>> {
    let (titles, dates): (Vec<_>, Vec<_>) = articles
        .iter()
        .filter(|article| same_as(article).is_some())
        .map(|article| (article.original.title.clone(), article.published_at))
        .unzip();
    if dates.is_empty() {
        return Ok(HashSet::new());
    }
    // A pair of a title and a date of two different entries comes along as
    // well, and is never asked for.
    let dated = sqlx::query!(
        "SELECT title, published_at FROM articles
         WHERE feed_id = $1 AND published_at = ANY($2) AND title = ANY($3)",
        feed_id,
        &dates,
        &titles,
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok(dated
        .into_iter()
        .map(|row| (row.title, row.published_at.unix_timestamp_nanos()))
        .collect())
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

/// A feed with no more than [`MAX_ARTICLES`] entries and [`MAX_ELEMENTS`]
/// elements, and how many entries it left out.
///
/// The library reads all of a feed before anything can be taken from it, and
/// a feed of 20 MiB of empty entries made it take several GiB. So entries
/// beyond the limit are left out before, and once a feed has used up its
/// elements, the entry being read and everything that begins after it, each
/// as a whole: what is left reads as before, only shorter. Where the XML
/// cannot be read on, the rest is left out, as the library cannot read it
/// either.
///
/// It tells JSON from XML as the library does. A JSON Feed is written anew
/// where something is left out, and one that cannot be read is an error
/// here already: the library would first hold all that comes before the
/// error.
fn limited(body: &[u8]) -> anyhow::Result<(Cow<'_, [u8]>, usize)> {
    if body.iter().find(|&&b| b == b'<' || b == b'{') != Some(&b'{') {
        return Ok(xml_limited(body));
    }
    let mut kept = Kept::default();
    let mut written = Vec::new();
    let mut reader = serde_json::Deserializer::from_slice(body);
    Copied {
        out: &mut written,
        kept: &mut kept,
        items: false,
        top: true,
    }
    .deserialize(&mut reader)?;
    reader.end()?;
    Ok(if kept.changed {
        (Cow::Owned(written), kept.left_out)
    } else {
        (Cow::Borrowed(body), 0)
    })
}

fn xml_limited(body: &[u8]) -> (Cow<'_, [u8]>, usize) {
    let mut reader = Reader::from_reader(body);
    reader.config_mut().check_end_names = false;
    let position =
        |reader: &Reader<&[u8]>| usize::try_from(reader.buffer_position()).unwrap_or(body.len());
    let mut kept = Vec::new();
    // Where the bytes still to be kept begin, and where they end.
    let (mut from, mut rest) = (0, body.len());
    let (mut depth, mut elements, mut entries, mut left_out) = (0, 0, 0, 0);
    // The entry being read, and the element being left out: depth and start.
    let mut entry: Option<(usize, usize)> = None;
    let mut leaving: Option<(usize, usize)> = None;
    let mut buf = Vec::new();
    loop {
        let start = position(&reader);
        let event = reader.read_event_into(&mut buf);
        let end = position(&reader);
        let closes = match event {
            Ok(Event::Start(ref element)) | Ok(Event::Empty(ref element)) => {
                let empty = matches!(event, Ok(Event::Empty(_)));
                if leaving.is_none() {
                    // An item of RSS 0.9x, 1.0 and 2.0, an entry of Atom.
                    let is_entry = entry.is_none()
                        && depth <= 2
                        && matches!(element.local_name().into_inner(), "item" | "entry");
                    // The library holds every attribute, too.
                    elements += 1 + element.attributes().with_checks(false).count();
                    entries += usize::from(is_entry);
                    if elements > MAX_ELEMENTS || (is_entry && entries > MAX_ARTICLES) {
                        // The element, or the entry it is part of, as a whole.
                        left_out += usize::from(is_entry || entry.is_some());
                        leaving = Some(entry.take().unwrap_or((depth, start)));
                    } else if is_entry && !empty {
                        entry = Some((depth, start));
                    }
                }
                if !empty {
                    depth += 1;
                }
                empty
            }
            Ok(Event::End(_)) => {
                depth = depth.saturating_sub(1);
                if entry.is_some_and(|(at, _)| at == depth) {
                    entry = None;
                }
                true
            }
            // Further on where it can, as the library may read on, too.
            Err(_) if end > start => false,
            Err(_) => {
                rest = start;
                break;
            }
            Ok(Event::Eof) => break,
            Ok(_) => false,
        };
        if let Some((_, cut)) = leaving.filter(|&(at, _)| closes && at == depth) {
            kept.extend_from_slice(body.get(from..cut).unwrap_or_default());
            from = end;
            leaving = None;
        }
        buf.clear();
    }
    // An element left out that does not end takes the rest with it.
    let rest = leaving.map_or(rest, |(_, cut)| cut.min(rest));
    if from == 0 && rest == body.len() {
        return (Cow::Borrowed(body), 0);
    }
    kept.extend_from_slice(body.get(from..rest).unwrap_or_default());
    (Cow::Owned(kept), left_out)
}

/// What [`Copied`] has read of a JSON Feed so far.
#[derive(Default)]
struct Kept {
    values: usize,
    items: usize,
    left_out: usize,
    changed: bool,
}

/// A value of a JSON Feed written to `out` as far as [`limited`] keeps it:
/// arrays and objects are closed early once the values are used up, and of
/// the feed's `items` only those read in full, up to [`MAX_ARTICLES`].
struct Copied<'a> {
    out: &'a mut Vec<u8>,
    kept: &'a mut Kept,
    /// This is the feed's `items`.
    items: bool,
    /// This is the feed itself.
    top: bool,
}

impl Copied<'_> {
    fn write<E: de::Error>(self, value: impl serde::Serialize) -> Result<(), E> {
        serde_json::to_writer(self.out, &value).map_err(E::custom)
    }

    /// Whether one more value may be read; once not, nothing is anymore.
    fn may_read(&mut self) -> bool {
        self.kept.values += 1;
        let may =
            self.kept.values <= MAX_ELEMENTS && !(self.items && self.kept.items >= MAX_ARTICLES);
        self.kept.changed |= !may;
        may
    }
}

impl<'de> DeserializeSeed<'de> for Copied<'_> {
    type Value = ();

    fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Copied<'_> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("JSON")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<(), E> {
        self.write(value)
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<(), E> {
        self.write(value)
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<(), E> {
        self.write(value)
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<(), E> {
        self.write(value)
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<(), E> {
        self.write(value)
    }

    fn visit_unit<E: de::Error>(self) -> Result<(), E> {
        self.write(())
    }

    fn visit_seq<A: SeqAccess<'de>>(mut self, mut seq: A) -> Result<(), A::Error> {
        self.out.push(b'[');
        let mut first = true;
        loop {
            if !self.may_read() {
                while seq.next_element::<IgnoredAny>()?.is_some() {
                    self.kept.left_out += usize::from(self.items);
                }
                break;
            }
            let mark = self.out.len();
            if !first {
                self.out.push(b',');
            }
            let value = Copied {
                out: self.out,
                kept: self.kept,
                items: false,
                top: false,
            };
            if seq.next_element_seed(value)?.is_none() {
                self.out.truncate(mark);
                break;
            }
            if self.items {
                // An item that was cut short is left out as a whole.
                if self.kept.values > MAX_ELEMENTS {
                    self.out.truncate(mark);
                    self.kept.left_out += 1;
                    continue;
                }
                self.kept.items += 1;
            }
            first = false;
        }
        self.out.push(b']');
        Ok(())
    }

    fn visit_map<A: MapAccess<'de>>(mut self, mut map: A) -> Result<(), A::Error> {
        self.out.push(b'{');
        let mut first = true;
        while let Some(key) = map.next_key::<String>()? {
            // The feed's own members stay, its title among them.
            if !self.top && !self.may_read() {
                map.next_value::<IgnoredAny>()?;
                continue;
            }
            if !first {
                self.out.push(b',');
            }
            first = false;
            serde_json::to_writer(&mut *self.out, &key).map_err(de::Error::custom)?;
            self.out.push(b':');
            map.next_value_seed(Copied {
                out: self.out,
                kept: self.kept,
                items: self.top && key == "items",
                top: false,
            })?;
        }
        self.out.push(b'}');
        Ok(())
    }
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
