//! What a page lists, as entries of a feed.
//!
//! A recipe for the page's host goes first ([`crate::recipes`]): the operator
//! wrote it for this site. Without one, or when it finds nothing, the first
//! way that finds at least [`MIN_ITEMS`] entries wins, from the most reliable
//! to the guessed:
//!
//! 1. Structured data the page gives about itself: JSON-LD (`ItemList`, and
//!    articles of schema.org in a list or a `@graph`).
//! 2. Microformats: `h-entry` elements, as an `h-feed` has them.
//! 3. The guess from blocks that look alike ([`crate::guess`]).
//!
//! An entry has a title, a link, a short text without markup and maybe an
//! image and a date; nothing that changes from one look to the next, such as
//! "3 hours ago" or a count of comments ([`steady`]), as rueder makes a new
//! version of an article whenever its text changes.

use std::collections::HashSet;

use html5ever::{local_name, ns};
use markup5ever_rcdom::{Handle, NodeData};
use serde_json::Value;
use time::{
    Date, OffsetDateTime, format_description::well_known::Rfc3339, macros::format_description,
};
use url::Url;

use crate::{guess, html, recipes::Recipes};

/// Fewer entries than this are no list, just a page that mentions a few things.
pub const MIN_ITEMS: usize = 3;
/// The most entries a feed gets.
pub const MAX_ITEMS: usize = 100;
/// The longest title, in characters.
pub const MAX_TITLE: usize = 300;
/// The longest text of an entry, in characters: a short summary, not the
/// article.
pub const MAX_SUMMARY: usize = 400;
/// The most bytes of JSON-LD read from one page.
const MAX_JSON_LD: usize = 1024 * 1024;
/// How deep JSON-LD is looked into, and at how many values at most.
const JSON_DEPTH: usize = 12;
const JSON_VALUES: usize = 20_000;
/// A page with less text than this, outside scripts, and with scripts or a
/// `<noscript>` is made in the browser.
const SCRIPTED_TEXT: usize = 200;

/// What a page lists.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Page {
    pub title: String,
    pub items: Vec<Item>,
    /// Where the entries came from: `recipe`, `json-ld`, `h-feed`, `guess`,
    /// or empty.
    pub source: &'static str,
    /// The page has hardly any text but scripts: it is made in the browser.
    pub scripted: bool,
}

/// One entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// The id the site gives the entry, where a recipe reads one; else the
    /// address is the id.
    pub id: Option<String>,
    pub url: Url,
    pub title: String,
    pub summary: Option<String>,
    pub image: Option<Url>,
    pub published: Option<OffsetDateTime>,
}

