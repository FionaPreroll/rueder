//! Recipes: how to read the pages of one site, as data. The ones that come
//! with RSSifier are in `rules/recipes.toml`, which also says how they are
//! written; the operator can add a file of more (`RSSIFIER_RECIPES`).

use std::path::Path;

use anyhow::Context;
use markup5ever_rcdom::Handle;
use serde::Deserialize;
use url::Url;

use crate::{
    extract::{self, Item, MAX_ITEMS, Parts},
    select::Selector,
};

/// A recipe as it is written.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Written {
    host: String,
    #[serde(default)]
    path: String,
    item: String,
    title: Option<String>,
    link: Option<String>,
    summary: Option<String>,
    date: Option<String>,
    image: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    recipe: Vec<Written>,
}

/// How to read the pages of one host, or of a part of it.
#[derive(Debug)]
pub struct Recipe {
    host: String,
    path: String,
    item: Selector,
    title: Option<Selector>,
    link: Option<Selector>,
    summary: Option<Selector>,
    date: Option<Selector>,
    image: Option<Selector>,
}

/// All recipes, those that go first first.
#[derive(Debug, Default)]
pub struct Recipes(Vec<Recipe>);

impl Recipes {
    /// The recipes of a file written like `rules/recipes.toml`.
    pub fn parse(text: &str) -> anyhow::Result<Self> {
        let file: File = toml::from_str(text)?;
        let recipes = file
            .recipe
            .into_iter()
            .map(|written| {
                let host = written
                    .host
                    .trim()
                    .trim_end_matches('.')
                    .to_ascii_lowercase();
                Recipe::of(written, host.clone()).with_context(|| format!("the recipe for {host}"))
            })
            .collect::<anyhow::Result<_>>()?;
        Ok(Self(recipes))
    }

    /// The recipes that come with RSSifier.
    pub fn built_in() -> anyhow::Result<Self> {
        Self::parse(include_str!("../rules/recipes.toml")).context("rules/recipes.toml")
    }

    /// The operator's recipes from `file`, before those that come with
    /// RSSifier.
    pub fn with_file(file: &Path) -> anyhow::Result<Self> {
        let text =
            std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
        let mut recipes = Self::parse(&text).with_context(|| file.display().to_string())?;
        recipes.0.extend(Self::built_in()?.0);
        Ok(recipes)
    }

    /// The recipe for a page: of those for its host, the one with the
    /// longest path that fits; the first of them if two are as long.
    pub fn for_page(&self, page: &Url) -> Option<&Recipe> {
        let host = page.host_str()?.trim_end_matches('.');
        let mut best: Option<&Recipe> = None;
        for recipe in &self.0 {
            let fits = host
                .strip_suffix(recipe.host.as_str())
                .is_some_and(|before| before.is_empty() || before.ends_with('.'))
                && page.path().starts_with(&recipe.path);
            if fits && best.is_none_or(|best| recipe.path.len() > best.path.len()) {
                best = Some(recipe);
            }
        }
        best
    }
}

impl Recipe {
    fn of(written: Written, host: String) -> Result<Self, crate::select::Invalid> {
        let selector = |text: Option<String>| text.as_deref().map(Selector::parse).transpose();
        Ok(Self {
            item: Selector::parse(&written.item)?,
            title: selector(written.title)?,
            link: selector(written.link)?,
            summary: selector(written.summary)?,
            date: selector(written.date)?,
            image: selector(written.image)?,
            path: written.path,
            host,
        })
    }

    /// The entries of a page this recipe fits.
    pub fn items(&self, document: &Handle, base: &Url) -> Vec<Item> {
        let below = |selector: &Option<Selector>, item: &Handle| {
            selector.as_ref().and_then(|selector| selector.first(item))
        };
        self.item
            .all(document, MAX_ITEMS)
            .iter()
            .filter_map(|item| {
                let parts = Parts {
                    title: below(&self.title, item),
                    link: below(&self.link, item),
                    summary: below(&self.summary, item),
                    date: below(&self.date, item),
                    image: below(&self.image, item),
                };
                extract::block_item(item, parts, base)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html;

    const RECIPES: &str = r#"
        [[recipe]]
        host = "example.org"
        item = "li.news"

        [[recipe]]
        host = "Example.org."
        path = "/club/"
        item = ".termine div"
        title = "b"
        link = "a.more"
        summary = "span"
        date = ".when"
    "#;

    #[test]
    fn the_recipes_that_come_with_rssifier_are_written_right() {
        Recipes::built_in().unwrap();
    }

    #[test]
    fn the_longest_path_of_the_host_and_its_subdomains_wins() {
        let recipes = Recipes::parse(RECIPES).unwrap();
        let path = |page: &str| {
            recipes
                .for_page(&Url::parse(page).unwrap())
                .map(|recipe| recipe.path.clone())
        };
        assert_eq!(path("https://example.org/").as_deref(), Some(""));
        assert_eq!(
            path("https://www.example.org/club/x").as_deref(),
            Some("/club/")
        );
        assert_eq!(path("https://example.org/clubs").as_deref(), Some(""));
        assert_eq!(path("https://badexample.org/"), None);
        assert_eq!(path("https://example.org.evil.net/"), None);
    }

    #[test]
    fn a_recipe_names_the_parts_or_lets_them_be_found() {
        let recipes = Recipes::parse(RECIPES).unwrap();
        let base = Url::parse("https://example.org/club/").unwrap();
        let page = r#"<section class=termine>
            <div><b>Sommerfest</b> <span>Mit Grill und Musik</span> <i class=when>12. Juli 2026</i>
              <a href="/x">x</a> <a class=more href="/fest?utm_source=x">mehr</a></div>
            <div><b>Hauptversammlung</b><a class=more href="/hv">mehr</a></div>
            <div><b></b><a class=more href="/leer">mehr</a></div>
        </section>"#;
        let document = html::read(page).unwrap();
        let items = recipes.for_page(&base).unwrap().items(&document, &base);
        assert_eq!(items.len(), 2, "an entry without a title is none");
        assert_eq!(items[0].title, "Sommerfest");
        assert_eq!(items[0].url.as_str(), "https://example.org/fest");
        assert_eq!(items[0].summary.as_deref(), Some("Mit Grill und Musik"));
        assert_eq!(
            items[0].published.unwrap().date(),
            time::macros::date!(2026 - 07 - 12)
        );
        assert_eq!(items[1].url.as_str(), "https://example.org/hv");
    }

    #[test]
    fn a_recipe_written_wrong_says_which() {
        let wrong = r#"[[recipe]]
            host = "example.net"
            item = "a > b""#;
        let err = format!("{:#}", Recipes::parse(wrong).unwrap_err());
        assert!(err.contains("example.net"), "{err}");
        assert!(Recipes::parse("[[recipe]]\nhost = \"x\"\nitem = \"a\"\nfoo = 1").is_err());
    }
}
