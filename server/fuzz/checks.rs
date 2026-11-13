//! What the fuzz targets check, for any input: the properties of "Fuzzing" in
//! `docs/architecture.md`. The targets in `fuzz_targets/` and the test that
//! plays the stored corpus with stable Rust (`tests/fuzz_corpus.rs`) both take
//! this file in, so the corpus checks the same as the fuzzer does.
//!
//! A broken property is a panic: that is what the fuzzer looks for.

use std::{io::Cursor, sync::LazyLock};

use image::{ImageFormat, ImageReader};
use rueder::{
    cleaning::{self, Base, Rules},
    feeds::{self, MAX_ARTICLES, MAX_CONTENT},
    greader::parse::{self, FeedRef, Params, Stream},
    images::{self, Format, MAX_PIXELS, Refused, Signer, Size},
};
use time::{OffsetDateTime, macros::datetime};

static RULES: LazyLock<Rules> =
    LazyLock::new(|| Rules::built_in().expect("the lists that come with rueder"));
static SIGNER: LazyLock<Signer> =
    LazyLock::new(|| Signer::new(vec![7; 32]).expect("a key of 32 bytes"));

/// An article on a page of its own, from a feed elsewhere, and one without
/// either: relative addresses resolve in the first and go in the second.
const BASES: [Base<'static>; 2] = [
    Base {
        article: Some("https://example.com/article/23?bar=baz"),
        feed: Some("https://feeds.example.net/foo/bar/feed.rss"),
    },
    Base {
        article: None,
        feed: None,
    },
];

const FEED_URL: &str = "https://example.org/feed.xml";
const FETCHED_AT: OffsetDateTime = datetime!(2026-11-11 12:00 UTC);

/// The cleaning, with any text: what comes out holds no script, no event
/// handler, no form and no address that is not a whole web address, a second
/// round changes nothing, and once delivered every image points to the proxy.
pub fn cleaning(data: &[u8]) {
    let text = String::from_utf8_lossy(data);
    for base in BASES {
        let once = cleaning::clean(&text, base, &RULES);
        assert_clean(&once.html);
        for found in &once.enclosures {
            assert!(is_web(&found.url), "enclosure {:?}", found.url);
        }
        let twice = cleaning::clean(&once.html, base, &RULES);
        assert_eq!(twice.html, once.html, "a second round changed the text");
        assert_delivered(&SIGNER.in_html(&once.html));
    }
}

/// Reading a feed, from its bytes to the finished articles: the limit for the
/// number of articles holds, every article has an id, its text is clean, and
/// the same bytes give the same articles with the same checksums.
pub fn feed(data: &[u8]) {
    let Ok(first) = feeds::parse(data, FEED_URL, FETCHED_AT, &RULES) else {
        return;
    };
    assert!(first.articles.len() <= MAX_ARTICLES, "too many articles");
    for article in &first.articles {
        assert!(!article.guid.is_empty(), "an article without an id");
        assert!(article.original.content.len() <= MAX_CONTENT, "too long");
        assert_clean(&article.html);
        for address in [&article.thumbnail_url, &article.image_url]
            .into_iter()
            .flatten()
        {
            assert!(is_web(address), "image {address:?}");
        }
        if let Some(site) = &first.site_url {
            assert!(is_web(site), "site {site:?}");
        }
    }
    let second = feeds::parse(data, FEED_URL, FETCHED_AT, &RULES).expect("read once already");
    assert_eq!(second.articles.len(), first.articles.len());
    for (a, b) in first.articles.iter().zip(&second.articles) {
        assert_eq!(a.guid, b.guid);
        assert_eq!(
            a.checksum(),
            b.checksum(),
            "the same input, another checksum"
        );
    }
}

/// The image proxy, with any bytes; the first byte says for which size. Only
/// the formats it lets through come out, in the size asked for, and an image
/// whose head says it is too large is refused before it is decoded.
pub fn image(data: &[u8]) {
    let Some((&which, bytes)) = data.split_first() else {
        return;
    };
    let size = [Size::Icon, Size::Thumbnail, Size::Image][usize::from(which) % 3];
    let claimed = dimensions(bytes);
    match images::fit(bytes.to_vec(), size) {
        Ok(picture) => {
            let format = match image::guess_format(&picture.bytes) {
                Ok(ImageFormat::Png) => Format::Png,
                Ok(ImageFormat::Jpeg) => Format::Jpeg,
                Ok(ImageFormat::Gif) => Format::Gif,
                Ok(ImageFormat::WebP) => Format::WebP,
                other => panic!("served as {:?}, but is {other:?}", picture.format),
            };
            assert_eq!(format, picture.format, "served under another format");
            let (width, height) = dimensions(&picture.bytes).expect("an image that was served");
            let (max_width, max_height) = size.fits();
            assert!(
                width <= max_width && height <= max_height,
                "{width}×{height} served as {size:?}"
            );
            if let Some((width, height)) = claimed {
                assert!(u64::from(width) * u64::from(height) <= MAX_PIXELS);
            }
        }
        Err(Refused::NotAnImage | Refused::TooLarge) => {}
        Err(other) => panic!("refused for another reason: {other}"),
    }
}