/// What a page lists. A page beyond the limits of the parser lists nothing.
pub fn page(text: &str, base: &Url, recipes: &Recipes) -> Page {
    let Some(document) = html::read(text) else {
        return Page::default();
    };
    let mut seen = Seen::default();
    look(&document, &mut seen, false);
    let title = clip(
        &seen.og_title.take().unwrap_or_else(|| words(&seen.title)),
        MAX_TITLE,
    );
    let scripted = seen.text < SCRIPTED_TEXT && (seen.scripts > 0 || seen.noscript);
    if let Some(recipe) = recipes.for_page(base) {
        let items = tidy(recipe.items(&document, base), base);
        if !items.is_empty() {
            return Page {
                title,
                items,
                source: "recipe",
                scripted,
            };
        }
    }
    // Each way only when the one before found too little.
    let ways: [(&'static str, &dyn Fn() -> Vec<Item>); 3] = [
        ("json-ld", &|| from_json_ld(&seen.json_ld, base)),
        ("h-feed", &|| from_h_entries(&seen.h_entries, base)),
        ("guess", &|| guess::items(&document, base)),
    ];
    for (source, way) in ways {
        let items = tidy(way(), base);
        if items.len() >= MIN_ITEMS {
            return Page {
                title,
                items,
                source,
                scripted,
            };
        }
    }
    Page {
        title,
        scripted,
        ..Page::default()
    }
}

/// What one walk through the tree collects.
#[derive(Default)]
struct Seen {
    title: String,
    og_title: Option<String>,
    json_ld: Vec<String>,
    json_ld_bytes: usize,
    h_entries: Vec<Handle>,
    /// Characters of text outside scripts and styles.
    text: usize,
    scripts: usize,
    noscript: bool,
}

fn look(node: &Handle, seen: &mut Seen, in_head_title: bool) {
    match &node.data {
        NodeData::Text { contents } => {
            let contents = contents.borrow();
            if in_head_title {
                seen.title.push_str(&contents);
            }
            seen.text += contents.trim().chars().count();
        }
        NodeData::Element { name, .. } => {
            if name.ns != ns!(html) {
                return;
            }
            match name.local {
                local_name!("script") => {
                    let kind = attribute(node, "type").unwrap_or_default();
                    if kind.trim().eq_ignore_ascii_case("application/ld+json") {
                        let text = text_of(node);
                        if seen.json_ld_bytes + text.len() <= MAX_JSON_LD {
                            seen.json_ld_bytes += text.len();
                            seen.json_ld.push(text);
                        }
                    } else {
                        seen.scripts += 1;
                    }
                    return;
                }
                local_name!("style") | local_name!("template") => return,
                local_name!("noscript") => {
                    seen.noscript = true;
                    return;
                }
                local_name!("title") => {
                    for child in node.children.borrow().iter() {
                        look(child, seen, true);
                    }
                    return;
                }
                local_name!("meta") => {
                    if attribute(node, "property").as_deref() == Some("og:title") {
                        seen.og_title = attribute(node, "content").map(|title| words(&title));
                    }
                }
                _ => {}
            }
            if has_class(node, "h-entry") {
                seen.h_entries.push(node.clone());
            }
        }
        _ => {}
    }
    for child in node.children.borrow().iter() {
        look(child, seen, in_head_title);
    }
}

/// The entries of the JSON-LD of a page.
fn from_json_ld(scripts: &[String], base: &Url) -> Vec<Item> {
    let mut items = Vec::new();
    let mut budget = JSON_VALUES;
    for script in scripts {
        // serde_json refuses to nest deeper than 128 levels.
        let Ok(value) = serde_json::from_str::<Value>(script) else {
            continue;
        };
        walk_json(&value, base, 0, &mut budget, &mut items);
    }
    items
}

const ARTICLES: [&str; 9] = [
    "Article",
    "NewsArticle",
    "BlogPosting",
    "Report",
    "ScholarlyArticle",
    "SocialMediaPosting",
    "TechArticle",
    "LiveBlogPosting",
    "Review",
];

fn walk_json(value: &Value, base: &Url, depth: usize, budget: &mut usize, items: &mut Vec<Item>) {
    if depth > JSON_DEPTH || *budget == 0 || items.len() >= MAX_ITEMS {
        return;
    }
    *budget -= 1;
    match value {
        Value::Array(values) => {
            for value in values {
                walk_json(value, base, depth + 1, budget, items);
            }
        }
        Value::Object(object) => {
            let is = |kind: &str| match object.get("@type") {
                Some(Value::String(found)) => found == kind,
                Some(Value::Array(found)) => found.iter().any(|found| found == kind),
                _ => false,
            };
            if ARTICLES.iter().any(|kind| is(kind)) {
                if let Some(item) = json_item(value, base) {
                    items.push(item);
                }
                return;
            }
            if is("ListItem") {
                // A list item is the thing it names, or names it itself.
                match object.get("item") {
                    Some(inner @ Value::Object(_)) => {
                        if let Some(item) = json_item(inner, base) {
                            items.push(item);
                            return;
                        }
                        walk_json(inner, base, depth + 1, budget, items);
                    }
                    _ => {
                        if let Some(item) = json_item(value, base) {
                            items.push(item);
                        }
                    }
                }
                return;
            }
            for value in object.values() {
                walk_json(value, base, depth + 1, budget, items);
            }
        }
        _ => {}
    }
}

/// An entry of an object of schema.org, if it has a link and a title.
fn json_item(value: &Value, base: &Url) -> Option<Item> {
    let text = |key: &str| value.get(key).and_then(Value::as_str);
    let address = text("url")
        .or_else(|| match value.get("mainEntityOfPage") {
            Some(Value::String(url)) => Some(url.as_str()),
            Some(other) => other.get("@id").and_then(Value::as_str),
            None => None,
        })
        .or_else(|| match value.get("item") {
            Some(Value::String(url)) => Some(url.as_str()),
            _ => None,
        })
        .or_else(|| text("@id"))?;
    let title = text("headline").or_else(|| text("name"))?;
    let image = match value.get("image") {
        Some(Value::String(url)) => Some(url.as_str()),
        Some(Value::Array(images)) => images.first().and_then(|image| {
            image
                .as_str()
                .or_else(|| image.get("url").and_then(Value::as_str))
        }),
        Some(image) => image.get("url").and_then(Value::as_str),
        None => None,
    };
    let published = text("datePublished")
        .or_else(|| text("dateCreated"))
        .or_else(|| text("uploadDate"));
    Some(Item {
        id: None,
        url: link(base, address)?,
        title: clip(&words(title), MAX_TITLE),
        summary: text("description").map(|summary| clip(&words(summary), MAX_SUMMARY)),
        image: image.and_then(|image| link(base, image)),
        published: published.and_then(date),
    })
}

/// The entries of the `h-entry` elements of a page.
fn from_h_entries(entries: &[Handle], base: &Url) -> Vec<Item> {
    entries
        .iter()
        .take(MAX_ITEMS)
        .filter_map(|entry| {
            let name = find(entry, &|node| has_class(node, "p-name"));
            let anchor = find(entry, &|node| {
                is_element(node, "a") && attribute(node, "href").is_some()
            });
            let address = find(entry, &|node| has_class(node, "u-url"))
                .or_else(|| anchor.clone())
                .and_then(|node| attribute(&node, "href"))?;
            let title = name.or(anchor).map(|node| words(&text_of(&node)))?;
            if title.is_empty() {
                return None;
            }
            let summary = find(entry, &|node| has_class(node, "p-summary"))
                .or_else(|| find(entry, &|node| has_class(node, "e-content")))
                .map(|node| clip(&words(&text_of(&node)), MAX_SUMMARY));
            let image = find(entry, &|node| {
                has_class(node, "u-photo") || is_element(node, "img")
            })
            .and_then(|node| attribute(&node, "src"));
            let published = find(entry, &|node| has_class(node, "dt-published"))
                .and_then(|node| attribute(&node, "datetime").or_else(|| Some(text_of(&node))));
            Some(Item {
                id: None,
                url: link(base, &address)?,
                title: clip(&title, MAX_TITLE),
                summary: summary.filter(|summary| !summary.is_empty()),
                image: image.and_then(|image| link(base, &image)),
                published: published.as_deref().and_then(date),
            })
        })
        .collect()
}

/// The entries once each, without the page itself, at most [`MAX_ITEMS`].
fn tidy(items: Vec<Item>, base: &Url) -> Vec<Item> {
    let mut seen = HashSet::new();
    items
        .into_iter()
        .filter(|item| item.url != *base && seen.insert(item.url.clone()))
        .take(MAX_ITEMS)
        .collect()
}

/// The parts of a block of the page that make one entry. What is not named
/// is looked for in the block: the link at or around the title, else the
/// first in the block; the title in the heading at or in the link, else the
/// link; the text of the block without the title; the first `<time>` or date
/// in the text; the first image.
#[derive(Default)]
pub(crate) struct Parts {
    pub title: Option<Handle>,
    pub link: Option<Handle>,
    pub summary: Option<Handle>,
    pub date: Option<Handle>,
    pub image: Option<Handle>,
}

const HEADINGS: [&str; 6] = ["h1", "h2", "h3", "h4", "h5", "h6"];

pub(crate) fn is_heading(node: &Handle) -> bool {
    HEADINGS.iter().any(|tag| is_element(node, tag))
}

pub(crate) fn is_link(node: &Handle) -> bool {
    is_element(node, "a") && attribute(node, "href").is_some()
}

/// `node` or the nearest element around it, up to `block`, that passes `test`.
fn around(node: &Handle, block: &Handle, test: impl Fn(&Handle) -> bool) -> Option<Handle> {
    let mut at = Some(node.clone());
    while let Some(node) = at {
        if test(&node) {
            return Some(node);
        }
        if std::rc::Rc::ptr_eq(&node, block) {
            return None;
        }
        at = crate::select::parent(&node);
    }
    None
}

/// The entry a block of the page makes, if it has a link and a title.
pub(crate) fn block_item(block: &Handle, parts: Parts, base: &Url) -> Option<Item> {
    let anchor = parts
        .link
        .or_else(|| {
            let title = parts.title.as_ref()?;
            around(title, block, is_link).or_else(|| find(title, &is_link))
        })
        .or_else(|| around(block, block, is_link))
        .or_else(|| find(block, &is_link))?;
    let address = attribute(&anchor, "href")?;
    let title_node = parts
        .title
        .or_else(|| around(&anchor, block, is_heading))
        .or_else(|| find(&anchor, &is_heading))
        .unwrap_or_else(|| anchor.clone());
    let title = words(&text_of(&title_node));
    if title.is_empty() {
        return None;
    }
    let text = match &parts.summary {
        Some(summary) => words(&text_of(summary)),
        None => words(&text_without(block, &title_node)),
    };
    let summary = clip(&steady(&text), MAX_SUMMARY);
    let published = match &parts.date {
        Some(node) => attribute(node, "datetime")
            .as_deref()
            .and_then(date)
            .or_else(|| date_in_text(&text_of(node))),
        None => find(block, &|node| {
            is_element(node, "time") && attribute(node, "datetime").is_some()
        })
        .and_then(|node| attribute(&node, "datetime"))
        .as_deref()
        .and_then(date)
        .or_else(|| date_in_text(&text_of(block))),
    };
    let image = parts
        .image
        .or_else(|| find(block, &|node| is_element(node, "img")))
        .and_then(|node| {
            ["src", "data-src"]
                .iter()
                .filter_map(|name| attribute(&node, name))
                .find_map(|address| link(base, &address))
        });
    Some(Item {
        id: None,
        url: link(base, &address)?,
        summary: (!summary.is_empty() && summary != title).then_some(summary),
        title: clip(&title, MAX_TITLE),
        image,
        published,
    })
}

/// The text under `block` without `left_out` and without what tells a time.
fn text_without(block: &Handle, left_out: &Handle) -> String {
    fn collect(node: &Handle, left_out: &Handle, text: &mut String) {
        for child in node.children.borrow().iter() {
            match &child.data {
                NodeData::Text { contents } => text.push_str(&contents.borrow()),
                NodeData::Element { name, .. }
                    if !std::rc::Rc::ptr_eq(child, left_out)
                        && !matches!(
                            name.local,
                            local_name!("script")
                                | local_name!("style")
                                | local_name!("time")
                                | local_name!("button")
                        ) =>
                {
                    collect(child, left_out, text);
                    text.push(' ');
                }
                _ => {}
            }
        }
    }
    let mut text = String::new();
    collect(block, left_out, &mut text);
    text
}

/// Words that say how long ago something was.
const UNITS: [&str; 40] = [
    "s", "sec", "secs", "second", "seconds", "m", "min", "mins", "minute", "minutes", "h", "hr",
    "hrs", "hour", "hours", "d", "day", "days", "week", "weeks", "month", "months", "year",
    "years", "sekunde", "sekunden", "minuten", "std", "stunde", "stunden", "tag", "tage", "tagen",
    "woche", "wochen", "monat", "monate", "monaten", "jahr", "jahren",
];
/// Words that follow a count that grows.
const COUNTED: [&str; 24] = [
    "comment",
    "comments",
    "kommentar",
    "kommentare",
    "point",
    "points",
    "punkt",
    "punkte",
    "like",
    "likes",
    "reply",
    "replies",
    "antwort",
    "antworten",
    "view",
    "views",
    "aufruf",
    "aufrufe",
    "share",
    "shares",
    "vote",
    "votes",
    "stimme",
    "stimmen",
];

/// Text without what changes from one look at the page to the next: "3 hours
/// ago", "vor einer Stunde", "12 comments". rueder would make a new version
/// of the entry each time.
pub(crate) fn steady(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    let plain = |i: usize| {
        words.get(i).map(|word| {
            word.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
    };
    let is = |i: usize, list: &[&str]| plain(i).is_some_and(|word| list.contains(&word.as_str()));
    let count = |i: usize| {
        plain(i).is_some_and(|word| {
            ["a", "an", "ein", "eine", "einer", "einem", "wenigen"].contains(&word.as_str())
                || (word.starts_with(|c: char| c.is_ascii_digit())
                    && word
                        .chars()
                        .all(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | 'k')))
        })
    };
    // "3h", "5min": a count and a unit in one word.
    let glued = |i: usize| {
        plain(i).is_some_and(|word| {
            let unit = word.trim_start_matches(|c: char| c.is_ascii_digit());
            unit.len() < word.len() && UNITS.contains(&unit)
        })
    };
    let mut kept = Vec::with_capacity(words.len());
    let mut i = 0;
    while i < words.len() {
        let three = (is(i, &["vor"]) && count(i + 1) && is(i + 2, &UNITS))
            || (count(i) && is(i + 1, &UNITS) && is(i + 2, &["ago"]));
        let two = (glued(i) && is(i + 1, &["ago"]))
            || (count(i) && is(i + 1, &COUNTED))
            || (is(i, &["just"]) && is(i + 1, &["now"]))
            || (is(i, &["gerade"]) && is(i + 1, &["eben"]));
        let skip = if three {
            3
        } else if two {
            2
        } else {
            0
        };
        if skip == 0 {
            kept.push(words[i]);
            i += 1;
        } else {
            i += skip;
        }
    }
    kept.join(" ")
}

const MONTHS: [(&str, u8); 37] = [
    ("jan", 1),
    ("januar", 1),
    ("january", 1),
    ("feb", 2),
    ("februar", 2),
    ("february", 2),
    ("mar", 3),
    ("mär", 3),
    ("märz", 3),
    ("march", 3),
    ("apr", 4),
    ("april", 4),
    ("mai", 5),
    ("may", 5),
    ("jun", 6),
    ("juni", 6),
    ("june", 6),
    ("jul", 7),
    ("juli", 7),
    ("july", 7),
    ("aug", 8),
    ("august", 8),
    ("sep", 9),
    ("sept", 9),
    ("september", 9),
    ("okt", 10),
    ("oct", 10),
    ("oktober", 10),
    ("october", 10),
    ("nov", 11),
    ("november", 11),
    ("dez", 12),
    ("dec", 12),
    ("dezember", 12),
    ("december", 12),
    ("maerz", 3),
    ("jänner", 1),
];
/// How many words of a text are looked through for a date.
const DATE_WORDS: usize = 300;

/// The first date in a text as people write it: `2026-11-20`, `20.11.2026`,
/// `20. November 2026`, `20 Nov 2026`, `November 20, 2026`.
pub(crate) fn date_in_text(text: &str) -> Option<OffsetDateTime> {
    let words: Vec<String> = text
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|word| !word.is_empty())
        .take(DATE_WORDS)
        .map(str::to_lowercase)
        .collect();
    let day = |word: &str| {
        let word = word.trim_end_matches('.');
        let word = ["st", "nd", "rd", "th"]
            .iter()
            .find_map(|end| word.strip_suffix(end))
            .unwrap_or(word);
        word.parse::<u8>().ok().filter(|day| (1..=31).contains(day))
    };
    let month = |word: &str| {
        let word = word.trim_end_matches('.');
        MONTHS
            .iter()
            .find(|(name, _)| *name == word)
            .map(|(_, month)| *month)
    };
    let year = |word: &str| {
        let word = word.trim_end_matches(|c: char| !c.is_ascii_digit());
        (word.len() == 4)
            .then(|| word.parse::<i32>().ok())
            .flatten()
            .filter(|year| (1990..=2100).contains(year))
    };
    let made = |year: i32, month: u8, day: u8| {
        let month = time::Month::try_from(month).ok()?;
        Some(
            Date::from_calendar_date(year, month, day)
                .ok()?
                .midnight()
                .assume_utc(),
        )
    };
    let at = |i: usize| words.get(i).map(String::as_str).unwrap_or_default();
    for i in 0..words.len() {
        let word = at(i).trim_matches(|c: char| !c.is_ascii_digit());
        let found = if word.len() >= 10 && word.as_bytes().get(4) == Some(&b'-') {
            date(word)
        } else if let [d, m, y] = word.split('.').collect::<Vec<_>>()[..]
            && let (Some(d), Ok(m), Some(y)) = (day(d), m.parse::<u8>(), year(y))
        {
            made(y, m, d)
        } else if let (Some(d), Some(m), Some(y)) = (day(at(i)), month(at(i + 1)), year(at(i + 2)))
        {
            made(y, m, d)
        } else if let (Some(m), Some(d), Some(y)) = (month(at(i)), day(at(i + 1)), year(at(i + 2)))
        {
            made(y, m, d)
        } else {
            None
        };
        if found.is_some() {
            return found;
        }
    }
    None
}

/// An address as the page wrote it, made whole: only http and https, without
/// the part after `#` and without the parameters that only track.
pub fn link(base: &Url, address: &str) -> Option<Url> {
    let mut url = base.join(address.trim()).ok()?;
    if !matches!(url.scheme(), "http" | "https") || url.host().is_none() {
        return None;
    }
    url.set_fragment(None);
    let tracking = |name: &str| name.starts_with("utm_") || name == "fbclid" || name == "gclid";
    if url.query_pairs().any(|(name, _)| tracking(&name)) {
        let kept: Vec<(String, String)> = url
            .query_pairs()
            .filter(|(name, _)| !tracking(name))
            .map(|(name, value)| (name.into_owned(), value.into_owned()))
            .collect();
        if kept.is_empty() {
            url.set_query(None);
        } else {
            url.query_pairs_mut().clear().extend_pairs(kept);
        }
    }
    Some(url)
}

/// A date as pages write it for machines: RFC 3339, also with the offset
/// without a colon (`+0000`), or just the day.
pub(crate) fn date(text: &str) -> Option<OffsetDateTime> {
    let text = text.trim();
    if let Ok(at) = OffsetDateTime::parse(text, &Rfc3339) {
        return Some(at);
    }
    if let Some((time, offset)) = text.split_at_checked(text.len().saturating_sub(4))
        && offset.bytes().all(|b| b.is_ascii_digit())
        && time.ends_with(['+', '-'])
        && let Ok(at) = OffsetDateTime::parse(
            &format!("{time}{}:{}", &offset[..2], &offset[2..]),
            &Rfc3339,
        )
    {
        return Some(at);
    }
    let day = Date::parse(text.get(..10)?, format_description!("[year]-[month]-[day]")).ok()?;
    Some(day.midnight().assume_utc())
}

pub(crate) fn is_element(node: &Handle, tag: &str) -> bool {
    matches!(&node.data, NodeData::Element { name, .. } if &*name.local == tag)
}

pub(crate) fn attribute(node: &Handle, wanted: &str) -> Option<String> {
    let NodeData::Element { attrs, .. } = &node.data else {
        return None;
    };
    let attrs = attrs.borrow();
    attrs
        .iter()
        .find(|attr| &*attr.name.local == wanted)
        .map(|attr| attr.value.to_string())
}

pub(crate) fn has_class(node: &Handle, class: &str) -> bool {
    attribute(node, "class")
        .is_some_and(|classes| classes.split_ascii_whitespace().any(|c| c == class))
}

/// The first element under `node` (itself left out) that passes `test`.
pub(crate) fn find(node: &Handle, test: &dyn Fn(&Handle) -> bool) -> Option<Handle> {
    for child in node.children.borrow().iter() {
        if matches!(child.data, NodeData::Element { .. }) {
            if test(child) {
                return Some(child.clone());
            }
            if let Some(found) = find(child, test) {
                return Some(found);
            }
        }
    }
    None
}

/// The text under a node, without scripts and styles.
pub(crate) fn text_of(node: &Handle) -> String {
    let mut text = String::new();
    collect_text(node, &mut text);
    text
}

fn collect_text(node: &Handle, text: &mut String) {
    for child in node.children.borrow().iter() {
        match &child.data {
            NodeData::Text { contents } => text.push_str(&contents.borrow()),
            NodeData::Element { name, .. }
                if !matches!(name.local, local_name!("script") | local_name!("style")) =>
            {
                collect_text(child, text);
                // Blocks are separated in the text, as the browser shows them.
                text.push(' ');
            }
            _ => {}
        }
    }
}

/// Text with every run of white space as one space.
pub(crate) fn words(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// At most `most` characters, with "…" where it was cut.
pub(crate) fn clip(text: &str, most: usize) -> String {
    match text.char_indices().nth(most) {
        None => text.to_owned(),
        Some((end, _)) => format!("{}…", text[..end].trim_end()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Url {
        Url::parse("https://example.org/news/").unwrap()
    }

    #[test]
    fn json_ld_lists_articles_with_dates_and_images() {
        let html = r#"<html><head><title>News</title>
            <script type="application/ld+json">{"@context":"https://schema.org","@type":"ItemList",
              "itemListElement":[
                {"@type":"ListItem","position":1,"item":{"@type":"NewsArticle","headline":"One",
                  "url":"/news/1?utm_source=x","datePublished":"2026-11-20T10:00:00+01:00",
                  "image":{"@type":"ImageObject","url":"/img/1.jpg"},"description":"  First   story "}},
                {"@type":"ListItem","position":2,"url":"https://example.org/news/2","name":"Two"},
                {"@type":"ListItem","position":3,"item":{"@type":"BlogPosting","headline":"Three",
                  "mainEntityOfPage":{"@id":"https://example.org/news/3"},"datePublished":"2026-11-18"}}
              ]}</script></head><body><p>Hello</p></body></html>"#;
        let page = page(html, &base(), &Recipes::default());
        assert_eq!(page.source, "json-ld");
        assert_eq!(page.title, "News");
        let one = &page.items[0];
        assert_eq!(one.url.as_str(), "https://example.org/news/1");
        assert_eq!(one.title, "One");
        assert_eq!(one.summary.as_deref(), Some("First story"));
        assert_eq!(
            one.image.as_ref().unwrap().as_str(),
            "https://example.org/img/1.jpg"
        );
        assert_eq!(one.published.unwrap().unix_timestamp(), 1_795_165_200);
        assert_eq!(page.items[1].title, "Two");
        assert_eq!(page.items[2].url.as_str(), "https://example.org/news/3");
        assert_eq!(
            page.items[2].published.unwrap().unix_timestamp(),
            1_794_960_000
        );
    }

    #[test]
    fn the_article_of_the_page_itself_is_no_entry() {
        let html = r#"<script type="application/ld+json">[
            {"@type":"Article","headline":"This page","url":"https://example.org/news/"},
            {"@type":"Article","headline":"A","url":"/a"},
            {"@type":"Article","headline":"B","url":"/b"}]</script>"#;
        let page = page(html, &base(), &Recipes::default());
        assert!(page.items.is_empty(), "{:?}", page.items);
    }

    #[test]
    fn h_entries_give_title_link_date_and_summary() {
        let entry = |n: u32| {
            format!(
                r#"<article class="h-entry"><h2 class="p-name"><a class="u-url" href="/post/{n}">Post {n}</a></h2>
                <time class="dt-published" datetime="2026-11-1{n}T08:00:00Z">yesterday</time>
                <p class="p-summary">Summary {n}</p><img src="/p{n}.png"></article>"#
            )
        };
        let html = format!(
            "<title> Blog </title><main class=h-feed>{}{}{}<a href=/about>About</a></main>",
            entry(1),
            entry(2),
            entry(3)
        );
        let page = page(&html, &base(), &Recipes::default());
        assert_eq!(page.source, "h-feed");
        assert_eq!(page.title, "Blog");
        assert_eq!(page.items.len(), 3);
        let first = &page.items[0];
        assert_eq!(first.url.as_str(), "https://example.org/post/1");
        assert_eq!(first.title, "Post 1");
        assert_eq!(first.summary.as_deref(), Some("Summary 1"));
        assert_eq!(
            first.image.as_ref().unwrap().as_str(),
            "https://example.org/p1.png"
        );
        assert!(first.published.is_some());
    }

    #[test]
    fn a_page_without_a_list_lists_nothing_and_one_made_by_scripts_says_so() {
        let plain = page(
            "<title>Hi</title><p>Just a page with <a href=/a>a link</a>.</p>",
            &base(),
            &Recipes::default(),
        );
        assert!(plain.items.is_empty());
        assert!(!plain.scripted);
        let scripted = page(
            r#"<div id="app"></div><noscript>Enable JavaScript</noscript><script src="/app.js"></script>"#,
            &base(),
            &Recipes::default(),
        );
        assert!(scripted.items.is_empty());
        assert!(scripted.scripted);
    }

    #[test]
    fn a_page_beyond_the_limits_of_the_parser_lists_nothing_and_ends_soon() {
        let deep = "<div>".repeat(100_000);
        let started = std::time::Instant::now();
        assert_eq!(page(&deep, &base(), &Recipes::default()), Page::default());
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
    }

    #[test]
    fn json_ld_nested_deeply_or_broken_is_left_alone() {
        let deep = format!(
            r#"<script type="application/ld+json">{}{}</script>"#,
            "[".repeat(10_000),
            "]".repeat(10_000)
        );
        assert!(page(&deep, &base(), &Recipes::default()).items.is_empty());
        assert!(
            page(
                r#"<script type="application/ld+json">{"@type":</script>"#,
                &base(),
                &Recipes::default(),
            )
            .items
            .is_empty()
        );
    }

    #[test]
    fn links_are_whole_and_without_tracking_and_only_to_the_web() {
        let base = base();
        assert_eq!(
            link(&base, "a?utm_medium=x&id=3#top").unwrap().as_str(),
            "https://example.org/news/a?id=3"
        );
        assert_eq!(
            link(&base, "/b?fbclid=1").unwrap().as_str(),
            "https://example.org/b"
        );
        assert!(link(&base, "javascript:alert(1)").is_none());
        assert!(link(&base, "mailto:a@example.org").is_none());
    }

    #[test]
    fn what_changes_from_look_to_look_is_left_out() {
        assert_eq!(
            steady("Neu: Grillfest vor 3 Stunden · 12 Kommentare (1.2k likes)"),
            "Neu: Grillfest ·"
        );
        assert_eq!(steady("posted an hour ago, 5h ago, just now"), "posted");
        assert_eq!(steady("vor einer Woche"), "");
        // What only looks like it stays.
        assert_eq!(
            steady("Vor dem Rathaus 3 Tage Fest"),
            "Vor dem Rathaus 3 Tage Fest"
        );
    }

    #[test]
    fn dates_are_read_as_people_write_them() {
        let day = |text: &str| date_in_text(text).map(|at| at.date().to_string());
        for (text, wanted) in [
            ("am 20.11.2026 um 10 Uhr", "2026-11-20"),
            ("Freitag, 20. November 2026", "2026-11-20"),
            ("Posted Nov 20th, 2026", "2026-11-20"),
            ("20 Nov. 2026", "2026-11-20"),
            ("on 2026-11-20T10:00", "2026-11-20"),
            ("3. März 2026", "2026-03-03"),
        ] {
            assert_eq!(day(text).as_deref(), Some(wanted), "{text}");
        }
        for text in [
            "31.02.2026",
            "20. Smarch 2026",
            "Version 1.2.3",
            "20 Nov 26",
            "",
        ] {
            assert_eq!(day(text), None, "{text}");
        }
    }

    #[test]
    fn long_texts_are_clipped_on_a_character() {
        assert_eq!(clip("äöü", 2), "äö…");
        assert_eq!(clip("abc", 3), "abc");
    }
}
