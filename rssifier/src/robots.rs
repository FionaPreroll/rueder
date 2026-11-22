//! `robots.txt`, after RFC 9309: RSSifier asks the site before it fetches a
//! page, under the name [`NAME`], and keeps what the site said for a day.
//!
//! A site without one (an answer of 4xx) allows everything; one whose
//! `robots.txt` cannot be read (5xx, no answer, too large) allows nothing for
//! now, and nothing of that is kept.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, Instant},
};

use hardened_http::{HttpClient, Request};
use url::Url;

/// The name RSSifier goes by in `robots.txt`.
pub const NAME: &str = "rueder-rssifier";
/// The most bytes of a `robots.txt` that are read; RFC 9309 asks for at least
/// 500 KiB.
pub const MAX_BYTES: usize = 512 * 1024;
/// The most rules that are kept, and the longest one.
const MAX_RULES: usize = 2000;
const MAX_PATTERN: usize = 2000;
/// How long what a site said holds; RFC 9309 asks for no more than a day.
const KEEP_FOR: Duration = Duration::from_secs(24 * 60 * 60);
/// How many sites are remembered.
const MAX_SITES: usize = 10_000;

/// The rules of a site for RSSifier.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Rules(Vec<Rule>);

#[derive(Debug, PartialEq, Eq)]
struct Rule {
    allow: bool,
    /// The path as the rule writes it, with `*` for anything and maybe `$`
    /// for the end.
    pattern: String,
}

impl Rules {
    /// The rules of the groups that name `name`, else of those for `*`.
    pub fn parse(text: &str, name: &str) -> Self {
        let mut ours = Vec::new();
        let mut anyone = Vec::new();
        let mut named = false;
        // The agents of the group being read, and whether its rules began.
        let mut agents: Vec<String> = Vec::new();
        let mut in_rules = false;
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or_default();
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let value = value.trim();
            match key.trim().to_ascii_lowercase().as_str() {
                "user-agent" => {
                    if in_rules {
                        agents.clear();
                        in_rules = false;
                    }
                    let token = value.split(['/', ' ']).next().unwrap_or_default();
                    agents.push(token.to_ascii_lowercase());
                    named |= token.eq_ignore_ascii_case(name);
                }
                key @ ("allow" | "disallow") => {
                    in_rules = true;
                    if value.is_empty() || value.len() > MAX_PATTERN {
                        continue;
                    }
                    let rule = || Rule {
                        allow: key == "allow",
                        pattern: encoded(value),
                    };
                    if agents.iter().any(|agent| agent.eq_ignore_ascii_case(name)) {
                        if ours.len() < MAX_RULES {
                            ours.push(rule());
                        }
                    } else if agents.iter().any(|agent| agent == "*") && anyone.len() < MAX_RULES {
                        anyone.push(rule());
                    }
                }
                _ => {}
            }
        }
        Self(if named { ours } else { anyone })
    }

    /// Rules that allow nothing.
    pub fn nothing() -> Self {
        Self(vec![Rule {
            allow: false,
            pattern: "/".into(),
        }])
    }

    /// Whether the page may be fetched: the longest rule that fits decides,
    /// and between two as long the one that allows.
    pub fn allows(&self, page: &Url) -> bool {
        let mut path = page.path().to_owned();
        if let Some(query) = page.query() {
            path.push('?');
            path.push_str(query);
        }
        let mut best: Option<(usize, bool)> = None;
        for rule in &self.0 {
            if fits(&rule.pattern, &path) {
                let length = rule.pattern.len();
                if best.is_none_or(|(longest, allow)| {
                    length > longest || (length == longest && rule.allow && !allow)
                }) {
                    best = Some((length, rule.allow));
                }
            }
        }
        best.is_none_or(|(_, allow)| allow)
    }
}

/// A pattern with what is not ASCII written as the address writes it.
fn encoded(pattern: &str) -> String {
    let mut written = String::with_capacity(pattern.len());
    for c in pattern.chars() {
        if c.is_ascii() {
            written.push(c);
        } else {
            let mut bytes = [0; 4];
            for byte in c.encode_utf8(&mut bytes).bytes() {
                written.push_str(&format!("%{byte:02X}"));
            }
        }
    }
    written
}

/// Whether a pattern fits the start of a path, or all of it with `$`. Each
/// piece between two `*` goes where it is first found, which leaves the most
/// for those after it.
fn fits(pattern: &str, path: &str) -> bool {
    let (pattern, to_the_end) = match pattern.strip_suffix('$') {
        Some(pattern) => (pattern, true),
        None => (pattern, false),
    };
    let mut pieces = pattern.split('*');
    let first = pieces.next().unwrap_or_default();
    let Some(mut rest) = path.strip_prefix(first) else {
        return false;
    };
    let pieces: Vec<&str> = pieces.collect();
    let Some((last, between)) = pieces.split_last() else {
        return !to_the_end || rest.is_empty();
    };
    for piece in between {
        match rest.find(piece) {
            Some(at) => rest = &rest[at + piece.len()..],
            None => return false,
        }
    }
    if to_the_end {
        rest.ends_with(last)
    } else {
        rest.contains(last)
    }
}

