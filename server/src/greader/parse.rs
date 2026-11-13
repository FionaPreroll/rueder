//! What a request of the Google Reader API says, read without the database:
//! its parameters, stream IDs, item IDs, continuations, times and counts.
//!
//! Everything here takes text from apps of other makers, so everything has a
//! limit and nothing panics. The fuzz target `greader` holds this module to
//! that, and to IDs that read back as what they were written as.

use time::OffsetDateTime;

/// How many parameters a request may bring: a thousand item IDs and what
/// goes with them.
pub const MAX_PAIRS: usize = 2048;
/// The longest value of a parameter, in bytes: an address of a feed, or the
/// path of a folder.
pub const MAX_VALUE: usize = 2048;
/// How deep folders nest, as in the tree.
pub const MAX_DEPTH: usize = 5;
/// The longest name of a folder, in characters, as in the tree.
pub const MAX_NAME: usize = 100;
/// The prefix of an item ID in its long form.
pub const ITEM_PREFIX: &str = "tag:google.com,2005:reader/item/";
/// What stands in a path for a `/` inside the name of a folder: the division
/// slash, U+2215.
pub const SLASH: char = '\u{2215}';

/// Why the parameters of a request cannot be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    TooMany,
    TooLong,
}

/// The parameters of a request, from the query and from a form body, in the
/// order they came. Names may repeat, such as `i` for item IDs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Params(Vec<(String, String)>);

impl Params {
    /// Reads the query and the body, within [`MAX_PAIRS`] and [`MAX_VALUE`].
    pub fn parse(query: &str, body: &[u8]) -> Result<Self, Refusal> {
        let mut pairs = Vec::new();
        for source in [query.as_bytes(), body] {
            for (name, value) in url::form_urlencoded::parse(source) {
                if pairs.len() == MAX_PAIRS {
                    return Err(Refusal::TooMany);
                }
                if name.len() > MAX_VALUE || value.len() > MAX_VALUE {
                    return Err(Refusal::TooLong);
                }
                pairs.push((name.into_owned(), value.into_owned()));
            }
        }
        Ok(Self(pairs))
    }

    /// Every parameter, name and value, in order.
    pub fn pairs(&self) -> &[(String, String)] {
        &self.0
    }

    /// The first value of a name.
    pub fn one(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// Every value of a name, in order.
    pub fn all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.0
            .iter()
            .filter(move |(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// Whether the app wants the answer as JSON.
    pub fn json(&self) -> bool {
        self.one("output") == Some("json")
    }
}

/// A state the Google Reader API knows an article to be in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Every article: the reading list.
    ReadingList,
    Read,
    Starred,
    /// Unread although under the read mark.
    KeptUnread,
    /// States that rueder does not have, such as `broadcast` and `like`.
    /// They are taken and change nothing.
    Other,
}

impl State {
    fn named(name: &str) -> Self {
        match name {
            "reading-list" => Self::ReadingList,
            "read" => Self::Read,
            "starred" => Self::Starred,
            "kept-unread" => Self::KeptUnread,
            _ => Self::Other,
        }
    }

    /// The stream ID of a state rueder has.
    pub fn id(self) -> Option<&'static str> {
        match self {
            Self::ReadingList => Some("user/-/state/com.google/reading-list"),
            Self::Read => Some("user/-/state/com.google/read"),
            Self::Starred => Some("user/-/state/com.google/starred"),
            Self::KeptUnread => Some("user/-/state/com.google/kept-unread"),
            Self::Other => None,
        }
    }
}

/// How a request names a feed: by its number, as the answers do, or by its
/// address, as apps do to subscribe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedRef {
    Id(i32),
    Url(String),
}

/// A stream: a feed, a folder or label, or a state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stream {
    Feed(FeedRef),
    /// `user/…/label/<name>`: a folder by its path, or a label by its name.
    Label(String),
    State(State),
}

/// Reads a stream ID. The user part may be `-` or anything else; it is
/// always the account that asks.
pub fn stream(id: &str) -> Option<Stream> {
    if id.len() > MAX_VALUE {
        return None;
    }
    if let Some(feed) = id.strip_prefix("feed/") {
        if let Ok(number) = feed.parse::<i32>() {
            return (number > 0).then_some(Stream::Feed(FeedRef::Id(number)));
        }
        return (!feed.is_empty()).then(|| Stream::Feed(FeedRef::Url(feed.to_owned())));
    }
    let rest = id.strip_prefix("user/")?;
    let (_user, rest) = rest.split_once('/')?;
    if let Some(name) = rest.strip_prefix("label/") {
        return (!name.trim().is_empty()).then(|| Stream::Label(name.to_owned()));
    }
    let state = rest.strip_prefix("state/com.google/")?;
    (!state.is_empty()).then(|| Stream::State(State::named(state)))
}

