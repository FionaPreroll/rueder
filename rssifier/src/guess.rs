//! The guess for a page that says nothing about itself: the entries of a list
//! look alike. After the idea of rss-proxy (now feedless).
//!
//! Every element is known by its path from the top of the page, as tags and
//! classes (`body/main/div.list/article.teaser`); elements with the same path
//! are a group, even when they lie in different parents, as in rows of a grid.
//! An element counts when it holds a link whose text is as long as a title.
//! A group with at least [`MIN_ITEMS`] different links of that kind is a
//! list, and scores for each entry: one, and more for an `<article>`, a
//! `<time>`, a heading around the link and an image. A group in `<main>`
//! scores more, one in `<nav>`, `<header>`, `<footer>` or `<aside>` far less;
//! menus are lists too, but of short words. The group with the most points
//! wins.

use std::{
    collections::{HashMap, HashSet},
    hash::{DefaultHasher, Hash, Hasher},
};

use html5ever::{local_name, ns};
use markup5ever_rcdom::{Handle, NodeData};
use url::Url;

use crate::extract::{
    self, Item, MIN_ITEMS, Parts, attribute, find, is_element, is_heading, is_link, text_of, words,
};

/// The shortest and the longest text of a link that can be a title, in
/// characters.
const TITLE_CHARS: std::ops::RangeInclusive<usize> = 15..=200;
/// The least points a group needs.
const MIN_SCORE: f64 = 3.0;
/// How many elements are looked at, at most.
const MAX_ELEMENTS: usize = 200_000;

/// Where an element lies.
#[derive(Clone, Copy, Default)]
struct Context {
    main: bool,
    aside: bool,
}

/// What an element holds, as far as the guess cares.
#[derive(Clone, Default)]
struct Holds {
    /// The link that can be the title, with the length of its text and
    /// whether a heading is at or around it.
    link: Option<(Handle, usize, bool)>,
    time: bool,
    image: bool,
}

impl Holds {
    fn add(&mut self, other: Self) {
        self.time |= other.time;
        self.image |= other.image;
        // A link in a heading is the title before any other, then the longer.
        let rank = |link: &Option<(Handle, usize, bool)>| {
            link.as_ref().map(|(_, chars, heading)| (*heading, *chars))
        };
        if rank(&other.link) > rank(&self.link) {
            self.link = other.link;
        }
    }
}

/// An element that holds a link that can be a title.
struct Member {
    node: Handle,
    holds: Holds,
    context: Context,
}

#[derive(Default)]
struct Walk {
    /// The members of each group, by the hash of their path, and the groups
    /// in the order they were first seen, so that the same page always gives
    /// the same guess.
    groups: HashMap<u64, Vec<Member>>,
    order: Vec<u64>,
    elements: usize,
}

/// The entries the guess finds, if any group passes.
pub fn items(document: &Handle, base: &Url) -> Vec<Item> {
    let mut walk = Walk::default();
    for child in document.children.borrow().iter() {
        look(child, 0, Context::default(), &mut walk);
    }
    let mut best: Option<(f64, &Vec<Member>)> = None;
    for path in &walk.order {
        let Some(members) = walk.groups.get(path) else {
            continue;
        };
        let score = score(members, base);
        if score >= MIN_SCORE && best.is_none_or(|(most, _)| score > most) {
            best = Some((score, members));
        }
    }
    let Some((_, members)) = best else {
        return Vec::new();
    };
    members
        .iter()
        .filter_map(|member| {
            let (link, _, _) = member.holds.link.clone()?;
            let parts = Parts {
                link: Some(link),
                ..Parts::default()
            };
            extract::block_item(&member.node, parts, base)
        })
        .take(extract::MAX_ITEMS)
        .collect()
}

/// The points of a group: nothing unless it links to enough different pages.
fn score(members: &[Member], base: &Url) -> f64 {
    let mut links = HashSet::new();
    let mut points = 0.0;
    for member in members {
        let Some((link, _, heading)) = &member.holds.link else {
            continue;
        };
        let Some(address) = attribute(link, "href").and_then(|href| extract::link(base, &href))
        else {
            continue;
        };
        if !links.insert(address) {
            continue;
        }
        let mut entry = 1.0;
        if is_element(&member.node, "article") {
            entry += 1.0;
        }
        if member.holds.time {
            entry += 1.0;
        }
        if *heading {
            entry += 1.0;
        }
        if member.holds.image {
            entry += 0.5;
        }
        if member.context.main {
            entry *= 1.5;
        }
        if member.context.aside {
            entry *= 0.2;
        }
        points += entry;
    }
    if links.len() < MIN_ITEMS { 0.0 } else { points }
}

