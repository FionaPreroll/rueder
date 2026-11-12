//! Reading feeds and putting what they list into the store: what is new, what
//! changed, and what only looks changed.

mod common;

use rueder::{
    articles::{self, Page},
    cleaning::Rules,
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
    feeds::parse(
        body,
        "https://example.org/feed.xml",
        NOW,
        &Rules::built_in().unwrap(),
    )
    .unwrap()
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
fn the_encoding_is_taken_from_the_declaration_wherever_it_ends() {
    // ISO-8859-1, with a declaration longer than the 64 bytes the library
    // reads ahead: it used to panic on that.
    let mut latin1 =
        b"<?xml version=\"1.0\"   encoding=\"ISO-8859-1\"   standalone=\"yes\" ?>\n".to_vec();
    latin1.extend_from_slice(
        b"<rss version=\"2.0\"><channel><title>K\xe4se</title><item><guid>1</guid>\
          <title>Gr\xfc\xdfe</title><description>\xb5</description></item></channel></rss>",
    );
    let parsed = parse(&latin1);
    assert_eq!(parsed.title.as_deref(), Some("Käse"));
    assert_eq!(parsed.articles[0].original.title, "Grüße");
    assert_eq!(parsed.articles[0].html, "µ");

    // A second declaration further down, as when two feeds were written one
    // after the other, names an encoding that no longer counts.
    let mut twice = latin1.clone();
    twice.extend_from_slice(b"\n<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rss/>");
    let parsed = feeds::parse(
        &twice,
        "https://example.org/feed.xml",
        NOW,
        &Rules::built_in().unwrap(),
    );
    if let Ok(parsed) = parsed {
        assert_eq!(parsed.title.as_deref(), Some("Käse"));
    }

    // A byte order mark says more than the declaration.
    let mut utf_16 = vec![0xff, 0xfe];
    for unit in "<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><rss version=\"2.0\"><channel>\
                 <title>Käse</title><item><guid>1</guid><title>x</title></item></channel></rss>"
        .encode_utf16()
    {
        utf_16.extend_from_slice(&unit.to_le_bytes());
    }
    assert_eq!(parse(&utf_16).title.as_deref(), Some("Käse"));
}

#[test]
fn a_byte_that_is_not_utf_8_makes_up_no_declaration() {
    // The library leaves out a byte it cannot decode and reads on, so
    // `enc\xC0oding` became `encoding` once it had read past the start: it
    // used to panic on that. Such a byte is a replacement character now.
    let broken = b"<?xml version=\"1.0\" enc\xC0oding=\"ISO-8859-1\"?>\n\
        <rss version=\"2.0\"><channel><title>K\xe4se</title><item><guid>1</guid>\
        <title>x</title></item></channel></rss>";
    assert_eq!(parse(broken).title.as_deref(), Some("K\u{fffd}se"));

    // A text in UTF-8 with one broken byte is read all the same.
    let feed = b"<?xml version=\"1.0\"?><rss version=\"2.0\"><channel>\
        <title>K\xc3\xa4se \xff</title><item><guid>1</guid><title>x</title></item>\
        </channel></rss>";
    assert_eq!(parse(feed).title.as_deref(), Some("K\u{e4}se \u{fffd}"));
}

#[test]
fn only_whole_web_addresses_are_kept_as_links() {
    let feed = "<?xml version=\"1.0\"?><rss version=\"2.0\"><channel><title>x</title>\
        <link>https://ex\u{8}ample.org/</link>\
        <item><title>a</title><link>https://exa mple.org/a</link></item>\
        <item><title>b</title><link>https://example.org/b?utm_source=x</link></item>\
        </channel></rss>";
    let parsed = parse(feed.as_bytes());
    assert_eq!(parsed.site_url, None);
    // Oldest first: the feed lists the newest on top.
    let [b, a] = &parsed.articles[..] else {
        panic!("two articles: {:?}", parsed.articles);
    };
    // The link still tells the article apart, as the old rueder did.
    assert_eq!(a.guid, "https://exa mple.org/a");
    assert_eq!(a.original.url, None);
    assert_eq!(b.original.url.as_deref(), Some("https://example.org/b"));
}

/// A feed in Atom 0.3, made up after the one tagesschau.de sends: its texts
/// escaped, one of them wrapped in a CDATA marker before it was escaped, and
/// its dates only as `dc:date`.
const ATOM_03: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://purl.org/atom/ns#" xmlns:dc="http://purl.org/dc/elements/1.1/" version="0.3">
  <title>The Daily Example</title>
  <link rel="alternate" href="https://news.example/" />
  <tagline>News from nowhere</tagline>
  <copyright>Nobody</copyright>
  <dc:date>2026-11-10T09:00:00Z</dc:date>
  <entry>
    <title>Second &amp; last</title>
    <link rel="alternate" href="https://news.example/2.html" />
    <id>2</id>
    <summary type="text/html" mode="escaped">Short &lt;b&gt;two&lt;/b&gt;</summary>
    <content mode="escaped">&lt;![CDATA[&lt;p&gt;Text &lt;b&gt;two&lt;/b&gt;&amp;nbsp;here.&lt;/p&gt;]]&gt;</content>
    <dc:date>2026-11-10T08:00:00Z</dc:date>
  </entry>
  <entry>
    <title>First</title>
    <link rel="alternate" href="https://news.example/1.html" />
    <id>1</id>
    <issued>2026-11-09T08:00:00Z</issued>
    <modified>2026-11-09T09:00:00Z</modified>
    <content type="text/plain" mode="xml">Plain words</content>
  </entry>
</feed>"#;

#[test]
fn atom_0_3_is_read_like_atom_1() {
    let parsed = parse(ATOM_03.as_bytes());
    assert_eq!(parsed.title.as_deref(), Some("The Daily Example"));
    assert_eq!(parsed.site_url.as_deref(), Some("https://news.example/"));
    assert_eq!(parsed.skipped, 0);
    let [first, second] = &parsed.articles[..] else {
        panic!("two articles: {:?}", parsed.articles);
    };
    assert_eq!((first.guid.as_str(), second.guid.as_str()), ("1", "2"));
    assert_eq!(first.published_at, datetime!(2026-11-09 08:00 UTC));
    assert_eq!(second.published_at, datetime!(2026-11-10 08:00 UTC));
    assert_eq!(second.original.title, "Second & last");
    assert_eq!(
        second.original.url.as_deref(),
        Some("https://news.example/2.html")
    );
    // The escaped HTML is HTML, without the marker around it.
    assert_eq!(second.html, "<p>Text <b>two</b>&nbsp;here.</p>");
    assert_eq!(second.teaser, "Short two");
    assert_eq!(first.html, "Plain words");
}

#[test]
fn only_atom_0_3_is_written_anew() {
    // Atom 1.0 that happens to say "issued" is read as it is.
    let atom_1 = r#"<feed xmlns="http://www.w3.org/2005/Atom"><title>One</title>
        <entry><id>a</id><title>A</title><issued>2020-01-01T00:00:00Z</issued>
        <updated>2026-11-09T08:00:00Z</updated><content type="html">&lt;p&gt;A&lt;/p&gt;</content></entry></feed>"#;
    let parsed = parse(atom_1.as_bytes());
    assert_eq!(
        parsed.articles[0].published_at,
        datetime!(2026-11-09 08:00 UTC)
    );
    // Atom 0.3 that is no well-formed XML goes to the library as it is, which
    // says what it says about it.
    let broken = ATOM_03.replace("</feed>", "");
    let parsed = feeds::parse(
        broken.as_bytes(),
        "https://example.org/feed.xml",
        NOW,
        &Rules::built_in().unwrap(),
    );
    assert!(parsed.map_or(true, |parsed| parsed.articles.len() <= 2));
    // A prefix for Atom 0.3 instead of the default namespace.
    let prefixed = r#"<a:feed xmlns:a="http://purl.org/atom/ns#"><a:title>Prefixed</a:title>
        <a:entry><a:id>p</a:id><a:title>P</a:title><a:issued>2026-11-09T07:00:00Z</a:issued>
        <a:content mode="escaped">&lt;p&gt;P&lt;/p&gt;</a:content></a:entry></a:feed>"#;
    let parsed = parse(prefixed.as_bytes());
    assert_eq!(parsed.title.as_deref(), Some("Prefixed"));
    assert_eq!(
        parsed.articles[0].published_at,
        datetime!(2026-11-09 07:00 UTC)
    );
    assert_eq!(parsed.articles[0].html, "<p>P</p>");
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
        assert!(
            feeds::parse(
                nonsense,
                "https://example.org/",
                NOW,
                &Rules::built_in().unwrap()
            )
            .is_err()
        );
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
        after: None,
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
        after: None,
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

#[test]
fn texts_come_out_of_a_feed_cleaned() {
    let text = r##"<p>Read <a href="/more?utm_source=feed&amp;page=2">more</a> or go <a href="#top">up</a>.</p>
        <img src="img/a.png?utm_medium=rss" alt="A">
        <img src="https://count.example.net/p.gif" width="1" height="1">
        <a href="https://www.facebook.com/sharer.php?u=x">Share</a>
        <video controls><source src="clip.mp4" type="video/mp4"></video>
        <script>track()</script>"##;
    let body = format!(
        "<?xml version=\"1.0\"?><rss version=\"2.0\"><channel><title>Example</title>\
         <link>https://example.org/?utm_source=feed</link><item>\
         <link>https://example.org/posts/1?utm_campaign=x&amp;id=7</link><title>One</title>\
         <enclosure url=\"https://example.org/posts/clip.mp4\" length=\"5\" type=\"video/mp4\"/>\
         <enclosure url=\"https://example.org/one.mp3?utm_source=feed\" length=\"9\" type=\"audio/mpeg\"/>\
         <description><![CDATA[{text}]]></description></item></channel></rss>"
    );
    let parsed = feeds::parse(
        body.as_bytes(),
        "https://feeds.example.net/example.xml",
        NOW,
        &Rules::built_in().unwrap(),
    )
    .unwrap();
    assert_eq!(parsed.site_url.as_deref(), Some("https://example.org/"));
    let article = &parsed.articles[0];

    // The link loses what only tracks. The id is the link as the feed gave it,
    // which is what the old rueder knows the article by.
    assert_eq!(
        article.original.url.as_deref(),
        Some("https://example.org/posts/1?id=7")
    );
    assert_eq!(
        article.guid,
        "https://example.org/posts/1?utm_campaign=x&id=7"
    );
    // What is kept as the version is the text as it came.
    assert_eq!(article.original.content, text);
    // Addresses are taken from the article's page, not from where the feed lies.
    assert_eq!(
        article
            .html
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
        "<p>Read <a href=\"https://example.org/more?page=2\" target=\"_blank\" \
         rel=\"nofollow noopener noreferrer\">more</a> or go \
         <a href=\"https://example.org/posts/1?id=7#top\" target=\"_blank\" \
         rel=\"nofollow noopener noreferrer\">up</a>.</p> \
         <img src=\"https://example.org/posts/img/a.png\" alt=\"A\" title=\"A\" loading=\"lazy\">"
    );
    assert_eq!(article.teaser, "Read more or go up. Share");
    // The feed's own enclosures, then what stood in the text and is not among them.
    assert_eq!(
        article.enclosures,
        serde_json::json!([
            { "url": "https://example.org/posts/clip.mp4", "type": "video/mp4", "length": 5 },
            { "url": "https://example.org/one.mp3", "type": "audio/mpeg", "length": 9 },
        ])
    );

    // An article without a page of its own takes addresses from the feed.
    let body = "<?xml version=\"1.0\"?><rss version=\"2.0\"><channel><item><guid>a</guid>\
        <description><![CDATA[<img src=\"a.png\"><audio src=\"/a.mp3\"></audio>]]></description>\
        </item></channel></rss>";
    let parsed = feeds::parse(
        body.as_bytes(),
        "https://feeds.example.net/dir/example.xml",
        NOW,
        &Rules::built_in().unwrap(),
    )
    .unwrap();
    let article = &parsed.articles[0];
    assert_eq!(
        article.html,
        "<img src=\"https://feeds.example.net/dir/a.png\" loading=\"lazy\">"
    );
    assert_eq!(
        article.enclosures,
        serde_json::json!([{ "url": "https://feeds.example.net/a.mp3", "type": null, "inline": true }])
    );
}

#[test]
fn a_feed_that_means_harm_is_read_all_the_same() {
    // Tags nested far deeper than any article has them, in every place that
    // is read as HTML.
    let deep = format!(
        "{}words{}",
        "<div><b>".repeat(20_000),
        "</b></div>".repeat(20_000)
    );
    let body = format!(
        "<?xml version=\"1.0\"?><rss version=\"2.0\"><channel><title><![CDATA[{deep}]]></title>\
         <item><guid>a</guid><title><![CDATA[{deep}]]></title>\
         <description><![CDATA[{deep}]]></description></item></channel></rss>"
    );
    let started = std::time::Instant::now();
    let parsed = parse(body.as_bytes());
    assert!(started.elapsed() < std::time::Duration::from_secs(20));
    assert_eq!(parsed.title.as_deref(), Some("words"));
    let article = &parsed.articles[0];
    assert_eq!(article.original.title, "words");
    assert_eq!(article.teaser, "words");
    // The text is kept, the markup that cannot be trusted is not.
    assert_eq!(article.html.trim(), "words");
}

#[sqlx::test]
async fn a_tracking_token_that_changes_is_no_new_version(db: PgPool) {
    let (account, feed) = common::reader(&db, "ada", 0).await;
    let store = async |token: &str, text: &str| {
        let content = format!(
            "<p>{text} <a href=\"https://example.org/a?utm_source={token}\">more</a></p>\
             <img src=\"https://count.example.net/p.gif?t={token}\" width=\"1\" height=\"1\">"
        );
        let parsed = parse(&rss(&[("a", "Title", &content)]));
        let mut tx = db.begin().await.unwrap();
        let stored = feeds::store(&mut tx, feed, &parsed.articles).await.unwrap();
        tx.commit().await.unwrap();
        stored
    };
    assert_eq!(store("first", "Text.").await.new, 1);
    // Sent differently on every fetch, read the same: nothing happens.
    assert_eq!(store("second", "Text.").await, Stored::default());
    assert_eq!(store("third", "Text.").await, Stored::default());
    let versions = articles::versions(&db, account, feed, 1).await.unwrap();
    assert_eq!(versions.len(), 1);
    assert!(versions[0].original.content.contains("utm_source=first"));

    // A real change is still one, and its version keeps the text as it came.
    assert_eq!(store("fourth", "Corrected text.").await.updated, 1);
    let versions = articles::versions(&db, account, feed, 1).await.unwrap();
    assert_eq!(versions.len(), 2);
    assert!(versions[1].original.content.contains("utm_source=fourth"));
    let opened = articles::open(&db, account, feed, 1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        opened.html,
        "<p>Corrected text. <a href=\"https://example.org/a\" target=\"_blank\" \
         rel=\"nofollow noopener noreferrer\">more</a></p>"
    );
}

#[sqlx::test]
async fn other_rules_are_no_new_version(db: PgPool) {
    let (account, feed) = common::reader(&db, "ada", 0).await;
    let content = "<p>Text. <a href=\"https://example.org/a?ref=feed\">more</a></p>\
                   <img src=\"https://ads.example.net/banner.png\">";
    let body = rss(&[("a", "Title", content)]);
    let store = async |rules: &Rules| {
        let parsed = feeds::parse(&body, "https://example.org/feed.xml", NOW, rules).unwrap();
        let mut tx = db.begin().await.unwrap();
        let stored = feeds::store(&mut tx, feed, &parsed.articles).await.unwrap();
        tx.commit().await.unwrap();
        stored
    };
    let text = async || {
        let opened = articles::open(&db, account, feed, 1).await.unwrap();
        opened.unwrap().html
    };
    assert_eq!(store(&Rules::built_in().unwrap()).await.new, 1);
    assert!(text().await.contains("banner.png"));

    // The operator's lists grow. The article was sent as before, so what is
    // read is put right, and that is all.
    let stricter = Rules::parse("ads.example.net\n", "ref\n").unwrap();
    assert_eq!(store(&stricter).await, Stored::default());
    assert_eq!(
        text().await,
        "<p>Text. <a href=\"https://example.org/a\" target=\"_blank\" \
         rel=\"nofollow noopener noreferrer\">more</a></p>"
    );
    assert_eq!(
        articles::versions(&db, account, feed, 1)
            .await
            .unwrap()
            .len(),
        1
    );
    let page = Page {
        before: None,
        after: None,
        limit: 10,
        unread_only: false,
    };
    let list = articles::page(&db, account, &[feed], page).await.unwrap();
    assert_eq!((list[0].version, list[0].updated_at), (1, None));
    // From here on nothing is left to do.
    assert_eq!(store(&stricter).await, Stored::default());
}