/// The stream ID of a feed.
pub fn feed_id(feed_id: i32) -> String {
    format!("feed/{feed_id}")
}

/// The stream ID of a label or of a folder's path (see [`path_name`]).
pub fn label_id(name: &str) -> String {
    format!("user/-/label/{name}")
}

/// The path of folders a label names, top first: `News/Tech` is the folder
/// `Tech` in `News`. A `/` within a folder's name is written [`SLASH`]. `None`
/// if it cannot be a path: an empty part, more than [`MAX_DEPTH`] parts, or a
/// part longer than a folder's name may be.
pub fn path(name: &str) -> Option<Vec<String>> {
    if name.len() > MAX_VALUE {
        return None;
    }
    let mut parts = Vec::new();
    for part in name.split('/') {
        let part = part.trim().replace(SLASH, "/");
        let part = part.trim();
        if part.is_empty() || part.chars().count() > MAX_NAME || parts.len() == MAX_DEPTH {
            return None;
        }
        parts.push(part.to_owned());
    }
    Some(parts)
}

/// The name a path of folders goes by, the other way round from [`path`].
pub fn path_name(parts: &[String]) -> String {
    parts
        .iter()
        .map(|part| part.replace('/', &SLASH.to_string()))
        .collect::<Vec<_>>()
        .join("/")
}

/// The item ID of an article: its feed and its number there, which never
/// change, as one 64-bit number. Apps keep it; no index is needed to find the
/// article again.
pub fn item_number(feed_id: i32, seq: i32) -> Option<i64> {
    let (feed, seq) = (u32::try_from(feed_id).ok()?, u32::try_from(seq).ok()?);
    i64::try_from((u64::from(feed) << 32) | u64::from(seq)).ok()
}

/// An item ID in its long form, as the contents of items have it.
pub fn long_item_id(number: i64) -> String {
    format!("{ITEM_PREFIX}{number:016x}")
}

/// The feed and number an item ID stands for, in any of the forms apps send:
/// `tag:google.com,2005:reader/item/` with up to 16 hexadecimal digits, the
/// 16 digits alone, or the number in decimal.
pub fn item(id: &str) -> Option<(i32, i32)> {
    let id = id.trim();
    let number = if let Some(hex) = id.strip_prefix(ITEM_PREFIX) {
        hexadecimal(hex)?
    } else if id.len() == 16
        && id.bytes().all(|b| b.is_ascii_hexdigit())
        && (id.starts_with('0') || id.bytes().any(|b| b.is_ascii_alphabetic()))
    {
        hexadecimal(id)?
    } else if !id.is_empty() && id.len() <= 20 && id.bytes().all(|b| b.is_ascii_digit()) {
        id.parse::<u64>().ok()?
    } else {
        return None;
    };
    let feed = i32::try_from(number >> 32).ok()?;
    let seq = i32::try_from(number & u64::from(u32::MAX)).ok()?;
    (feed > 0 && seq > 0).then_some((feed, seq))
}

fn hexadecimal(digits: &str) -> Option<u64> {
    let fits = !digits.is_empty() && digits.len() <= 16;
    if !fits || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u64::from_str_radix(digits, 16).ok()
}

/// A continuation: where the next page starts, the order of arrival of the
/// last article of this one.
pub fn continuation(c: &str) -> Option<i64> {
    let c = c.trim();
    if c.is_empty() || c.len() > 19 || !c.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    c.parse::<i64>().ok().filter(|n| *n > 0)
}

/// A time as apps send it: seconds since 1970, or milliseconds (13 digits and
/// more) or microseconds (16 digits and more), as Google Reader did.
pub fn time(ts: &str) -> Option<OffsetDateTime> {
    let ts = ts.trim();
    if ts.is_empty() || ts.len() > 19 || !ts.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let number: i64 = ts.parse().ok()?;
    let nanos = match ts.len() {
        16.. => i128::from(number) * 1_000,
        13..=15 => i128::from(number) * 1_000_000,
        _ => i128::from(number) * 1_000_000_000,
    };
    OffsetDateTime::from_unix_timestamp_nanos(nanos).ok()
}

