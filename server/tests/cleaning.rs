//! The cleaning of article text. The first tests are the cases of the old
//! rueder's `ArticleCleaner.test.ts` and `url.test.ts`, which this replaces.

use std::time::{Duration, Instant};

use proptest::prelude::*;
use rueder::cleaning::{Base, Cleaned, Enclosure, Rules, clean};
use url::Url;

fn rules() -> Rules {
    Rules::built_in().unwrap()
}

const NO_BASE: Base<'static> = Base {
    article: None,
    feed: None,
};

/// An article on a page of its own, from a feed elsewhere.
const BASE: Base<'static> = Base {
    article: Some("https://example.com/article/23?bar=baz"),
    feed: Some("https://feeds.example.net/foo/bar/feed.rss"),
};

fn html(text: &str, base: Base<'_>) -> String {
    clean(text, base, &rules()).html
}

const WITH_ANCHOR_LINKS: &str = r##"
<ul>
<li><a href="#background" rel="nofollow">A bit of background explanation</a></li>
<li><a href="#questions" rel="nofollow">And what to do if you still have questions</a></li>
</ul>
<h3><a name="background" rel="nofollow" id="background"></a>Background explanation</h3>
<h3><a name="questions" rel="nofollow" id="questions"></a>If you still have questions...</h3>
"##;

const WITH_IMAGE: &str = r#"
<p>
    <strong>Die Entwicklung der Kryptowährungen und ihr Einfluss auf die Gesellschaft</strong>
</p>
<div>
  <a target="_blank" href="https://media.example.com/media/stuff/stuff224-stuff-stuff-2.jpg">

<img alt="Episode image forSTUFF224 Stuff 2" width="200" src="https://example.com/podlove/image/stuff" />

  </a>
  <p>
  In Fortsetzung des Gesprächs ...
  </p>
"#;

const WITH_VIDEO: &str = r#"
<p>I couldn't find an example for this so here's an artificial one:</p>
<video width="320" height="240" controls>
  <source src="https://movies.example.com/movie.mp4" type="video/mp4">
  <source src="https://movies.example.com/movie.ogg" type="video/ogg">
  Ancient browser alarm.
</video>
"#;

const WITH_RELATIVE_VIDEO: &str = r#"
<video width="320" height="240" controls>
  <source src="movie.mp4" type="video/mp4; codecs=avc1">
  <source src="/movie.ogg" type="video/ogg">
</video>
"#;

fn enclosure(url: &str, kind: &str) -> Enclosure {
    Enclosure {
        url: url.into(),
        kind: Some(kind.into()),
    }
}

#[test]
fn nothing_stays_nothing() {
    assert_eq!(clean("", NO_BASE, &rules()), Cleaned::default());
    assert_eq!(clean("", BASE, &rules()), Cleaned::default());
}

#[test]
fn plain_text_stays_as_it_is() {
    let text = "foo bar baz\nboo";
    assert_eq!(html(text, NO_BASE), text);
    // What would be markup is shown as the characters it is.
    assert_eq!(html("1 < 2 & 3 > 2", NO_BASE), "1 &lt; 2 &amp; 3 &gt; 2");
}

#[test]
fn links_into_the_article_point_to_its_page() {
    let cleaned = html(WITH_ANCHOR_LINKS, BASE);
    assert!(cleaned.contains(
        r#"<a href="https://example.com/article/23?bar=baz#background" target="_blank" rel="nofollow noopener noreferrer">A bit of background explanation</a>"#
    ), "{cleaned}");
}

#[test]
fn a_link_that_leads_nowhere_leaves_its_words() {
    // Without the article's page, "#background" has nothing to point into.
    let cleaned = html(WITH_ANCHOR_LINKS, NO_BASE);
    assert!(cleaned.contains("<li>A bit of background explanation</li>"));
    assert!(cleaned.contains("<h3>Background explanation</h3>"));
    assert!(!cleaned.contains("<a"));
    assert!(!cleaned.contains("id="));
    assert!(!cleaned.contains("name="));

    assert_eq!(
        html(
            r#"Write to <a href="mailto:me@example.com">me</a> or <a>nobody</a>."#,
            BASE
        ),
        "Write to me or nobody."
    );
}

