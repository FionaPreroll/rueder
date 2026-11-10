//! Reading feeds and putting what they list into the store: what is new, what
//! changed, and what only looks changed.

mod common;

use rueder::{
    articles::{self, Page},
    feeds::{self, MAX_ARTICLES, Stored},
};
use sqlx::PgPool;
use time::{OffsetDateTime, macros::datetime};

const NOW: OffsetDateTime = datetime!(2026-11-10 12:00 UTC);

/// A feed as the old rueder's tests recorded it.
fn recorded(name: &str) -> Vec<u8> {
    std::fs::read(format!("../backend/test/data/{name}")).unwrap()
}

/// A small feed with one entry per (id, title, text).
fn rss(entries: &[(&str, &str, &str)]) -> Vec<u8> {
    let items: String = entries
        .iter()
        .map(|(id, title, text)| {
            format!(
                "<item><guid>{id}</guid><title>{title}</title><link>https://example.org/{id}</link>\
                 <description><![CDATA[{text}]]></description></item>"
            )
        })
        .collect();
    format!(
        "<?xml version=\"1.0\"?><rss version=\"2.0\"><channel><title>Example</title>\
         <link>https://example.org/</link>{items}</channel></rss>"
    )
    .into_bytes()
}

fn parse(body: &[u8]) -> feeds::Parsed {
    feeds::parse(body, "https://example.org/feed.xml", NOW).unwrap()
}

#[test]
fn reads_the_recorded_feeds() {
    // RSS 1.0 in ISO-8859-1. The recording says so but was saved as UTF-8, so
    // put it back into the bytes the site sent.
    let latin1: Vec<u8> = String::from_utf8(recorded("golem.xml"))
        .unwrap()
        .chars()
        .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
        .collect();
    let golem = parse(&latin1);
    assert_eq!(golem.articles.len(), 3);
    assert_eq!(golem.skipped, 0);
    assert!(
        golem
            .title
            .as_deref()
            .is_some_and(|title| title.contains("Golem")),
        "{:?}",
        golem.title
    );
    assert!(
        golem
            .site_url
            .as_deref()
            .is_some_and(|url| url.starts_with("https://"))
    );
    for article in &golem.articles {
        assert!(!article.guid.is_empty());
        assert!(!article.original.title.is_empty());
        assert!(
            article
                .original
                .url
                .as_deref()
                .is_some_and(|url| url.starts_with("https://"))
        );
        assert!(!article.teaser.contains('<'), "{}", article.teaser);
        assert!(article.published_at <= NOW);
    }
    // Umlauts survive the old encoding.
    let all: String = golem
        .articles
        .iter()
        .map(|article| format!("{} {}", article.original.title, article.teaser))
        .collect();
    assert!(all.chars().any(|c| "äöüÄÖÜß".contains(c)), "{all}");
    assert!(!all.contains('\u{fffd}'), "{all}");

    let fefe = parse(&recorded("fefe-html.xml"));
    assert_eq!(fefe.articles.len(), 10);
    let ids: std::collections::HashSet<_> =
        fefe.articles.iter().map(|article| &article.guid).collect();
    assert_eq!(ids.len(), 10);

    // A podcast: the audio comes along as an enclosure.
    let podcast = parse(&recorded("raumzeit-podcast-oga.xml"));
    assert_eq!(podcast.articles.len(), 5);
    for article in &podcast.articles {
        let enclosures = article.enclosures.as_array().unwrap();
        assert!(!enclosures.is_empty(), "{}", article.original.title);
        assert!(enclosures[0]["url"].as_str().unwrap().starts_with("http"));
    }
}