/// Reading a request of the Google Reader API, with any bytes: the first byte
/// says where the query ends and the form body begins. The limits of count and
/// length hold, nothing panics, and every stream ID, item ID and path of
/// folders that is read is read back as itself once it is written.
pub fn greader(data: &[u8]) {
    let Some((split, rest)) = data.split_first() else {
        return;
    };
    let at = usize::from(*split) * rest.len() / 255;
    let (query, body) = rest.split_at(at);
    let Ok(params) = Params::parse(&String::from_utf8_lossy(query), body) else {
        return;
    };
    assert!(params.pairs().len() <= parse::MAX_PAIRS);
    for (name, value) in params.pairs() {
        assert!(name.len() <= parse::MAX_VALUE && value.len() <= parse::MAX_VALUE);
        assert!(params.one(name).is_some());
        greader_value(value);
    }
}

/// What one value of a request is read as.
fn greader_value(value: &str) {
    if let Some(stream) = parse::stream(value) {
        let written = match &stream {
            Stream::Feed(FeedRef::Id(id)) => Some(parse::feed_id(*id)),
            Stream::Feed(FeedRef::Url(_)) => None,
            Stream::Label(name) => Some(parse::label_id(name)),
            Stream::State(state) => state.id().map(str::to_owned),
        };
        if let Some(written) = written {
            assert_eq!(parse::stream(&written), Some(stream), "{written}");
        }
    }
    if let Some((feed, seq)) = parse::item(value) {
        assert!(feed > 0 && seq > 0);
        let number = parse::item_number(feed, seq).expect("a feed and a number fit");
        assert!(number > 0);
        assert_eq!(parse::item(&parse::long_item_id(number)), Some((feed, seq)));
        assert_eq!(parse::item(&number.to_string()), Some((feed, seq)));
    }
    if let Some(path) = parse::path(value) {
        assert!((1..=parse::MAX_DEPTH).contains(&path.len()));
        for part in &path {
            assert!(!part.trim().is_empty() && part.chars().count() <= parse::MAX_NAME);
            assert_eq!(part.trim(), part);
        }
        assert_eq!(parse::path(&parse::path_name(&path)), Some(path));
    }
    if let Some(after) = parse::continuation(value) {
        assert!(after > 0);
        assert_eq!(parse::continuation(&after.to_string()), Some(after));
    }
    let _ = parse::time(value);
    let n = parse::count(Some(value), 20, 1000);
    assert!((1..=1000).contains(&n));
}

/// The width and height the head of an image says it has.
fn dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()
}

fn is_web(address: &str) -> bool {
    url::Url::parse(address)
        .is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
}

/// What every cleaned text keeps to, as `tests/cleaning.rs` checks it.
fn assert_clean(cleaned: &str) {
    assert!(!cleaned.contains("<!--"), "a comment in {cleaned:?}");
    for (name, attributes) in tags(cleaned) {
        let name = name.trim_start_matches('/').to_ascii_lowercase();
        assert!(
            !matches!(
                name.as_str(),
                "script"
                    | "style"
                    | "iframe"
                    | "frame"
                    | "frameset"
                    | "form"
                    | "input"
                    | "button"
                    | "textarea"
                    | "select"
                    | "object"
                    | "embed"
                    | "video"
                    | "audio"
                    | "source"
                    | "svg"
                    | "math"
                    | "link"
                    | "meta"
                    | "base"
                    | "template"
                    | "noscript"
            ),
            "<{name}> in {cleaned:?}"
        );
        for (attribute, value) in attributes {
            let attribute = attribute.to_ascii_lowercase();
            // Attributes that run code, load more than was asked for or change the page.
            assert!(
                !attribute.starts_with("on")
                    && !matches!(
                        attribute.as_str(),
                        "srcset"
                            | "style"
                            | "class"
                            | "id"
                            | "name"
                            | "ping"
                            | "action"
                            | "formaction"
                    ),
                "{attribute} on <{name}> in {cleaned:?}"
            );
            // Every address is a whole web address.
            if matches!(attribute.as_str(), "href" | "src") {
                assert!(is_web(value), "{attribute}={value:?} in {cleaned:?}");
            }
        }
    }
}

/// A cleaned text as the browser gets it: its images come from the proxy and
/// nowhere else.
fn assert_delivered(html: &str) {
    for (name, attributes) in tags(html) {
        for (attribute, value) in attributes {
            if attribute == "src" {
                assert!(
                    name == "img" && value.starts_with("/img/"),
                    "<{name} src={value:?}> in {html:?}"
                );
            }
        }
    }
}

/// The tags of a text as the cleaning writes it, with their attributes: it
/// writes `<` in text as `&lt;` and every attribute as `name="value"`, with `"`
/// in the value as `&quot;`.
fn tags(html: &str) -> Vec<(&str, Vec<(&str, &str)>)> {
    let mut tags = Vec::new();
    let mut rest = html;
    while let Some((_, tag)) = rest.split_once('<') {
        let end = tag.find([' ', '>']).unwrap_or(tag.len());
        let (name, mut after) = tag.split_at(end);
        let mut attributes = Vec::new();
        loop {
            after = after.trim_start_matches(' ');
            if after.is_empty() || after.starts_with('>') {
                break;
            }
            let end = after.find(['=', ' ', '>']).unwrap_or(after.len());
            if end == 0 {
                // A `=` without a name before it, which the cleaning never writes.
                after = &after[1..];
                continue;
            }
            let (attribute, behind) = after.split_at(end);
            let Some(quoted) = behind.strip_prefix("=\"") else {
                attributes.push((attribute, ""));
                after = behind;
                continue;
            };
            let (value, behind) = quoted.split_once('"').unwrap_or((quoted, ""));
            attributes.push((attribute, value));
            after = behind;
        }
        tags.push((name, attributes));
        rest = after;
    }
    tags
}