#[test]
fn images_stay_with_their_whole_address() {
    let cleaned = clean(WITH_IMAGE, NO_BASE, &rules());
    assert!(cleaned.html.contains(
        r#"<img alt="Episode image forSTUFF224 Stuff 2" width="200" src="https://example.com/podlove/image/stuff" title="Episode image forSTUFF224 Stuff 2" loading="lazy">"#
    ), "{}", cleaned.html);
    assert!(cleaned.html.contains(
        r#"<a href="https://media.example.com/media/stuff/stuff224-stuff-stuff-2.jpg" target="_blank" rel="nofollow noopener noreferrer">"#
    ));
    // An image is not an enclosure.
    assert_eq!(cleaned.enclosures, []);
}

#[test]
fn audio_and_video_become_enclosures() {
    let cleaned = clean(WITH_VIDEO, NO_BASE, &rules());
    assert_eq!(
        cleaned.enclosures,
        [
            enclosure("https://movies.example.com/movie.mp4", "video/mp4"),
            enclosure("https://movies.example.com/movie.ogg", "video/ogg"),
        ]
    );
    // The player is gone, with what it would say to browsers that cannot play.
    assert!(!cleaned.html.contains("video"));
    assert!(!cleaned.html.contains("source"));
    assert!(!cleaned.html.contains("Ancient browser alarm"));
    assert!(cleaned.html.contains("<p>I couldn't find an example"));
    assert_eq!(clean(WITH_ANCHOR_LINKS, BASE, &rules()).enclosures, []);

    // Relative addresses: from the feed where the article has no page ...
    let feed_only = Base {
        article: None,
        ..BASE
    };
    assert_eq!(
        clean(WITH_RELATIVE_VIDEO, feed_only, &rules()).enclosures,
        [
            enclosure("https://feeds.example.net/foo/bar/movie.mp4", "video/mp4"),
            enclosure("https://feeds.example.net/movie.ogg", "video/ogg"),
        ]
    );
    // ... from the article's page where it has one ...
    assert_eq!(
        clean(WITH_RELATIVE_VIDEO, BASE, &rules()).enclosures,
        [
            enclosure("https://example.com/article/movie.mp4", "video/mp4"),
            enclosure("https://example.com/movie.ogg", "video/ogg"),
        ]
    );
    // ... and none at all where there is nothing to take them from.
    assert_eq!(clean(WITH_RELATIVE_VIDEO, NO_BASE, &rules()).enclosures, []);

    // The tags' own addresses count too, each file once, never a blocked one.
    let cleaned = clean(
        r#"<audio src="https://a.example/1.mp3?utm_source=feed"></audio>
           <audio controls><source src="https://a.example/1.mp3"><source src="javascript:alert(1)">
           <source src="https://cdn.facebook.com/2.mp3"></audio>
           <picture><source srcset="https://a.example/big.webp"><img src="https://a.example/small.jpg"></picture>"#,
        NO_BASE,
        &rules(),
    );
    assert_eq!(
        cleaned.enclosures,
        [Enclosure {
            url: "https://a.example/1.mp3".into(),
            kind: None
        }]
    );
    // A picture keeps the image every browser can show.
    assert_eq!(
        cleaned.html.trim(),
        r#"<img src="https://a.example/small.jpg" loading="lazy">"#
    );
}