#[test]
fn articles_come_oldest_first() {
    let podcast = parse(&recorded("raumzeit-podcast-oga.xml"));
    let dates: Vec<_> = podcast
        .articles
        .iter()
        .map(|article| article.published_at)
        .collect();
    assert!(dates.is_sorted(), "{dates:?}");
    assert!(dates[0] < dates[4]);

    // Without dates, the feed's order counts: its first entry is its newest.
    let undated = parse(&rss(&[
        ("c", "Third", "x"),
        ("b", "Second", "x"),
        ("a", "First", "x"),
    ]));
    let ids: Vec<_> = undated
        .articles
        .iter()
        .map(|article| article.guid.as_str())
        .collect();
    assert_eq!(ids, ["a", "b", "c"]);
    assert!(
        undated
            .articles
            .iter()
            .all(|article| article.published_at == NOW)
    );
}

#[test]
fn the_text_for_reading_is_cleaned_and_the_original_is_kept() {
    let text = "<p onclick=\"steal()\">Hello <b>world</b></p><script>alert(1)</script>\
                <img src=\"https://example.org/a.png\" onerror=\"steal()\"><iframe src=\"https://evil.example/\"></iframe>\
                <a href=\"javascript:alert(1)\">click</a>";
    let parsed = parse(&rss(&[(
        "a",
        "A &lt;b&gt;bold&lt;/b&gt; title &amp; more",
        text,
    )]));
    let article = &parsed.articles[0];
    assert_eq!(article.original.content, text);
    for bad in [
        "script",
        "onclick",
        "onerror",
        "iframe",
        "javascript:",
        "steal",
        "alert",
    ] {
        assert!(!article.html.contains(bad), "{bad} in {}", article.html);
    }
    assert!(article.html.contains("<b>world</b>"), "{}", article.html);
    assert!(
        article.html.contains("https://example.org/a.png"),
        "{}",
        article.html
    );
    // Titles and teasers are plain text.
    assert_eq!(article.original.title, "A bold title & more");
    assert_eq!(article.teaser, "Hello world click");
    assert_eq!(
        article.original.url.as_deref(),
        Some("https://example.org/a")
    );
}

#[test]
fn an_entry_keeps_its_identity_from_fetch_to_fetch() {
    // No id: the link stands in. No link either: title and text do.
    let feed = b"<?xml version=\"1.0\"?><rss version=\"2.0\"><channel><title>T</title>\
        <item><title>With link</title><link>https://example.org/1</link><description>one</description></item>\
        <item><title>Bare</title><description>two</description></item>\
        <item></item>\
        </channel></rss>";
    let first = parse(feed);
    let second = parse(feed);
    assert_eq!(first.articles.len(), 2);
    assert_eq!(first.skipped, 1);
    let ids = |parsed: &feeds::Parsed| {
        parsed
            .articles
            .iter()
            .map(|article| article.guid.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&first), ids(&second));
    assert!(ids(&first).contains(&"https://example.org/1".to_owned()));
}

#[test]
fn links_that_are_not_for_the_web_are_dropped() {
    let feed = b"<?xml version=\"1.0\"?><rss version=\"2.0\"><channel><title>T</title><link>javascript:alert(1)</link>\
        <item><guid>a</guid><title>A</title><link>javascript:alert(1)</link><description>x</description></item>\
        </channel></rss>";
    let parsed = parse(feed);
    assert_eq!(parsed.site_url, None);
    assert_eq!(parsed.articles[0].original.url, None);
}

#[test]
fn limits_hold_and_nonsense_is_an_error() {
    let entries: Vec<(String, String)> = (0..MAX_ARTICLES + 20)
        .map(|n| (format!("id{n}"), format!("Title {n}")))
        .collect();
    let entries: Vec<(&str, &str, &str)> = entries
        .iter()
        .map(|(id, title)| (id.as_str(), title.as_str(), "text"))
        .collect();
    let parsed = parse(&rss(&entries));
    assert_eq!(parsed.articles.len(), MAX_ARTICLES);
    assert_eq!(parsed.skipped, 20);
    // The newest are kept: they stand at the top of the feed.
    assert_eq!(parsed.articles.last().unwrap().guid, "id0");

    let huge = "x".repeat(feeds::MAX_CONTENT + 1);
    let parsed = parse(&rss(&[("a", "Fine", "short"), ("b", "Huge", &huge)]));
    assert_eq!(parsed.articles.len(), 1);
    assert_eq!(parsed.skipped, 1);

    for nonsense in [
        &b""[..],
        b"hello",
        b"<html><body>not a feed</body></html>",
        b"{\"a\": 1}",
        &[0xff, 0xfe, 0x00][..],
    ] {
        assert!(feeds::parse(nonsense, "https://example.org/", NOW).is_err());
    }
}

