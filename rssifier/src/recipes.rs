//! Recipes: how to read the pages of one site, as data. The ones that come
//! with RSSifier are in `rules/recipes.toml`, which also says how they are
//! written; the operator can add a file of more (`RSSIFIER_RECIPES`). A
//! recipe that is `optional` is off until the operator names it
//! (`RSSIFIER_ENABLE`).

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
    optional: Option<String>,
    #[serde(default = "yes")]
    robots: bool,
    fetch: Option<String>,
    attributes: Option<Attributes>,
}

fn yes() -> bool {
    true
}

/// The attributes of an item that hold its parts.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Attributes {
    title: String,
    link: String,
    date: Option<String>,
    id: Option<String>,
}

/// The longest id of an entry, in bytes.
const MAX_ID: usize = 200;

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
    optional: Option<String>,
    /// Whether it is used: an optional recipe only once it is turned on.
    on: bool,
    /// Whether `robots.txt` is asked for the pages it fits.
    pub robots: bool,
    fetch: Option<String>,
    attributes: Option<Attributes>,
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

    /// Turns on the optional recipes named in `names`, separated by commas,
    /// as `RSSIFIER_ENABLE` has them. A name no recipe has is an error, so
    /// that a typo does not go unnoticed.
    pub fn enable(mut self, names: &str) -> anyhow::Result<Self> {
        for name in names.split(',').map(str::trim).filter(|n| !n.is_empty()) {
            let mut found = false;
            for recipe in &mut self.0 {
                if recipe.optional.as_deref() == Some(name) {
                    recipe.on = true;
                    found = true;
                }
            }
            anyhow::ensure!(found, "no optional recipe is called {name:?}");
        }
        Ok(self)
    }

    /// The address to fetch for a page: what the recipe for it says in
    /// `fetch`, else the page.
    pub fn address(&self, page: &Url) -> Url {
        self.for_page(page)
            .and_then(|recipe| recipe.address(page))
            .unwrap_or_else(|| page.clone())
    }

    /// The recipe for a page: of those for its host, the one with the
    /// longest path that fits; the first of them if two are as long.
    pub fn for_page(&self, page: &Url) -> Option<&Recipe> {
        let host = page.host_str()?.trim_end_matches('.');
        let mut best: Option<&Recipe> = None;
        for recipe in self.0.iter().filter(|recipe| recipe.on) {
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
    fn of(written: Written, host: String) -> anyhow::Result<Self> {
        if !written.robots && written.optional.is_none() {
            anyhow::bail!("only an optional recipe leaves out robots.txt");
        }
        let named = [&written.title, &written.link, &written.date];
        if written.attributes.is_some() && named.iter().any(|part| part.is_some()) {
            anyhow::bail!("title, link and date come from the attributes or from selectors");
        }
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
            on: written.optional.is_none(),
            optional: written.optional,
            robots: written.robots,
            fetch: written.fetch,
            attributes: written.attributes,
        })
    }

    /// `fetch` with `{1}` to `{9}` as the parts of the page's path, if it
    /// has one and the page has every part it names.
    fn address(&self, page: &Url) -> Option<Url> {
        let mut address = self.fetch.clone()?;
        let parts: Vec<&str> = page.path_segments()?.collect();
        for n in 1..=9 {
            let placeholder = format!("{{{n}}}");
            if address.contains(&placeholder) {
                let part = parts.get(n - 1).filter(|part| !part.is_empty())?;
                address = address.replace(&placeholder, part);
            }
        }
        extract::link(page, &address)
    }

    /// The entries of a page this recipe fits.
    pub fn items(&self, document: &Handle, base: &Url) -> Vec<Item> {
        if let Some(attributes) = &self.attributes {
            return self.items_in_attributes(attributes, document, base);
        }
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

    /// The entries of a recipe whose items hold their parts in attributes.
    /// Nothing is looked for elsewhere: a summary or an image only comes
    /// from what the recipe names.
    fn items_in_attributes(&self, names: &Attributes, document: &Handle, base: &Url) -> Vec<Item> {
        self.item
            .all(document, MAX_ITEMS)
            .iter()
            .filter_map(|item| {
                let value = |name: &str| extract::attribute(item, name).map(|v| extract::words(&v));
                let title = value(&names.title).filter(|title| !title.is_empty())?;
                let below = |selector: &Option<Selector>| {
                    selector.as_ref().and_then(|selector| selector.first(item))
                };
                let summary = below(&self.summary)
                    .map(|node| extract::steady(&extract::words(&extract::text_of(&node))))
                    .filter(|summary| !summary.is_empty())
                    .map(|summary| extract::clip(&summary, extract::MAX_SUMMARY));
                let image = below(&self.image).and_then(|node| {
                    ["src", "data-src"]
                        .iter()
                        .filter_map(|name| extract::attribute(&node, name))
                        .find_map(|address| extract::link(base, &address))
                });
                let id = names.id.as_deref().and_then(value).filter(|id| {
                    !id.is_empty() && id.len() <= MAX_ID && !id.contains(char::is_control)
                });
                Some(Item {
                    id,
                    url: extract::link(base, &value(&names.link)?)?,
                    title: extract::clip(&title, extract::MAX_TITLE),
                    summary,
                    image,
                    published: names
                        .date
                        .as_deref()
                        .and_then(value)
                        .as_deref()
                        .and_then(extract::date),
                })
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

    /// A page of a subreddit as Reddit shows it without a login, made up.
    const SUBREDDIT: &str = r#"<shreddit-feed>
        <article><shreddit-post id="t3_1abc" post-title="  Rust 2.0 is   out "
            permalink="/r/rust/comments/1abc/rust_20_is_out/" score="1234" comment-count="56"
            created-timestamp="2026-11-20T10:15:00.123000+0000" author="someone">
          <a slot="title" href="/r/rust/comments/1abc/rust_20_is_out/">Rust 2.0 is out</a>
          <div slot="text-body"><p>Read all about it. 12 comments</p></div>
          <faceplate-number number="1234">1.2k</faceplate-number>
          <img src="/avatar.png">
        </shreddit-post></article>
        <article><shreddit-post id="t3_1abd" post-title="A link"
            permalink="/r/rust/comments/1abd/a_link/" created-timestamp="2026-11-19T08:00:00+0100">
          <div slot="thumbnail"><img src="https://preview.example/1abd.jpg"></div>
          <span>99 points</span>
        </shreddit-post></article>
        <shreddit-post id="t3_1abe" permalink="/r/rust/comments/1abe/x/"></shreddit-post>
    </shreddit-feed>"#;

    #[test]
    fn reddit_is_off_until_the_operator_turns_it_on() {
        let page = Url::parse("https://www.reddit.com/r/rust/").unwrap();
        let recipes = Recipes::built_in().unwrap();
        assert!(recipes.for_page(&page).is_none());
        assert_eq!(recipes.address(&page), page);
        let recipes = recipes.enable(" reddit ,").unwrap();
        assert!(!recipes.for_page(&page).unwrap().robots);
        let err = Recipes::built_in().unwrap().enable("redit").unwrap_err();
        assert!(err.to_string().contains("redit"), "{err}");
    }

    #[test]
    fn reddit_is_read_newest_first_from_the_attributes_of_its_posts() {
        let recipes = Recipes::built_in().unwrap().enable("reddit").unwrap();
        let address = |page: &str| recipes.address(&Url::parse(page).unwrap()).to_string();
        let newest = "https://www.reddit.com/r/rust/new/";
        assert_eq!(address("https://reddit.com/r/rust"), newest);
        assert_eq!(address("https://old.reddit.com/r/rust/top/?t=week"), newest);
        assert_eq!(
            address("https://www.reddit.com/r/"),
            "https://www.reddit.com/r/"
        );

        let base = Url::parse(newest).unwrap();
        let document = html::read(SUBREDDIT).unwrap();
        let items = recipes.for_page(&base).unwrap().items(&document, &base);
        assert_eq!(items.len(), 2, "a post without a title is none");
        assert_eq!(items[0].id.as_deref(), Some("t3_1abc"));
        assert_eq!(items[0].title, "Rust 2.0 is out");
        assert_eq!(
            items[0].url.as_str(),
            "https://www.reddit.com/r/rust/comments/1abc/rust_20_is_out/"
        );
        // Nothing that counts: no score, no comments.
        assert_eq!(items[0].summary.as_deref(), Some("Read all about it."));
        assert_eq!(items[0].image, None, "an avatar is no image of the post");
        assert_eq!(
            items[0].published,
            Some(time::macros::datetime!(2026-11-20 10:15:00.123 UTC))
        );
        assert_eq!(items[1].summary, None);
        assert_eq!(
            items[1].image.as_ref().map(Url::as_str),
            Some("https://preview.example/1abd.jpg")
        );
        assert_eq!(
            items[1].published,
            Some(time::macros::datetime!(2026-11-19 08:00:00 +1))
        );
    }

    #[test]
    fn a_recipe_written_wrong_says_which() {
        let wrong = r#"[[recipe]]
            host = "example.net"
            item = "a > b""#;
        let err = format!("{:#}", Recipes::parse(wrong).unwrap_err());
        assert!(err.contains("example.net"), "{err}");
        assert!(Recipes::parse("[[recipe]]\nhost = \"x\"\nitem = \"a\"\nfoo = 1").is_err());
        let robots = "[[recipe]]\nhost = \"x\"\nitem = \"a\"\nrobots = false";
        assert!(Recipes::parse(robots).is_err(), "only an optional one");
        let both = "[[recipe]]\nhost = \"x\"\nitem = \"a\"\ntitle = \"b\"\n\
                    attributes = { title = \"t\", link = \"l\" }";
        assert!(Recipes::parse(both).is_err());
    }
}