#[test]
fn tracking_parameters_go() {
    let rules = rules();
    for (address, clean) in [
        // heise.de
        (
            "https://www.example.com/foo/KI-erspaeht-7092096.html?wt_mc=rss.red.ho.ho.atom.beitrag.beitrag",
            "https://www.example.com/foo/KI-erspaeht-7092096.html",
        ),
        // Slashdot
        (
            "https://tech.example.com/story/22/05/14/2026232/should-social?utm_source=rss1.0mainlinkanon&utm_medium=feed",
            "https://tech.example.com/story/22/05/14/2026232/should-social",
        ),
        // medium.com
        (
            "https://example.com/episode-155-how-the-cafef00db4d?source=rss-abcdef010142------2",
            "https://example.com/episode-155-how-the-cafef00db4d",
        ),
        // The Guardian
        (
            "https://www.example.com/search?entry=10000101994&utm_source=guardian-feast&utm_medium=referral&utm_campaign=guardian-feast-recipe",
            "https://www.example.com/search?entry=10000101994",
        ),
        (
            "https://www.example.com/culture/series/saved-for-later?CMP=dcba_sfl",
            "https://www.example.com/culture/series/saved-for-later",
        ),
        // Facebook, and parameters that only look alike.
        (
            "https://example.com/a?fbclid=abc&from=rss&from=home&source=web&wt=1&cmp=x&utm=1#part",
            "https://example.com/a?from=home&source=web&wt=1&cmp=x&utm=1#part",
        ),
        // What stays is not rewritten, and a link that ends in "?" or "?&" loses that.
        (
            "https://example.com/a?q=a+b%20c&empty=&flag&x=1=2",
            "https://example.com/a?q=a+b%20c&empty=&flag&x=1=2",
        ),
        ("https://example.com/a?", "https://example.com/a"),
        (
            "https://example.com/a?&foo=bar&&=x",
            "https://example.com/a?foo=bar",
        ),
        (
            "https://example.com/a?utm_source=x#top",
            "https://example.com/a#top",
        ),
        // Not web addresses, or not addresses at all: left alone.
        (
            "mailto:me@example.com?utm_source=x",
            "mailto:me@example.com?utm_source=x",
        ),
        ("not an address?utm_source=x", "not an address?utm_source=x"),
        ("https://example.com/plain", "https://example.com/plain"),
    ] {
        assert_eq!(rules.clean_url(address), clean);
    }

    assert_eq!(
        html(
            r#"<a href="https://example.com/a?utm_source=feed&amp;id=7">read</a>
               <img src="https://example.com/a.png?utm_medium=rss">"#,
            NO_BASE
        ),
        r#"<a href="https://example.com/a?id=7" target="_blank" rel="nofollow noopener noreferrer">read</a>
               <img src="https://example.com/a.png" loading="lazy">"#
    );
}

#[test]
fn share_buttons_and_counters_go() {
    let rules = rules();
    let blocks = |address: &str| rules.blocks(&Url::parse(address).unwrap());
    assert!(blocks("https://a.fsdn.com/sd/twitter_icon_large.png"));
    assert!(blocks("http://FSDN.com/"));
    assert!(blocks("https://www.facebook.com/sharer.php?u=x"));
    assert!(blocks("https://twitter.com/home?status=Read+this"));
    assert!(blocks("https://mobile.twitter.com/intent?STATUS=x"));
    assert!(blocks("https://feeds.feedburner.com/~ff/blog?a=xyz:abc"));
    // A profile is not a share button, and a domain is not what it ends in.
    assert!(!blocks("https://twitter.com/rueder"));
    assert!(!blocks("https://feeds.feedburner.com/blog"));
    assert!(!blocks("https://notfacebook.com/"));
    assert!(!blocks("https://facebook.com.example.org/"));
    assert!(!blocks("https://example.org/facebook.com"));

    assert_eq!(
        html(
            r#"<p>Read on.</p><p><a href="https://www.facebook.com/sharer.php?u=x">Share <b>this</b></a><img src="https://a.fsdn.com/sd/facebook_icon_large.png" alt="Facebook"></p>"#,
            NO_BASE
        ),
        "<p>Read on.</p>"
    );
    // Pixels that only count readers.
    assert_eq!(
        html(
            r#"<p>Text.<img src="https://cpx.example.com/pixel.gif" width="1" height="1" alt="">
               <img src="https://vg.example.com/count" height=" 0 " width="0"></p>
               <img src="https://example.com/line.png" width="400" height="1">"#,
            NO_BASE
        )
        .trim(),
        "<p>Text. </p>\n               <img src=\"https://example.com/line.png\" width=\"400\" height=\"1\" loading=\"lazy\">"
    );
}

