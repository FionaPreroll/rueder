//! Feeds laid out to take the server's memory: however the largest body a
//! fetch takes is filled, reading it stays within a small budget. Without
//! the limits of `feeds::limited`, 20 MiB of empty entries took 5.8 GiB, and
//! as a JSON Feed 11.6 GiB.
//!
//! One test, as the count of the allocator is shared by the whole program.
//! Past the budget, an allocation fails and ends the program.

use std::alloc::System;

use cap::Cap;
use rueder::{
    cleaning::Rules,
    feeds::{self, MAX_ARTICLES},
    http_client,
};
use time::macros::datetime;

#[global_allocator]
static ALLOCATOR: Cap<System> = Cap::new(System, usize::MAX);

/// What reading one feed may take on top of its bytes.
const BUDGET: usize = 64 * 1024 * 1024;

/// `unit` over and over between `head` and `tail`, as long as a fetch takes.
fn filled(head: &str, unit: &str, tail: &str) -> Vec<u8> {
    let size = http_client::Config::default().max_body;
    let times = (size - head.len() - tail.len()) / unit.len();
    [head, &unit.repeat(times), tail].concat().into_bytes()
}

#[test]
fn a_feed_laid_out_to_take_memory_takes_little() {
    let rss = "<rss version=\"2.0\" xmlns:media=\"http://search.yahoo.com/mrss/\"><channel><title>x</title>";
    let item = "<item><guid>a</guid><title>t</title>";
    let atom = "<feed xmlns=\"http://www.w3.org/2005/Atom\"><title>x</title>";
    let json = "{\"version\":\"https://jsonfeed.org/version/1.1\",\"title\":\"x\",\"items\":[";
    let attributes: String = (0..3_000_000).map(|n| format!(" a{n:x}=\"\"")).collect();
    let shapes = [
        filled(rss, "<item/>", "</channel></rss>"),
        filled(rss, "<item><title>t</title></item>", "</channel></rss>"),
        filled(atom, "<entry/>", "</feed>"),
        filled(
            "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" xmlns=\"http://purl.org/rss/1.0/\">",
            "<item><title>t</title></item>",
            "</rdf:RDF>",
        ),
        filled(
            &format!("{rss}{item}"),
            "<enclosure url=\"a\"/>",
            "</item></channel></rss>",
        ),
        filled(
            &format!("{rss}{item}<media:group>"),
            "<media:content url=\"a\"/>",
            "</media:group></item></channel></rss>",
        ),
        filled(rss, "<category>a</category>", "</channel></rss>"),
        format!("{rss}<item{attributes}><title>t</title></item></channel></rss>").into_bytes(),
        filled(json, "{},", "{}]}"),
        filled(
            &format!("{json}{{\"id\":\"a\",\"tags\":["),
            "\"\",",
            "\"\"]}]}",
        ),
        filled(
            &format!("{json}{{\"id\":\"a\",\"attachments\":["),
            "{\"url\":\"a\",\"mime_type\":\"a\"},",
            "{}]}]}",
        ),
    ];
    let rules = Rules::built_in().unwrap();
    for body in shapes {
        ALLOCATOR.set_limit(ALLOCATOR.allocated() + BUDGET).unwrap();
        let parsed = feeds::parse(
            &body,
            "https://example.org/feed.xml",
            datetime!(2026-11-20 12:00 UTC),
            &rules,
        );
        ALLOCATOR.set_limit(usize::MAX).unwrap();
        let parsed = parsed.unwrap();
        assert!(parsed.articles.len() <= MAX_ARTICLES);
    }
}