/// Walks the tree below `node`, whose parent's path has the hash `parent`,
/// and says what `node` holds.
fn look(node: &Handle, parent: u64, context: Context, walk: &mut Walk) -> Holds {
    let NodeData::Element { name, .. } = &node.data else {
        return Holds::default();
    };
    walk.elements += 1;
    if name.ns != ns!(html) || walk.elements > MAX_ELEMENTS {
        return Holds::default();
    }
    let mut context = context;
    match name.local {
        local_name!("script")
        | local_name!("style")
        | local_name!("template")
        | local_name!("noscript")
        | local_name!("head") => return Holds::default(),
        local_name!("main") => context.main = true,
        local_name!("nav")
        | local_name!("header")
        | local_name!("footer")
        | local_name!("aside") => context.aside = true,
        _ => {}
    }
    let mut classes: Vec<String> = attribute(node, "class")
        .unwrap_or_default()
        .split_ascii_whitespace()
        .map(str::to_owned)
        .collect();
    classes.sort();
    let mut hasher = DefaultHasher::new();
    (parent, &*name.local, &classes).hash(&mut hasher);
    let path = hasher.finish();
    // Seen here first, before the groups below it: of two groups with the
    // same points, the outer one wins, as its entries hold more.
    walk.groups.entry(path).or_insert_with(|| {
        walk.order.push(path);
        Vec::new()
    });

    let mut holds = Holds::default();
    for child in node.children.borrow().iter() {
        holds.add(look(child, path, context, walk));
    }
    match name.local {
        local_name!("time") => holds.time = true,
        local_name!("img") => holds.image = true,
        _ => {}
    }
    if is_link(node) {
        // A card: the whole entry is the link, and its heading the title.
        let title = find(node, &is_heading).map_or_else(|| text_of(node), |h| text_of(&h));
        let chars = words(&title).chars().count();
        if TITLE_CHARS.contains(&chars) {
            let heading = find(node, &is_heading).is_some();
            holds.link = Some((node.clone(), chars, heading));
        } else {
            holds.link = None;
        }
    }
    if is_heading(node)
        && let Some((_, _, heading)) = &mut holds.link
    {
        *heading = true;
    }
    if holds.link.is_some()
        && let Some(members) = walk.groups.get_mut(&path)
    {
        members.push(Member {
            node: node.clone(),
            holds: holds.clone(),
            context,
        });
    }
    holds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html;

    fn guess(page: &str) -> Vec<Item> {
        let base = Url::parse("https://example.org/").unwrap();
        items(&html::read(page).unwrap(), &base)
    }

    fn menu() -> String {
        let links: String = ["Home", "Über uns", "Termine", "Kontakt", "Impressum"]
            .iter()
            .map(|word| format!(r#"<li><a href="/{word}">{word}</a></li>"#))
            .collect();
        format!("<nav><ul>{links}</ul></nav>")
    }

    #[test]
    fn a_list_of_articles_is_found_past_the_menu_and_the_footer() {
        let article = |n: u32| {
            format!(
                r#"<article class="post"><h2><a href="/posts/{n}">Ein Beitrag mit Nummer {n}</a></h2>
                <time datetime="2026-11-0{n}">gestern</time><img data-src="/i/{n}.jpg">
                <p>Was im Beitrag {n} steht. 12 Kommentare</p></article>"#
            )
        };
        let footer: String = (0..4)
            .map(|n| format!(r#"<p><a href="/legal/{n}">Rechtliche Hinweise Teil {n}</a></p>"#))
            .collect();
        let page = format!(
            "<body>{}<main><div class=grid><div class=row>{}{}</div><div class=row>{}</div></div></main><footer>{footer}</footer></body>",
            menu(),
            article(1),
            article(2),
            article(3)
        );
        let items = guess(&page);
        let titles: Vec<&str> = items.iter().map(|item| item.title.as_str()).collect();
        assert_eq!(
            titles,
            [
                "Ein Beitrag mit Nummer 1",
                "Ein Beitrag mit Nummer 2",
                "Ein Beitrag mit Nummer 3"
            ]
        );
        let first = &items[0];
        assert_eq!(first.url.as_str(), "https://example.org/posts/1");
        assert_eq!(first.summary.as_deref(), Some("Was im Beitrag 1 steht."));
        assert_eq!(
            first.image.as_ref().unwrap().as_str(),
            "https://example.org/i/1.jpg"
        );
        assert_eq!(
            first.published.unwrap().date(),
            time::macros::date!(2026 - 11 - 01)
        );
    }

    #[test]
    fn cards_that_are_links_have_their_heading_as_title() {
        let cards: String = (1..=4)
            .map(|n| {
                format!(
                    r#"<a class=card href="/p/{n}"><h3>Neues Produkt Nummer {n}</h3><p>Seit 3. März 2026 im Laden</p></a>"#
                )
            })
            .collect();
        let items = guess(&format!("<div class=cards>{cards}</div>"));
        assert_eq!(items.len(), 4);
        assert_eq!(items[3].title, "Neues Produkt Nummer 4");
        assert_eq!(
            items[3].summary.as_deref(),
            Some("Seit 3. März 2026 im Laden")
        );
        assert_eq!(
            items[3].published.unwrap().date(),
            time::macros::date!(2026 - 03 - 03)
        );
    }

    #[test]
    fn menus_short_lists_and_one_link_many_times_are_no_list() {
        assert!(guess(&menu()).is_empty());
        let two: String = (1..=2)
            .map(|n| format!(r#"<p><a href="/{n}">Ein langer Titel Nummer {n}</a></p>"#))
            .collect();
        assert!(guess(&two).is_empty());
        let same = r#"<p><a href="/a">Immer derselbe lange Titel</a></p>"#.repeat(5);
        assert!(guess(&same).is_empty());
    }

    #[test]
    fn a_large_page_ends_soon() {
        let blocks: String = (0..20_000)
            .map(|n| {
                format!(r#"<div class=x><a href="/{n}">Ein Titel der lang genug ist</a></div>"#)
            })
            .collect();
        let page = format!("{}{blocks}", "<div>".repeat(200));
        let started = std::time::Instant::now();
        let items = guess(&page);
        assert_eq!(items.len(), extract::MAX_ITEMS);
        assert!(started.elapsed() < std::time::Duration::from_secs(10));
    }
}
