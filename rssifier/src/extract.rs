//! What a page lists, as entries of a feed.
//!
//! The first way that finds at least [`MIN_ITEMS`] entries wins, from the most
//! reliable to the guessed:
//!
//! 1. Structured data the page gives about itself: JSON-LD (`ItemList`, and
//!    articles of schema.org in a list or a `@graph`).
//! 2. Microformats: `h-entry` elements, as an `h-feed` has them.
//!
//! Rules for single hosts and a guess from repeated blocks follow in a later
//! step. An entry has a title, a link, a short text without markup and maybe
//! an image and a date; nothing that changes from one look to the next, such
//! as "3 hours ago" or a count of comments, as rueder makes a new version of
//! an article whenever its text changes.

use std::collections::HashSet;

use html5ever::{local_name, ns};
use markup5ever_rcdom::{Handle, NodeData};
use serde_json::Value;
use time::{
    Date, OffsetDateTime, format_description::well_known::Rfc3339, macros::format_description,
};
use url::Url;

use crate::html;

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
    /// Where the entries came from: `json-ld`, `h-feed`, or empty.
    pub source: &'static str,
    /// The page has hardly any text but scripts: it is made in the browser.
    pub scripted: bool,
}

/// One entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub url: Url,
    pub title: String,
    pub summary: Option<String>,
    pub image: Option<Url>,
    pub published: Option<OffsetDateTime>,
}

/// What a page lists. A page beyond the limits of the parser lists nothing.
pub fn page(text: &str, base: &Url) -> Page {
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
    let ways: [(&'static str, Vec<Item>); 2] = [
        ("json-ld", from_json_ld(&seen.json_ld, base)),
        ("h-feed", from_h_entries(&seen.h_entries, base)),
    ];
    for (source, items) in ways {
        let items = tidy(items, base);
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

/// A date as pages write it for machines: RFC 3339, or just the day.
fn date(text: &str) -> Option<OffsetDateTime> {
    let text = text.trim();
    if let Ok(at) = OffsetDateTime::parse(text, &Rfc3339) {
        return Some(at);
    }
    let day = Date::parse(text.get(..10)?, format_description!("[year]-[month]-[day]")).ok()?;
    Some(day.midnight().assume_utc())
}

fn is_element(node: &Handle, tag: &str) -> bool {
    matches!(&node.data, NodeData::Element { name, .. } if &*name.local == tag)
}

fn attribute(node: &Handle, wanted: &str) -> Option<String> {
    let NodeData::Element { attrs, .. } = &node.data else {
        return None;
    };
    let attrs = attrs.borrow();
    attrs
        .iter()
        .find(|attr| &*attr.name.local == wanted)
        .map(|attr| attr.value.to_string())
}

fn has_class(node: &Handle, class: &str) -> bool {
    attribute(node, "class")
        .is_some_and(|classes| classes.split_ascii_whitespace().any(|c| c == class))
}

/// The first element under `node` (itself left out) that passes `test`.
fn find(node: &Handle, test: &dyn Fn(&Handle) -> bool) -> Option<Handle> {
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
fn text_of(node: &Handle) -> String {
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
fn words(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// At most `most` characters, with "…" where it was cut.
fn clip(text: &str, most: usize) -> String {
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
        let page = page(html, &base());
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
        let page = page(html, &base());
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
        let page = page(&html, &base());
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
        );
        assert!(plain.items.is_empty());
        assert!(!plain.scripted);
        let scripted = page(
            r#"<div id="app"></div><noscript>Enable JavaScript</noscript><script src="/app.js"></script>"#,
            &base(),
        );
        assert!(scripted.items.is_empty());
        assert!(scripted.scripted);
    }

    #[test]
    fn a_page_beyond_the_limits_of_the_parser_lists_nothing_and_ends_soon() {
        let deep = "<div>".repeat(100_000);
        let started = std::time::Instant::now();
        assert_eq!(page(&deep, &base()), Page::default());
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
    }

    #[test]
    fn json_ld_nested_deeply_or_broken_is_left_alone() {
        let deep = format!(
            r#"<script type="application/ld+json">{}{}</script>"#,
            "[".repeat(10_000),
            "]".repeat(10_000)
        );
        assert!(page(&deep, &base()).items.is_empty());
        assert!(
            page(
                r#"<script type="application/ld+json">{"@type":</script>"#,
                &base()
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
    fn long_texts_are_clipped_on_a_character() {
        assert_eq!(clip("äöü", 2), "äö…");
        assert_eq!(clip("abc", 3), "abc");
    }
}