#[sqlx::test]
async fn a_fetch_adds_what_is_new_and_nothing_twice(db: PgPool) {
    let (account, feed) = common::reader(&db, "ada", 0).await;
    let first = parse(&rss(&[("b", "Second", "two"), ("a", "First", "one")]));
    let mut tx = db.begin().await.unwrap();
    let stored = feeds::store(&mut tx, feed, &first.articles).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        stored,
        Stored {
            new: 2,
            updated: 0,
            first_new: Some(1)
        }
    );

    // The same feed again: nothing happens.
    let mut tx = db.begin().await.unwrap();
    let stored = feeds::store(&mut tx, feed, &first.articles).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(stored, Stored::default());

    // A new entry on top, and one id listed twice.
    let next = parse(&rss(&[
        ("c", "Third", "three"),
        ("c", "Third", "three"),
        ("b", "Second", "two"),
        ("a", "First", "one"),
    ]));
    let mut tx = db.begin().await.unwrap();
    let stored = feeds::store(&mut tx, feed, &next.articles).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        stored,
        Stored {
            new: 1,
            updated: 0,
            first_new: Some(3)
        }
    );

    let page = Page {
        before: None,
        limit: 10,
        unread_only: false,
    };
    let list = articles::page(&db, account, &[feed], page).await.unwrap();
    let titles: Vec<_> = list
        .iter()
        .map(|item| (item.seq, item.title.as_str(), item.version))
        .collect();
    assert_eq!(titles, [(3, "Third", 1), (2, "Second", 1), (1, "First", 1)]);
}

#[sqlx::test]
async fn a_changed_text_is_a_new_version_and_a_reflowed_one_is_not(db: PgPool) {
    let (account, feed) = common::reader(&db, "ada", 0).await;
    let store = async |entries: &[(&str, &str, &str)]| {
        let parsed = parse(&rss(entries));
        let mut tx = db.begin().await.unwrap();
        let stored = feeds::store(&mut tx, feed, &parsed.articles).await.unwrap();
        tx.commit().await.unwrap();
        stored
    };
    store(&[("a", "Title", "<p>The first text.</p>")]).await;

    // Only the white space differs: not a change.
    let stored = store(&[("a", "Title", "<p>The   first\n\ttext.</p>\n")]).await;
    assert_eq!(stored, Stored::default());

    // The text differs: a new version of the same article, not a new article.
    let stored = store(&[("a", "Title", "<p>The corrected text.</p>")]).await;
    assert_eq!(
        stored,
        Stored {
            new: 0,
            updated: 1,
            first_new: None
        }
    );
    let opened = articles::open(&db, account, feed, 1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(opened.html, "<p>The corrected text.</p>");
    let versions = articles::versions(&db, account, feed, 1).await.unwrap();
    assert_eq!(versions.len(), 2);

    // The title counts too.
    let stored = store(&[("a", "Another title", "<p>The corrected text.</p>")]).await;
    assert_eq!(stored.updated, 1);
    let page = Page {
        before: None,
        limit: 10,
        unread_only: false,
    };
    let list = articles::page(&db, account, &[feed], page).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(
        (list[0].title.as_str(), list[0].version),
        ("Another title", 3)
    );
}

#[sqlx::test]
async fn storing_into_a_feed_that_is_not_there_fails(db: PgPool) {
    let parsed = parse(&rss(&[("a", "Title", "text")]));
    let mut tx = db.begin().await.unwrap();
    assert!(feeds::store(&mut tx, 4711, &parsed.articles).await.is_err());
}