#[test]
fn relative_addresses_become_whole_or_go() {
    assert_eq!(
        html(
            r#"<a href="/about">about</a> <a href="next?page=2">next</a> <a href="//cdn.example.org/x">cdn</a>
               <img src="../img/a.png" alt="A"> <img src="b.png">"#,
            BASE
        ),
        r#"<a href="https://example.com/about" target="_blank" rel="nofollow noopener noreferrer">about</a> <a href="https://example.com/article/next?page=2" target="_blank" rel="nofollow noopener noreferrer">next</a> <a href="https://cdn.example.org/x" target="_blank" rel="nofollow noopener noreferrer">cdn</a>
               <img src="https://example.com/img/a.png" alt="A" title="A" loading="lazy"> <img src="https://example.com/article/b.png" loading="lazy">"#
    );
    // With nothing to take them from, they would point into rueder itself.
    assert_eq!(
        html(
            r#"<a href="/api/logout">out</a><img src="/api/logout"><img src="x.png"><img>"#,
            NO_BASE
        ),
        "out"
    );
    // A base that is no web address is no base.
    let odd = Base {
        article: Some("javascript:alert(1)"),
        feed: Some("file:///etc/passwd"),
    };
    assert_eq!(html(r##"<a href="#x">x</a><img src="shadow">"##, odd), "x");
}

#[test]
fn only_harmless_markup_stays() {
    for (dirty, clean) in [
        ("<p>a<script>alert(1)</script>b</p>", "<p>ab</p>"),
        (
            "<style>p { color: red }</style><p style=\"color:red\" class=\"x\" id=\"y\">a</p>",
            "<p>a</p>",
        ),
        (
            "<p onclick=\"alert(1)\" title=\"T\" lang=\"de\" dir=\"rtl\">a</p>",
            "<p title=\"T\" lang=\"de\" dir=\"rtl\">a</p>",
        ),
        ("<a href=\"javascript:alert(1)\">a</a>", "a"),
        ("<a href=\" JaVaScRiPt:alert(1)\">a</a>", "a"),
        (
            "<a href=\"data:text/html,<script>alert(1)</script>\">a</a>",
            "a",
        ),
        (
            "<img src=\"data:image/png;base64,AAAA\" onerror=\"alert(1)\">",
            "",
        ),
        (
            "<img src=\"https://example.com/a.png\" onerror=\"alert(1)\" srcset=\"https://example.com/b.png 2x\">",
            "<img src=\"https://example.com/a.png\" loading=\"lazy\">",
        ),
        (
            "<form action=\"https://example.com/\"><input name=\"q\"><button>Go</button></form>",
            "Go",
        ),
        (
            "<iframe src=\"https://example.com/\"></iframe>after",
            "after",
        ),
        (
            "<object data=\"https://example.com/x.swf\"></object><embed src=\"https://example.com/x.swf\">x",
            "x",
        ),
        (
            "<svg><script>alert(1)</script><a xlink:href=\"javascript:alert(1)\">x</a></svg>",
            "x",
        ),
        (
            "<math><mi xlink:href=\"javascript:alert(1)\">x</mi></math>",
            "x",
        ),
        (
            "<base href=\"https://evil.example/\"><link rel=\"stylesheet\" href=\"https://evil.example/x.css\"><meta http-equiv=\"refresh\" content=\"0;url=https://evil.example/\">x",
            "x",
        ),
        (
            "<a href=\"https://example.com/\" target=\"_top\" rel=\"opener\" ping=\"https://example.com/ping\">a</a>",
            "<a href=\"https://example.com/\" target=\"_blank\" rel=\"nofollow noopener noreferrer\">a</a>",
        ),
        ("a<!-- a comment -->b<![CDATA[ c ]]>", "ab"),
        (
            "<article><section><h2>T</h2><center>c</center></section></article>",
            "<h2>T</h2>c",
        ),
        (
            "<table><tr><td colspan=\"2\" bgcolor=\"red\">a</td></tr></table>",
            "<table><tbody><tr><td colspan=\"2\">a</td></tr></tbody></table>",
        ),
        (
            "<details><summary>More</summary><ol start=\"3\"><li>x</li></ol></details>",
            "<details><summary>More</summary><ol start=\"3\"><li>x</li></ol></details>",
        ),
    ] {
        assert_eq!(html(dirty, BASE), clean, "{dirty}");
    }
}

#[test]
fn what_is_left_empty_goes_and_white_space_is_single() {
    assert_eq!(
        html(
            "<p>one</p><p></p><p> \n </p><span>\t</span><p><span></span></p><p>two</p>",
            NO_BASE
        ),
        "<p>one</p><p>two</p>"
    );
    // A paragraph with something to show in it is not empty.
    assert_eq!(
        html(
            "<p><img src=\"https://example.com/a.png\"></p><p><br></p>",
            NO_BASE
        ),
        "<p><img src=\"https://example.com/a.png\" loading=\"lazy\"></p><p><br></p>"
    );
    assert_eq!(
        html(
            "<p>a<br><br/> <br>\n<br>b<br>c</p><div>d<br><span> </span><br>e</div>",
            NO_BASE
        ),
        "<p>a<br> b<br>c</p><div>d<br>e</div>"
    );
    assert_eq!(
        html(
            "<p>\n    some   <b>bold\n\t  words</b>  \n</p><pre>  keep\n\tthis  </pre>",
            NO_BASE
        ),
        "<p> some <b>bold words</b> </p><pre>  keep\n\tthis  </pre>"
    );
}

/// Texts that have a bit of everything.
fn samples() -> Vec<String> {
    let mut samples: Vec<String> = [
        WITH_ANCHOR_LINKS,
        WITH_IMAGE,
        WITH_VIDEO,
        WITH_RELATIVE_VIDEO,
        "<p>a<br><br><br>b</p><span> </span><p><a href=\"?utm_source=x\">x</a></p>",
        "<table><td><a href=x><p><b><i>unclosed <img src=/a.png alt='\"<>&'>",
        "<svg><style><a href='</style><img src=x onerror=alert(1)>'></svg><p title='a\"b'>x",
        "<a href='https://example.com/?a=1&amp;utm_x=2&b=<>'>&lt;script&gt;alert(1)&lt;/script&gt;</a>",
        "plain & simple < text\n\nwith lines",
    ]
    .map(String::from)
    .into();
    samples.push(format!("{WITH_IMAGE}{WITH_VIDEO}{WITH_ANCHOR_LINKS}"));
    samples
}

/// What every cleaned text keeps to, whatever went in.
fn assert_clean(cleaned: &str, from: &str) {
    let lower = cleaned.to_ascii_lowercase();
    for gone in [
        "<script",
        "<style",
        "<iframe",
        "<form",
        "<input",
        "<object",
        "<embed",
        "<video",
        "<audio",
        "<source",
        "<svg",
        "<math",
        "<link",
        "<meta",
        "<base",
        "<template",
        "<!--",
    ] {
        assert!(!lower.contains(gone), "{gone} in {cleaned:?} from {from:?}");
    }
    for tag in tags(&lower) {
        // Attributes that run code, load more than was asked for or change the page.
        for gone in [
            " on", " srcset=", " style=", " class=", " id=", " name=", " ping=",
        ] {
            assert!(!tag.contains(gone), "{gone} in {cleaned:?} from {from:?}");
        }
        // Every address is a whole web address.
        for attribute in [" href=\"", " src=\""] {
            if let Some((_, address)) = tag.split_once(attribute) {
                assert!(
                    address.starts_with("http://") || address.starts_with("https://"),
                    "{tag} in {cleaned:?} from {from:?}"
                );
            }
        }
    }
}

/// The tags of a cleaned text, which writes `<` in text and attributes as `&lt;`.
fn tags(html: &str) -> impl Iterator<Item = &str> {
    html.split('<')
        .skip(1)
        .filter_map(|rest| rest.split('>').next())
}

#[test]
fn cleaning_again_changes_nothing() {
    for sample in samples() {
        for base in [BASE, NO_BASE] {
            let once = clean(&sample, base, &rules());
            assert_clean(&once.html, &sample);
            let twice = clean(&once.html, base, &rules());
            assert_eq!(twice.html, once.html, "{sample}");
            assert_eq!(twice.enclosures, []);
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    /// Texts thrown together from pieces of markup, most of them broken.
    #[test]
    fn any_text_comes_out_clean(pieces in prop::collection::vec(prop::sample::select(vec![
        "<p>", "</p>", "<a href=\"", "<a href='#x'>", "</a>", "<img src=\"", "\">", "'>", "<span>", "</span>",
        "<br>", "<video src=x>", "<source src=\"https://e.example/v.mp4\">", "</video>", "<script>", "</script>",
        "<style>", "</style>", "<svg>", "</svg>", "<table>", "<td>", "<pre>", "</pre>", "<!--", "-->", "<b>",
        "</b>", "<div onclick=alert(1)>", "</div>", "javascript:alert(1)", "https://example.com/a?utm_source=x&b=1",
        "//cdn.example.org/a.png", "/relative", "https://www.facebook.com/share", " width=1 height=1 ",
        "text", " ", "\n", "&amp;", "&lt;", "<", ">", "\"", "'", "=", "<textarea>", "<title>", "<noscript>",
        "<template>", "<plaintext>", "<math><mtext>", "<select><option>", "\u{0}",
    ]), 0..40)) {
        let text: String = pieces.concat();
        let rules = rules();
        for base in [BASE, NO_BASE] {
            let once = clean(&text, base, &rules);
            assert_clean(&once.html, &text);
            for found in &once.enclosures {
                prop_assert!(found.url.starts_with("http"));
            }
            let twice = clean(&once.html, base, &rules);
            prop_assert_eq!(&twice.html, &once.html, "from {:?}", text);
        }
    }
}

#[test]
fn deep_and_long_texts_do_not_hang() {
    let deep = |open: &str, depth: usize| format!("{}x", open.repeat(depth));
    let megabyte = 1024 * 1024;
    for (name, text) in [
        ("nested divs", deep("<div>", megabyte / 5)),
        (
            "nested links",
            deep("<a href=\"https://example.com/\">", 20_000),
        ),
        ("nested spans", deep("<span>", megabyte / 6)),
        ("nested formatting", deep("<b><i><u>", 30_000)),
        ("nested tables", deep("<table><tr><td>", 20_000)),
        ("nested lists", deep("<ul><li>", megabyte / 8)),
        ("nested players", deep("<video><audio>", 50_000)),
        (
            "many images",
            "<img src=\"https://example.com/a.png?utm_source=x\">".repeat(megabyte / 48),
        ),
        ("many breaks", "<br>".repeat(megabyte / 4)),
        ("many comments", "<!---->".repeat(megabyte / 7)),
        ("one long word", "x".repeat(megabyte)),
    ] {
        let started = Instant::now();
        let cleaned = clean(&text, BASE, &rules());
        let took = started.elapsed();
        assert!(took < Duration::from_secs(20), "{name} took {took:?}");
        assert_clean(&cleaned.html, name);
        println!(
            "{name}: {} bytes in, {} out, {took:?}",
            text.len(),
            cleaned.html.len()
        );
    }
}

#[test]
fn the_lists_are_read_from_files() {
    let rules = Rules::parse(
        "# a comment\n\n  Example.ORG  \nshare.example /Button\n",
        "ref\nmc_*\nvia=feed\nsrc=*news*\n",
    )
    .unwrap();
    let blocks = |address: &str| rules.blocks(&Url::parse(address).unwrap());
    assert!(blocks("https://www.example.org/"));
    assert!(blocks("https://share.example/x/button?id=1"));
    assert!(!blocks("https://share.example/x"));
    assert!(!blocks("https://www.facebook.com/"));
    assert_eq!(
        rules.clean_url(
            "https://a.example/?ref=1&mc_id=2&via=feed&via=web&src=thenewsletter&src=x&utm_source=y"
        ),
        "https://a.example/?via=web&src=x&utm_source=y"
    );

    for (blocked, tracking) in [
        ("not a domain!", ""),
        ("example.org two pieces", ""),
        ("https://example.org/", ""),
        ("", "a b"),
        ("", "=value"),
        ("", "*"),
        ("", "name="),
        ("", "na*me"),
        ("", "name=**"),
        ("", "name=a&b"),
    ] {
        assert!(
            Rules::parse(blocked, tracking).is_err(),
            "{blocked:?} {tracking:?}"
        );
    }

    // An operator's own lists, in place of the ones that come with rueder.
    let dir = std::env::temp_dir().join(format!("rueder-rules-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    assert!(Rules::load(&dir).is_err());
    std::fs::write(dir.join("blocked-links.txt"), "ads.example\n").unwrap();
    std::fs::write(dir.join("tracking-parameters.txt"), "").unwrap();
    let own = Rules::load(&dir).unwrap();
    assert!(own.blocks(&Url::parse("https://ads.example/1.gif").unwrap()));
    assert_eq!(
        own.clean_url("https://a.example/?utm_source=y"),
        "https://a.example/?utm_source=y"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_feeds_a_page_names_are_found_within_limits() {
    use rueder::cleaning::{MAX_FEED_LINKS, MAX_NODES, feed_links};
    let base = Url::parse("https://blog.example/posts/").unwrap();
    let urls = |html: &str| -> Vec<String> {
        let links = feed_links(html, &base).into_iter();
        links.map(|link| link.url.into()).collect()
    };
    let link = |rel: &str, kind: &str, href: &str| {
        format!("<link rel=\"{rel}\" type=\"{kind}\" href=\"{href}\">")
    };
    let rss = "application/rss+xml";

    assert_eq!(urls(""), Vec::<String>::new());
    assert_eq!(
        urls(&link("alternate", rss, "../feed")),
        ["https://blog.example/feed"]
    );
    // Not an alternate, not a feed, no address, or none of the web.
    for (rel, kind, href) in [
        ("stylesheet", rss, "/feed"),
        ("alternate", "text/html", "/feed"),
        ("alternate", rss, "data:application/rss+xml,x"),
        ("alternate", rss, "javascript:alert(1)"),
        ("alternate", rss, "https://"),
    ] {
        assert_eq!(
            urls(&link(rel, kind, href)),
            Vec::<String>::new(),
            "{rel} {kind} {href}"
        );
    }
    let no_href = "<link rel=\"alternate\" type=\"application/rss+xml\">";
    assert_eq!(urls(no_href), Vec::<String>::new());
    // Links in the text of a page are not the page's word.
    let anchor = "<a rel=\"alternate\" type=\"application/rss+xml\" href=\"/feed\">Feed</a>";
    assert_eq!(urls(anchor), Vec::<String>::new());

    // A name is a text of one line and no more than fits.
    let named = format!(
        "<link rel=\"alternate\" type=\"{rss}\" href=\"/feed\" title=\" A\n {} \">",
        "b".repeat(500)
    );
    let title = &feed_links(&named, &base)[0].title;
    assert!(title.starts_with("A bbb"));
    assert_eq!(title.chars().count(), 200);

    // No more than the limit, in the order of the page.
    let many: String = (0..MAX_FEED_LINKS + 5)
        .map(|n| link("alternate", rss, &format!("/feed/{n}")))
        .collect();
    let found = urls(&many);
    assert_eq!(found.len(), MAX_FEED_LINKS);
    assert_eq!(found[0], "https://blog.example/feed/0");
    assert_eq!(found[1], "https://blog.example/feed/1");
    // A page beyond what the parser reads has none, and takes no time.
    let started = Instant::now();
    let huge = format!(
        "{}{}",
        "<i></i>".repeat(MAX_NODES),
        link("alternate", rss, "/feed")
    );
    assert_eq!(urls(&huge), Vec::<String>::new());
    let deep = format!(
        "{}{}",
        "<div>".repeat(100_000),
        link("alternate", rss, "/feed")
    );
    assert_eq!(urls(&deep), Vec::<String>::new());
    assert!(started.elapsed() < Duration::from_secs(20));
}

#[test]
fn the_first_picture_of_a_text_is_the_first_that_is_not_small() {
    use rueder::cleaning::first_image;
    let base = Base {
        article: Some("https://example.org/post/1"),
        feed: None,
    };
    let text = html(
        "<p>Hi <img src=\"/smiley.png\" width=\"20\"></p>\
         <figure><img src=\"/big.jpg\" alt=\"Big\"><figcaption>Big</figcaption></figure>\
         <img src=\"/second.jpg\">",
        base,
    );
    assert_eq!(
        first_image(&text).as_deref(),
        Some("https://example.org/big.jpg"),
        "{text}"
    );
    // A size that is no number says nothing.
    assert_eq!(
        first_image("<img src=\"https://e.example/a.png\" width=\"wide\">").as_deref(),
        Some("https://e.example/a.png")
    );
    for none in [
        "",
        "words only",
        "<img alt=\"no address\">",
        "<img src=\"https://e.example/i.png\" height=\"99\">",
    ] {
        assert_eq!(first_image(none), None, "{none}");
    }
    // A tree too deep to read is read as words.
    let deep = format!(
        "{}<img src=\"https://e.example/x.png\">",
        "<div>".repeat(100_000)
    );
    let _ = first_image(&deep);
}