/// What sites said, by their origin.
#[derive(Default)]
pub struct Robots {
    sites: Mutex<HashMap<String, (Instant, Arc<Rules>)>>,
}

impl Robots {
    /// The rules of the page's site, asked for if they are not known; why
    /// not, if the site's `robots.txt` cannot be read.
    pub async fn rules(&self, client: &HttpClient, page: &Url) -> Result<Arc<Rules>, String> {
        let origin = page.origin().ascii_serialization();
        if let Some(rules) = self.known(&origin, Instant::now()) {
            return Ok(rules);
        }
        let address = page
            .join("/robots.txt")
            .map_err(|err| err.to_string())?
            .to_string();
        let request = Request {
            accept: Some("text/plain"),
            max_body: Some(MAX_BYTES),
            ..Request::new(&address)
        };
        let response = client.get(request).await.map_err(|err| err.to_string())?;
        let rules = if response.status.is_success() {
            Rules::parse(&String::from_utf8_lossy(&response.body), NAME)
        } else if response.status.is_server_error() {
            return Err(format!("it answers with {}", response.status));
        } else {
            Rules::default()
        };
        let rules = Arc::new(rules);
        self.keep(origin, rules.clone(), Instant::now());
        Ok(rules)
    }

    fn known(&self, origin: &str, now: Instant) -> Option<Arc<Rules>> {
        let sites = self.sites.lock().unwrap_or_else(PoisonError::into_inner);
        let (at, rules) = sites.get(origin)?;
        (now.saturating_duration_since(*at) < KEEP_FOR).then(|| rules.clone())
    }

    fn keep(&self, origin: String, rules: Arc<Rules>, now: Instant) {
        let mut sites = self.sites.lock().unwrap_or_else(PoisonError::into_inner);
        if sites.len() >= MAX_SITES {
            sites.retain(|_, (at, _)| now.saturating_duration_since(*at) < KEEP_FOR);
            if sites.len() >= MAX_SITES {
                sites.clear();
            }
        }
        sites.insert(origin, (now, rules));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allows(text: &str, page: &str) -> bool {
        Rules::parse(text, NAME).allows(&Url::parse(&format!("https://example.org{page}")).unwrap())
    }

    #[test]
    fn the_group_that_names_rssifier_goes_before_the_one_for_everybody() {
        let text = "User-agent: *\nDisallow: /\n\n# for us\nuser-agent: Rueder-RSSifier/1.0\nuser-agent: other\nDisallow: /private\n";
        assert!(allows(text, "/news"));
        assert!(!allows(text, "/private/x"));
        let others = "User-agent: other\nDisallow: /\n";
        assert!(allows(others, "/news"));
        assert!(!allows("User-agent: *\nDisallow: /", "/news"));
    }

    #[test]
    fn the_longest_rule_decides_and_allow_wins_a_tie() {
        let text = "User-agent: *\nDisallow: /news\nAllow: /news/today\nDisallow: /*.pdf$\nAllow: /a\nDisallow: /a\nDisallow: /search?q=*\n";
        assert!(!allows(text, "/news/old"));
        assert!(allows(text, "/news/today/x"));
        assert!(!allows(text, "/files/x.pdf"));
        assert!(allows(text, "/files/x.pdf.html"));
        assert!(allows(text, "/a"));
        assert!(!allows(text, "/search?q=rust"));
        assert!(allows(text, "/search"));
    }

    #[test]
    fn rules_are_matched_as_the_address_writes_them() {
        let text = "User-agent: *\nDisallow: /über\nDisallow: /a*b*c$\n";
        assert!(!allows(text, "/über/x"));
        assert!(!allows(text, "/aXbYc"));
        assert!(allows(text, "/aXbYcZ"));
    }

    #[test]
    fn broken_and_endless_files_are_read_within_limits() {
        assert!(allows("Disallow: /\nno colon\n:\nUser-agent:\n", "/x"));
        assert!(allows("User-agent: *\nDisallow:\n", "/x"));
        let many = format!(
            "User-agent: *\n{}",
            "Disallow: /*a*a*a*a*b$\n".repeat(100_000)
        );
        let rules = Rules::parse(&many, NAME);
        assert_eq!(rules.0.len(), MAX_RULES);
        let started = Instant::now();
        assert!(
            rules
                .allows(&Url::parse(&format!("https://example.org/{}", "a".repeat(1900))).unwrap())
        );
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
