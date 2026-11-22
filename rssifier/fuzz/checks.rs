//! What the fuzz target checks, for any input: the properties of "Fuzzing" in
//! `../docs/architecture.md`. The target in `fuzz_targets/` and the test that
//! plays the stored corpus with stable Rust (`tests/fuzz_corpus.rs`) both take
//! this file in, so the corpus checks the same as the fuzzer does.
//!
//! A broken property is a panic: that is what the fuzzer looks for.

use std::{collections::HashSet, sync::LazyLock};

use rssifier::{
    extract::{self, MAX_ITEMS, MAX_SUMMARY, MAX_TITLE},
    recipes::Recipes,
};
use url::Url;

/// A recipe for the page of the first base, so that the input also goes
/// through the selectors, and one that reads attributes, as some sites write them, for
/// the third; the second base has none.
static RECIPES: LazyLock<Recipes> = LazyLock::new(|| {
    Recipes::parse(
        r#"[[recipe]]
        host = "example.org"
        item = "div.entry, li"
        title = "h2, .title"
        summary = "p"
        date = "time, .date"
        image = "img[src]"

        [[recipe]]
        host = "example.com"
        item = "board-post, article"
        summary = "[slot=text-body]"
        image = "[slot=thumbnail] img"
        attributes = { title = "post-title", link = "permalink", date = "created-timestamp", id = "id" }
        "#,
    )
    .expect("the recipe of the fuzz target")
});

const BASES: [&str; 3] = [
    "https://example.org/news/",
    "https://example.net/a/b?c=d",
    "https://example.com/r/x/new/",
];

/// Reading a page, with any text: at most `MAX_ITEMS` entries, each with a
/// whole web address that is its own, a title and short texts, and the same
/// page read again gives the same entries, as rueder would otherwise make new
/// versions of them.
pub fn page(data: &[u8]) {
    let text = String::from_utf8_lossy(data);
    for base in BASES {
        let base = Url::parse(base).expect("a base of the fuzz target");
        let page = extract::page(&text, &base, &RECIPES);
        assert!(
            page.items.len() <= MAX_ITEMS,
            "{} entries",
            page.items.len()
        );
        assert!(page.title.chars().count() <= MAX_TITLE + 1);
        let mut seen = HashSet::new();
        for item in &page.items {
            assert!(is_web(&item.url), "{}", item.url);
            assert!(item.url.fragment().is_none(), "{}", item.url);
            assert!(item.url != base, "the page itself is an entry");
            assert!(seen.insert(item.url.clone()), "{} twice", item.url);
            assert!(!item.title.trim().is_empty(), "an entry without a title");
            assert!(item.title.chars().count() <= MAX_TITLE + 1);
            if let Some(summary) = &item.summary {
                assert!(summary.chars().count() <= MAX_SUMMARY + 1);
            }
            if let Some(image) = &item.image {
                assert!(is_web(image), "{image}");
            }
            if let Some(id) = &item.id {
                assert!(!id.is_empty() && id.len() <= 200, "{id:?}");
                assert!(!id.contains(char::is_control), "{id:?}");
            }
        }
        assert_eq!(
            extract::page(&text, &base, &RECIPES),
            page,
            "the same page read twice"
        );
    }
}

fn is_web(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https") && url.host().is_some()
}