/// How many are asked for: `n` if it is a number from 1 on, at most `max`;
/// else `default`.
pub fn count(n: Option<&str>, default: usize, max: usize) -> usize {
    match n.and_then(|n| n.trim().parse::<usize>().ok()) {
        Some(n) if n > 0 => n.min(max),
        _ => default.min(max),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_ids_read_in_every_form_and_back() {
        let number = item_number(1234, 5).unwrap();
        let long = long_item_id(number);
        assert_eq!(long, "tag:google.com,2005:reader/item/000004d200000005");
        for written in [
            long.clone(),
            number.to_string(),
            "000004d200000005".to_owned(),
            "tag:google.com,2005:reader/item/4d200000005".to_owned(),
            " 000004D200000005 ".to_owned(),
        ] {
            assert_eq!(item(&written), Some((1234, 5)), "{written}");
        }
        for wrong in [
            "",
            "-5",
            "abc",
            "tag:google.com,2005:reader/item/",
            "tag:google.com,2005:reader/item/00000000000000000005",
            "tag:google.com,2005:reader/item/zz",
            // A feed or a number of 0 is no article.
            "5",
            "tag:google.com,2005:reader/item/4d200000000",
            "99999999999999999999999",
        ] {
            assert_eq!(item(wrong), None, "{wrong}");
        }
        assert_eq!(item_number(-1, 5), None);
    }

    #[test]
    fn streams_are_feeds_labels_and_states() {
        assert_eq!(stream("feed/12"), Some(Stream::Feed(FeedRef::Id(12))));
        assert_eq!(
            stream("feed/https://site.example/feed.xml"),
            Some(Stream::Feed(FeedRef::Url(
                "https://site.example/feed.xml".into()
            )))
        );
        assert_eq!(stream("feed/0"), None);
        assert_eq!(stream("feed/"), None);
        assert_eq!(
            stream("user/-/label/News/Tech"),
            Some(Stream::Label("News/Tech".into()))
        );
        assert_eq!(
            stream("user/1234/state/com.google/starred"),
            Some(Stream::State(State::Starred))
        );
        assert_eq!(
            stream("user/-/state/com.google/broadcast"),
            Some(Stream::State(State::Other))
        );
        for wrong in [
            "",
            "user/-",
            "user/-/label/",
            "user/-/state/x",
            "pop/topic/1",
        ] {
            assert_eq!(stream(wrong), None, "{wrong}");
        }
        for state in [
            State::ReadingList,
            State::Read,
            State::Starred,
            State::KeptUnread,
        ] {
            assert_eq!(stream(state.id().unwrap()), Some(Stream::State(state)));
        }
    }

    #[test]
    fn paths_keep_slashes_in_names() {
        let parts = vec!["News".to_owned(), "A/B".to_owned()];
        let name = path_name(&parts);
        assert_eq!(name, "News/A\u{2215}B");
        assert_eq!(path(&name), Some(parts));
        assert_eq!(
            path(" News / Tech "),
            Some(vec!["News".into(), "Tech".into()])
        );
        assert_eq!(path("a//b"), None);
        assert_eq!(path("a/b/c/d/e/f"), None);
        assert_eq!(path(&"x".repeat(101)), None);
        assert_eq!(path("a/b/c/d/e").map(|p| p.len()), Some(5));
    }

    #[test]
    fn parameters_have_limits() {
        let params = Params::parse("output=json&s=feed%2F1", b"i=1&i=2&T=x").unwrap();
        assert!(params.json());
        assert_eq!(params.one("s"), Some("feed/1"));
        assert_eq!(params.all("i").collect::<Vec<_>>(), ["1", "2"]);
        let many = "i=1&".repeat(MAX_PAIRS + 1);
        assert_eq!(Params::parse(&many, b""), Err(Refusal::TooMany));
        let long = format!("s={}", "x".repeat(MAX_VALUE + 1));
        assert_eq!(Params::parse("", long.as_bytes()), Err(Refusal::TooLong));
    }

    #[test]
    fn times_counts_and_continuations() {
        let second = time("1700000000").unwrap();
        assert_eq!(time("1700000000000"), Some(second));
        assert_eq!(time("1700000000000000"), Some(second));
        assert_eq!(time("-1"), None);
        assert_eq!(time("99999999999999999999"), None);
        assert_eq!(count(Some("50"), 20, 1000), 50);
        assert_eq!(count(Some("5000"), 20, 1000), 1000);
        assert_eq!(count(Some("0"), 20, 1000), 20);
        assert_eq!(count(Some("x"), 20, 1000), 20);
        assert_eq!(count(None, 1000, 250), 250);
        assert_eq!(continuation("42"), Some(42));
        assert_eq!(continuation("0"), None);
        assert_eq!(continuation("abc"), None);
    }
}
