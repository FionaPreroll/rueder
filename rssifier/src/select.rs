//! The part of CSS selectors the recipes need: tags, classes, ids and
//! attributes, put together in one compound (`article.teaser[data-id]`),
//! descendants of each other (`main article h2`), and lists of them
//! (`h2, h3`). Nothing that needs a browser, such as `:hover`, and no `>`:
//! a descendant is enough to point into a page, and it is matched without
//! looking back, so no selector can make a page expensive.

use std::fmt;

use markup5ever_rcdom::{Handle, NodeData};

/// A selector that was not written right, with what is wrong.
#[derive(Debug, PartialEq, Eq)]
pub struct Invalid(pub String);

impl fmt::Display for Invalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Invalid {}

/// A list of selectors, one of which has to match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector(Vec<Chain>);

/// Compounds, each a descendant of the one before.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Chain(Vec<Compound>);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Compound {
    tag: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
    /// An attribute that is there, or that has exactly this value.
    attributes: Vec<(String, Option<String>)>,
}

/// How many nodes [`Selector::all`] looks at, at most, below one element.
const MAX_LOOKED_AT: usize = 200_000;

impl Selector {
    pub fn parse(text: &str) -> Result<Self, Invalid> {
        let chains = text
            .split(',')
            .map(|chain| {
                let compounds = chain
                    .split_ascii_whitespace()
                    .map(compound)
                    .collect::<Result<Vec<_>, _>>()?;
                if compounds.is_empty() {
                    return Err(Invalid(format!("an empty selector in {text:?}")));
                }
                Ok(Chain(compounds))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self(chains))
    }

    /// Whether `node` matches.
    pub fn matches(&self, node: &Handle) -> bool {
        self.0.iter().any(|chain| chain.matches(node))
    }

    /// The first element below `scope`, in the order of the page, that
    /// matches.
    pub fn first(&self, scope: &Handle) -> Option<Handle> {
        self.all(scope, 1).pop()
    }

    /// The elements below `scope` that match, in the order of the page, at
    /// most `most`. What lies below an element that matches is not looked
    /// at, so the elements found never hold each other.
    pub fn all(&self, scope: &Handle, most: usize) -> Vec<Handle> {
        let mut found = Vec::new();
        let mut looked_at = 0;
        let mut stack: Vec<Handle> = children(scope).into_iter().rev().collect();
        while let Some(node) = stack.pop() {
            looked_at += 1;
            if found.len() >= most || looked_at > MAX_LOOKED_AT {
                break;
            }
            if !matches!(node.data, NodeData::Element { .. }) {
                continue;
            }
            if self.matches(&node) {
                found.push(node);
            } else {
                stack.extend(children(&node).into_iter().rev());
            }
        }
        found
    }
}

impl Chain {
    fn matches(&self, node: &Handle) -> bool {
        let Some((last, before)) = self.0.split_last() else {
            return false;
        };
        if !last.matches(node) {
            return false;
        }
        // Each compound before matches the nearest ancestor it can: that
        // leaves the most ancestors for those before it, so nothing has to be
        // tried twice.
        let mut wanted = before.iter().rev().peekable();
        let mut up = parent(node);
        while let (Some(compound), Some(ancestor)) = (wanted.peek(), up) {
            if compound.matches(&ancestor) {
                wanted.next();
            }
            up = parent(&ancestor);
        }
        wanted.peek().is_none()
    }
}

impl Compound {
    fn matches(&self, node: &Handle) -> bool {
        let NodeData::Element { name, attrs, .. } = &node.data else {
            return false;
        };
        if self.tag.as_deref().is_some_and(|tag| *name.local != *tag) {
            return false;
        }
        let attrs = attrs.borrow();
        let value = |wanted: &str| {
            attrs
                .iter()
                .find(|attr| *attr.name.local == *wanted)
                .map(|attr| &*attr.value)
        };
        if self
            .id
            .as_deref()
            .is_some_and(|id| value("id").is_none_or(|found| found != id))
        {
            return false;
        }
        if !self.classes.is_empty() {
            let classes = value("class").unwrap_or_default();
            if !self
                .classes
                .iter()
                .all(|class| classes.split_ascii_whitespace().any(|c| c == class))
            {
                return false;
            }
        }
        self.attributes
            .iter()
            .all(|(name, wanted)| match (value(name), wanted) {
                (None, _) => false,
                (Some(_), None) => true,
                (Some(found), Some(wanted)) => found == wanted,
            })
    }
}

/// One compound, such as `a.more[rel=bookmark]`.
fn compound(text: &str) -> Result<Compound, Invalid> {
    let invalid = || Invalid(format!("{text:?} is not a selector this knows"));
    let mut compound = Compound::default();
    let mut rest = text;
    let name_end = |text: &str| {
        text.find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .unwrap_or(text.len())
    };
    let end = name_end(rest);
    if end > 0 {
        compound.tag = Some(rest[..end].to_ascii_lowercase());
        rest = &rest[end..];
    } else if let Some(after) = rest.strip_prefix('*') {
        rest = after;
    }
    while let Some(first) = rest.chars().next() {
        let after = &rest[first.len_utf8()..];
        match first {
            '.' | '#' => {
                let end = name_end(after);
                if end == 0 {
                    return Err(invalid());
                }
                let name = after[..end].to_owned();
                if first == '.' {
                    compound.classes.push(name);
                } else {
                    compound.id = Some(name);
                }
                rest = &after[end..];
            }
            '[' => {
                let end = after.find(']').ok_or_else(invalid)?;
                let inside = &after[..end];
                let (name, value) = match inside.split_once('=') {
                    Some((name, value)) => {
                        let value = value.trim();
                        let unquoted = value
                            .strip_prefix('"')
                            .and_then(|v| v.strip_suffix('"'))
                            .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
                            .unwrap_or(value);
                        (name.trim(), Some(unquoted.to_owned()))
                    }
                    None => (inside.trim(), None),
                };
                if name.is_empty() || name_end(name) != name.len() {
                    return Err(invalid());
                }
                compound.attributes.push((name.to_ascii_lowercase(), value));
                rest = &after[end + 1..];
            }
            _ => return Err(invalid()),
        }
    }
    Ok(compound)
}

fn children(node: &Handle) -> Vec<Handle> {
    node.children.borrow().clone()
}

/// The element `node` lies in, if any.
pub fn parent(node: &Handle) -> Option<Handle> {
    let weak = node.parent.take();
    let parent = weak.as_ref().and_then(std::rc::Weak::upgrade);
    node.parent.set(weak);
    parent.filter(|parent| matches!(parent.data, NodeData::Element { .. }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html;

    fn texts(selector: &str, page: &str) -> Vec<String> {
        let document = html::read(page).unwrap();
        Selector::parse(selector)
            .unwrap()
            .all(&document, 100)
            .iter()
            .map(|node| {
                let mut text = String::new();
                for child in node.children.borrow().iter() {
                    if let NodeData::Text { contents } = &child.data {
                        text.push_str(&contents.borrow());
                    }
                }
                text
            })
            .collect()
    }

    const PAGE: &str = r#"<main id=news><article class="teaser big"><h2 data-id=1>One</h2></article>
        <article class=teaser><h2>Two</h2><h3>Three</h3></article></main>
        <aside><article class=teaser><h2>Four</h2></article></aside>"#;

    #[test]
    fn compounds_descendants_and_lists_match_as_in_css() {
        assert_eq!(texts("h2", PAGE), ["One", "Two", "Four"]);
        assert_eq!(texts("main h2", PAGE), ["One", "Two"]);
        assert_eq!(texts("#news .teaser h2", PAGE), ["One", "Two"]);
        assert_eq!(texts("article.teaser.big h2", PAGE), ["One"]);
        assert_eq!(texts("h2[data-id]", PAGE), ["One"]);
        assert_eq!(texts("[data-id='1']", PAGE), ["One"]);
        assert_eq!(texts("h2[data-id=2]", PAGE), Vec::<String>::new());
        assert_eq!(texts("main h3, aside h2", PAGE), ["Three", "Four"]);
        assert_eq!(texts("body * h3", PAGE), ["Three"]);
        assert_eq!(texts("aside main h2", PAGE), Vec::<String>::new());
    }

    #[test]
    fn what_this_does_not_know_is_refused() {
        for wrong in [
            "", "a,", "a > b", "a:hover", "a[", "a[=x]", ".", "a..b", "ä",
        ] {
            assert!(Selector::parse(wrong).is_err(), "{wrong:?}");
        }
    }

    #[test]
    fn a_long_chain_on_a_deep_page_ends_soon() {
        let page = "<div>".repeat(250) + &"<p>x</p>".repeat(1000);
        let document = html::read(&page).unwrap();
        let selector = Selector::parse(&format!("span {} p", "div ".repeat(40))).unwrap();
        let started = std::time::Instant::now();
        assert!(selector.all(&document, 100).is_empty());
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
    }
}
