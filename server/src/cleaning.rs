//! Turns the text of an article as its feed sends it into what is stored for
//! reading. This runs once per version on the server, never in the browser.
//!
//! Two steps, in this order:
//!
//! 1. rueder's own rules, on the document tree: links and images get whole
//!    addresses without tracking parameters, share buttons and counting pixels
//!    go, audio and video become enclosures, and what is left empty goes too.
//! 2. `ammonia` with a fixed list of what may stay. It always has the last
//!    word, so whatever the first step does or overlooks, only markup from that
//!    list leaves this module, and no address that is not `http` or `https`.
//!
//! The lists the first step works with are data: `rules/blocked-links.txt` and
//! `rules/tracking-parameters.txt`. They are built into the program, and an
//! operator can put their own in place of them (see [`Rules::from_env`]).

use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    env, fs,
    path::Path,
    rc::Rc,
    sync::LazyLock,
};

use anyhow::{Context, bail};
use html5ever::{
    Attribute, ExpandedName, LocalName, ParseOpts, QualName, ns, parse_fragment,
    serialize::{SerializeOpts, serialize},
    tendril::{StrTendril, TendrilSink},
    tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink},
};
use markup5ever_rcdom::{Handle, Node, NodeData, RcDom, SerializableHandle};
use url::{ParseError, Position, Url};

/// How deep a text may nest its tags. Real articles stay far below it. The
/// parser looks through all open tags again and again, so its work grows with
/// the square of the depth: a megabyte of nothing but opening tags would keep
/// it busy for minutes.
pub const MAX_DEPTH: u32 = 256;
/// How many tags, pieces of text and comments a text may come to. Markup that
/// leaves formatting open makes the parser repeat it in every paragraph that
/// follows, so a megabyte of text can come to a hundred times as much tree.
pub const MAX_NODES: usize = 200_000;
/// The text reaches the parser in pieces of this many bytes, and the limits are
/// looked at after each.
const PIECE: usize = 1024;

/// What decides about links, images and addresses.
#[derive(Debug, Clone, Default)]
pub struct Rules {
    blocked: Vec<Blocked>,
    tracking: Vec<Tracking>,
}

/// A domain whose links and images are left out.
#[derive(Debug, Clone)]
struct Blocked {
    domain: String,
    /// Only addresses that have this behind the domain.
    containing: Option<String>,
}

/// A parameter that is taken out of addresses.
#[derive(Debug, Clone)]
enum Tracking {
    Name(String),
    NamePrefix(String),
    Value(String, String),
    ValueContaining(String, String),
}

impl Rules {
    /// The lists that come with rueder.
    pub fn built_in() -> anyhow::Result<Self> {
        Self::parse(
            include_str!("../rules/blocked-links.txt"),
            include_str!("../rules/tracking-parameters.txt"),
        )
    }

    /// The lists of the directory `CLEANING_RULES_DIR` names, else the ones
    /// that come with rueder.
    pub fn from_env() -> anyhow::Result<Self> {
        match env::var("CLEANING_RULES_DIR") {
            Ok(dir) if !dir.is_empty() => Self::load(Path::new(&dir)),
            _ => Self::built_in(),
        }
    }

    /// The lists `blocked-links.txt` and `tracking-parameters.txt` of a directory.
    pub fn load(dir: &Path) -> anyhow::Result<Self> {
        let read = |name: &str| {
            let path = dir.join(name);
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))
        };
        Self::parse(
            &read("blocked-links.txt")?,
            &read("tracking-parameters.txt")?,
        )
    }

    /// Reads the two lists. Empty lines and lines starting with `#` say nothing.
    pub fn parse(blocked_links: &str, tracking_parameters: &str) -> anyhow::Result<Self> {
        let mut rules = Self::default();
        for (number, line) in lines(blocked_links) {
            let mut parts = line.split_whitespace();
            let (Some(domain), containing, None) = (parts.next(), parts.next(), parts.next())
            else {
                bail!("blocked links, line {number}: a domain and at most one piece of text");
            };
            let domain = domain.trim_matches('.').to_ascii_lowercase();
            let is_domain = domain
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.');
            if domain.is_empty() || !is_domain {
                bail!("blocked links, line {number}: {domain} is not a domain");
            }
            rules.blocked.push(Blocked {
                domain,
                containing: containing.map(str::to_ascii_lowercase),
            });
        }
        for (number, line) in lines(tracking_parameters) {
            let rule = match line.split_once('=') {
                None => match line.strip_suffix('*') {
                    Some(prefix) => Tracking::NamePrefix(prefix.to_owned()),
                    None => Tracking::Name(line.to_owned()),
                },
                Some((name, value)) => {
                    let inner = value
                        .strip_prefix('*')
                        .and_then(|value| value.strip_suffix('*'));
                    match inner {
                        Some(inner) => Tracking::ValueContaining(name.to_owned(), inner.to_owned()),
                        None => Tracking::Value(name.to_owned(), value.to_owned()),
                    }
                }
            };
            let (Tracking::Name(name)
            | Tracking::NamePrefix(name)
            | Tracking::Value(name, _)
            | Tracking::ValueContaining(name, _)) = &rule;
            let odd =
                |text: &str| text.contains(|c: char| c.is_whitespace() || "*&=#?".contains(c));
            let value_is_odd = match &rule {
                Tracking::Value(_, value) | Tracking::ValueContaining(_, value) => {
                    value.is_empty() || odd(value)
                }
                _ => false,
            };
            if name.is_empty() || odd(name) || value_is_odd {
                bail!("tracking parameters, line {number}: cannot read {line}");
            }
            rules.tracking.push(rule);
        }
        Ok(rules)
    }

    /// Whether links and images of this address are left out.
    pub fn blocks(&self, url: &Url) -> bool {
        let Some(host) = url.host_str() else {
            return false;
        };
        let host = host.trim_end_matches('.').to_ascii_lowercase();
        let behind = url[Position::BeforePath..].to_ascii_lowercase();
        self.blocked.iter().any(|blocked| {
            let in_domain = host
                .strip_suffix(&blocked.domain)
                .is_some_and(|sub| sub.is_empty() || sub.ends_with('.'));
            in_domain
                && blocked
                    .containing
                    .as_ref()
                    .is_none_or(|text| behind.contains(text))
        })
    }

    /// Takes the tracking parameters out of an address. The rest of it stays
    /// as it is written.
    pub fn strip_tracking(&self, url: &mut Url) {
        let Some(query) = url.query() else { return };
        let kept: Vec<&str> = query
            .split('&')
            .filter(|parameter| {
                let (name, value) = match parameter.split_once('=') {
                    Some((name, value)) => (name, Some(value)),
                    None => (*parameter, None),
                };
                // Some sites end their links in "?" or "?&" for no reason.
                !name.is_empty() && !self.tracks(name, value)
            })
            .collect();
        let query = kept.join("&");
        url.set_query((!query.is_empty()).then_some(&query));
    }

    fn tracks(&self, name: &str, value: Option<&str>) -> bool {
        self.tracking.iter().any(|rule| match rule {
            Tracking::Name(tracked) => name == tracked,
            Tracking::NamePrefix(prefix) => name.starts_with(prefix.as_str()),
            Tracking::Value(tracked, exactly) => name == tracked && value == Some(exactly),
            Tracking::ValueContaining(tracked, part) => {
                name == tracked && value.is_some_and(|value| value.contains(part.as_str()))
            }
        })
    }

    /// A web address without its tracking parameters. Anything that is not a
    /// web address comes back as it is.
    pub fn clean_url(&self, url: &str) -> String {
        match Url::parse(url.trim()) {
            Ok(mut parsed) if is_web(&parsed) && parsed.query().is_some() => {
                self.strip_tracking(&mut parsed);
                parsed.into()
            }
            _ => url.to_owned(),
        }
    }
}

/// The lines of a list that say something, with their numbers.
fn lines(list: &str) -> impl Iterator<Item = (usize, &str)> {
    list.lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line.trim()))
        .filter(|(_, line)| !line.is_empty() && !line.starts_with('#'))
}

fn is_web(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https") && url.host_str().is_some()
}

/// Where the addresses in an article's text are relative to.
#[derive(Debug, Clone, Copy, Default)]
pub struct Base<'a> {
    /// The article's own page. Links like `#notes` point into it, and other
    /// relative addresses are taken from it first: a text that says
    /// `images/1.png` was written for that page.
    pub article: Option<&'a str>,
    /// The feed, for articles without a page of their own.
    pub feed: Option<&'a str>,
}

struct Bases {
    article: Option<Url>,
    feed: Option<Url>,
}

impl Bases {
    fn new(base: Base<'_>) -> Self {
        let web = |url: Option<&str>| {
            url.and_then(|url| Url::parse(url.trim()).ok())
                .filter(is_web)
        };
        Self {
            article: web(base.article),
            feed: web(base.feed),
        }
    }

    /// The whole web address an attribute means, if it means one.
    fn resolve(&self, address: &str) -> Option<Url> {
        let address = address.trim();
        if address.is_empty() {
            return None;
        }
        let url = match Url::parse(address) {
            Ok(url) => url,
            Err(ParseError::RelativeUrlWithoutBase) => {
                let base = if address.starts_with('#') {
                    self.article.as_ref()
                } else {
                    self.article.as_ref().or(self.feed.as_ref())
                };
                base?.join(address).ok()?
            }
            Err(_) => return None,
        };
        is_web(&url).then_some(url)
    }
}

/// An audio or video file that stood in the text of an article.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enclosure {
    pub url: String,
    /// Its media type, such as `video/mp4`, if the text says.
    pub kind: Option<String>,
}

/// What is left of an article's text.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Cleaned {
    /// For reading. Only markup from the fixed list, and every address in it is
    /// a whole `http` or `https` address.
    pub html: String,
    /// What `audio` and `video` in the text pointed to. The players are gone
    /// from the text: media loads when the reader asks for it, not before.
    pub enclosures: Vec<Enclosure>,
}

/// Cleans the text of an article.
///
/// What comes out is written down as HTML and read again by the browser, and
/// in rare cases the rules of HTML make it read differently from how it was
/// meant: a table cannot stand inside a paragraph, say, so the paragraph ends
/// before it. So the result is cleaned again until it stays the same. That
/// takes one more round for nearly every text; a text that still changes after
/// [`ROUNDS`] is not worth the trust and is kept as words without markup.
pub fn clean(html: &str, base: Base<'_>, rules: &Rules) -> Cleaned {
    let bases = Bases::new(base);
    let (mut html, enclosures) = round(html, &bases, rules);
    for _ in 1..ROUNDS {
        let (again, _) = round(&html, &bases, rules);
        if again == html {
            return Cleaned { html, enclosures };
        }
        html = again;
    }
    Cleaned {
        html: allowed_only(&without_markup(&html)),
        enclosures,
    }
}

/// How often a text is cleaned at most until it stays the same.
const ROUNDS: usize = 4;

/// Both steps, once.
fn round(html: &str, bases: &Bases, rules: &Rules) -> (String, Vec<Enclosure>) {
    let (html, enclosures) = apply_rules(html, bases, rules);
    (allowed_only(&html), enclosures)
}

/// The second step. What it is given was written out by the first step or has
/// no tags at all, so the tree it builds for itself stays as small as the one
/// that was counted.
fn allowed_only(html: &str) -> String {
    #[expect(clippy::disallowed_methods)]
    let document = ALLOWED.clean(html);
    document.to_string()
}

/// The tags that may stay.
const TAGS: [&str; 64] = [
    "a",
    "abbr",
    "b",
    "bdi",
    "bdo",
    "blockquote",
    "br",
    "caption",
    "cite",
    "code",
    "col",
    "colgroup",
    "dd",
    "del",
    "details",
    "dfn",
    "div",
    "dl",
    "dt",
    "em",
    "figcaption",
    "figure",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "hr",
    "i",
    "img",
    "ins",
    "kbd",
    "li",
    "mark",
    "ol",
    "p",
    "pre",
    "q",
    "rp",
    "rt",
    "ruby",
    "s",
    "samp",
    "small",
    "span",
    "strike",
    "strong",
    "sub",
    "summary",
    "sup",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "time",
    "tr",
    "tt",
    "u",
    "ul",
    "var",
    "wbr",
];

/// What may stay. Tags that are not on the list go and leave their content
/// behind; `script` and `style` go with their content.
static ALLOWED: LazyLock<ammonia::Builder<'static>> = LazyLock::new(|| {
    let mut allowed = ammonia::Builder::empty();
    allowed
        .tags(HashSet::from(TAGS))
        .clean_content_tags(HashSet::from(["script", "style"]))
        // No `id`, `name`, `class` or `style`: nothing in an article can pose as
        // a part of rueder's own page or change how it looks.
        .generic_attributes(HashSet::from(["dir", "lang", "title"]))
        .tag_attributes(HashMap::from([
            ("a", HashSet::from(["href"])),
            // No `srcset`: it would tell the site the size of the reader's screen.
            ("img", HashSet::from(["src", "alt", "width", "height"])),
            ("ol", HashSet::from(["start"])),
            ("td", HashSet::from(["colspan", "rowspan"])),
            ("th", HashSet::from(["colspan", "rowspan"])),
            ("time", HashSet::from(["datetime"])),
        ]))
        .url_schemes(HashSet::from(["http", "https"]))
        // An address without a host would point into rueder itself.
        .url_relative(ammonia::UrlRelative::Deny)
        // A link opens in a new tab and tells the site nothing about where the
        // reader came from.
        .link_rel(Some("nofollow noopener noreferrer"))
        .set_tag_attribute_value("a", "target", "_blank")
        .set_tag_attribute_value("img", "loading", "lazy")
        .strip_comments(true);
    allowed
});

/// Builds the tree for the parser and keeps count of what it is asked to build.
struct Counting {
    dom: RcDom,
    nodes: Cell<usize>,
    deepest: Cell<u32>,
    /// How deep every element lies.
    depths: RefCell<HashMap<*const Node, u32>>,
}

impl Counting {
    fn depth(&self, node: &Handle) -> u32 {
        let depths = self.depths.borrow();
        depths.get(&Rc::as_ptr(node)).copied().unwrap_or(0)
    }

    /// Notes a new node that lies `depth` deep.
    fn count(&self, child: &NodeOrText<Handle>, depth: u32) {
        self.nodes.set(self.nodes.get() + 1);
        if let NodeOrText::AppendNode(node) = child {
            self.depths.borrow_mut().insert(Rc::as_ptr(node), depth);
            self.deepest.set(self.deepest.get().max(depth));
        }
    }

    fn within_limits(&self) -> bool {
        self.nodes.get() <= MAX_NODES && self.deepest.get() <= MAX_DEPTH
    }
}

impl TreeSink for Counting {
    type Handle = Handle;
    /// The tree, unless it went beyond the limits.
    type Output = Option<Handle>;
    type ElemName<'a> = ExpandedName<'a>;

    fn finish(self) -> Option<Handle> {
        self.within_limits().then_some(self.dom.document)
    }

    fn append(&self, parent: &Handle, child: NodeOrText<Handle>) {
        self.count(&child, self.depth(parent) + 1);
        self.dom.append(parent, child);
    }

    fn append_before_sibling(&self, sibling: &Handle, new_node: NodeOrText<Handle>) {
        self.count(&new_node, self.depth(sibling));
        self.dom.append_before_sibling(sibling, new_node);
    }

    fn append_based_on_parent_node(
        &self,
        element: &Handle,
        prev_element: &Handle,
        child: NodeOrText<Handle>,
    ) {
        // Next to `element` or into `prev_element`; the deeper of the two will do.
        let depth = self.depth(element).max(self.depth(prev_element) + 1);
        self.count(&child, depth);
        self.dom
            .append_based_on_parent_node(element, prev_element, child);
    }

    fn get_template_contents(&self, target: &Handle) -> Handle {
        let contents = self.dom.get_template_contents(target);
        let depth = self.depth(target);
        self.depths
            .borrow_mut()
            .insert(Rc::as_ptr(&contents), depth);
        contents
    }

    // The rest is passed on as it is.

    fn parse_error(&self, msg: Cow<'static, str>) {
        self.dom.parse_error(msg);
    }

    fn get_document(&self) -> Handle {
        self.dom.get_document()
    }

    fn elem_name<'a>(&'a self, target: &'a Handle) -> ExpandedName<'a> {
        self.dom.elem_name(target)
    }

    fn create_element(&self, name: QualName, attrs: Vec<Attribute>, flags: ElementFlags) -> Handle {
        self.dom.create_element(name, attrs, flags)
    }

    fn create_comment(&self, text: StrTendril) -> Handle {
        self.dom.create_comment(text)
    }

    fn create_pi(&self, target: StrTendril, data: StrTendril) -> Handle {
        self.dom.create_pi(target, data)
    }

    fn append_doctype_to_document(
        &self,
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    ) {
        self.dom
            .append_doctype_to_document(name, public_id, system_id);
    }

    fn mark_script_already_started(&self, node: &Handle) {
        self.dom.mark_script_already_started(node);
    }

    fn pop(&self, node: &Handle) {
        self.dom.pop(node);
    }

    fn same_node(&self, x: &Handle, y: &Handle) -> bool {
        self.dom.same_node(x, y)
    }

    fn set_quirks_mode(&self, mode: QuirksMode) {
        self.dom.set_quirks_mode(mode);
    }

    fn add_attrs_if_missing(&self, target: &Handle, attrs: Vec<Attribute>) {
        self.dom.add_attrs_if_missing(target, attrs);
    }

    fn associate_with_form(
        &self,
        target: &Handle,
        form: &Handle,
        nodes: (&Handle, Option<&Handle>),
    ) {
        self.dom.associate_with_form(target, form, nodes);
    }

    fn remove_from_parent(&self, target: &Handle) {
        self.dom.remove_from_parent(target);
    }

    fn reparent_children(&self, node: &Handle, new_parent: &Handle) {
        self.dom.reparent_children(node, new_parent);
    }

    fn is_mathml_annotation_xml_integration_point(&self, handle: &Handle) -> bool {
        self.dom.is_mathml_annotation_xml_integration_point(handle)
    }

    fn set_current_line(&self, line_number: u64) {
        self.dom.set_current_line(line_number);
    }

    fn allow_declarative_shadow_roots(&self, intended_parent: &Handle) -> bool {
        self.dom.allow_declarative_shadow_roots(intended_parent)
    }

    fn attach_declarative_shadow(
        &self,
        location: &Handle,
        template: &Handle,
        attrs: &[Attribute],
    ) -> bool {
        self.dom
            .attach_declarative_shadow(location, template, attrs)
    }

    fn maybe_clone_an_option_into_selectedcontent(&self, option: &Handle) {
        self.dom.maybe_clone_an_option_into_selectedcontent(option);
    }
}

/// Reads a piece of HTML into a tree and returns the element that holds it, or
/// `None` if the text nests too deep or comes to too much (see [`MAX_DEPTH`]
/// and [`MAX_NODES`]). Then the parser stops where it is.
fn read_within_limits(html: &str) -> Option<Handle> {
    let sink = Counting {
        dom: RcDom::default(),
        nodes: Cell::new(0),
        deepest: Cell::new(0),
        depths: RefCell::default(),
    };
    let context = QualName::new(None, ns!(html), LocalName::from("div"));
    // The one place that reads HTML from outside; `clippy.toml` forbids it elsewhere.
    #[expect(clippy::disallowed_methods)]
    let mut parser = parse_fragment(sink, ParseOpts::default(), context, Vec::new(), false);
    let mut rest = html;
    while !rest.is_empty() {
        let (piece, later) = rest.split_at(rest.floor_char_boundary(PIECE));
        parser.process(piece.into());
        if !parser.tokenizer.sink.sink.within_limits() {
            return None;
        }
        rest = later;
    }
    // What is still open is closed at the end, which can add to the tree.
    let document = parser.finish()?;
    // The tree library empties what is under a node when the node goes, wanted
    // elsewhere or not. So the element leaves the document before that goes.
    let held = std::mem::take(&mut *document.children.borrow_mut());
    held.into_iter().next()
}

/// Reads a piece of HTML into a tree and returns the element that holds it. A
/// text beyond the limits is taken for what it is, somebody trying something,
/// and read without its markup.
fn read(html: &str) -> Handle {
    let tree = read_within_limits(html).or_else(|| read_within_limits(&without_markup(html)));
    // Text without tags makes a flat tree, so this never comes to the empty one.
    tree.unwrap_or_else(|| Node::new(NodeData::Document))
}

/// The text with everything that looks like a tag cut out, without reading it
/// as HTML.
fn without_markup(html: &str) -> String {
    let mut plain = String::with_capacity(html.len());
    let mut rest = html;
    while let Some((before, after)) = rest.split_once('<') {
        plain.push_str(before);
        let opens_tag = after.starts_with(|c: char| c.is_ascii_alphabetic() || "/!?".contains(c));
        rest = match (opens_tag, after.split_once('>')) {
            (true, Some((_, behind))) => {
                plain.push(' ');
                behind
            }
            // A tag that never ends takes the rest with it.
            (true, None) => "",
            (false, _) => {
                plain.push_str("&lt;");
                after
            }
        };
    }
    plain.push_str(rest);
    plain
}

/// The words of a piece of HTML, without its markup, with single blanks.
pub fn text(html: &str) -> String {
    let mut words = String::new();
    // Stays until the end: a node that goes takes what is under it along.
    let tree = read(html);
    // A node to read, or `None` for the end of a line.
    let mut waiting = vec![Some(tree.clone())];
    while let Some(next) = waiting.pop() {
        let Some(node) = next else {
            words.push(' ');
            continue;
        };
        match &node.data {
            NodeData::Text { contents } => words.push_str(&contents.borrow()),
            NodeData::Element { name, .. } => {
                let name = &*name.local;
                if matches!(name, "script" | "style" | "template") {
                    continue;
                }
                // Where a paragraph or a line ends, words must not run together.
                if !INLINE.contains(&name) {
                    words.push(' ');
                    waiting.push(None);
                }
                waiting.extend(node.children.borrow().iter().rev().cloned().map(Some));
            }
            _ => {}
        }
    }
    words.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Tags that stand inside a line of text. All others start a new one.
const INLINE: [&str; 28] = [
    "a", "abbr", "b", "bdi", "bdo", "cite", "code", "del", "dfn", "em", "font", "i", "ins", "kbd",
    "mark", "q", "s", "samp", "small", "span", "strike", "strong", "sub", "sup", "time", "tt", "u",
    "var",
];

/// What becomes of a node.
#[derive(Clone, Copy)]
enum Fate {
    /// It goes, with all that is in it.
    Drop,
    /// It goes, and what is in it takes its place.
    Unwrap,
}

/// A node and what its place in the tree says about it.
struct Visit {
    node: Handle,
    /// Inside `pre`, where white space is meant as written.
    preformatted: bool,
    /// Inside `audio` or `video`.
    in_media: bool,
    /// The text as a whole, which holds everything else.
    root: bool,
}

/// rueder's own rules: reads the text into a tree, changes the tree and writes
/// it out again. Returns the text and the enclosures found in it.
///
/// Texts nest as deep as their senders like, so the tree is walked with a list
/// of nodes, never by a function calling itself.
fn apply_rules(html: &str, bases: &Bases, rules: &Rules) -> (String, Vec<Enclosure>) {
    let root = read(html);

    // Every node, parents before their children.
    let mut order = Vec::new();
    let mut waiting = vec![Visit {
        node: root.clone(),
        preformatted: false,
        in_media: false,
        root: true,
    }];
    while let Some(visit) = waiting.pop() {
        let name = element_name(&visit.node);
        let preformatted = visit.preformatted || matches!(name, Some("pre" | "textarea"));
        let in_media = visit.in_media || matches!(name, Some("audio" | "video"));
        for child in visit.node.children.borrow().iter().rev() {
            waiting.push(Visit {
                node: child.clone(),
                preformatted,
                in_media,
                root: false,
            });
        }
        order.push(visit);
    }

    // Children before their parents: when a node is looked at, what is in it
    // has its final shape.
    let mut fates: HashMap<*const Node, Fate> = HashMap::new();
    let mut media: Vec<(usize, Enclosure)> = Vec::new();
    for (place, visit) in order.iter().enumerate().rev() {
        let fate = match &visit.node.data {
            NodeData::Text { .. } => None,
            NodeData::Element { name, attrs, .. } => {
                let as_written =
                    visit.preformatted || matches!(&*name.local, "pre" | "textarea") || visit.root;
                settle_children(&visit.node, &mut fates, as_written);
                let mut attrs = attrs.borrow_mut();
                match &*name.local {
                    "script" | "style" => Some(Fate::Drop),
                    // What is not HTML (drawings, formulas) leaves its words.
                    _ if name.ns != ns!(html) => Some(Fate::Unwrap),
                    "a" => link(&mut attrs, bases, rules),
                    "img" => image(&mut attrs, bases, rules),
                    "audio" | "video" => {
                        media.extend(enclosure(&attrs, bases, rules).map(|found| (place, found)));
                        Some(Fate::Drop)
                    }
                    "source" if visit.in_media => {
                        media.extend(enclosure(&attrs, bases, rules).map(|found| (place, found)));
                        None
                    }
                    "span" | "p" if is_blank(&visit.node) => Some(Fate::Drop),
                    // The second step would do this too. Doing it here lets the
                    // rules above see what the text comes to in the end: a
                    // paragraph that holds nothing but an unknown tag is empty.
                    name if !TAGS.contains(&name) => Some(Fate::Unwrap),
                    _ => None,
                }
            }
            // Comments and the like say nothing to a reader.
            _ => Some(Fate::Drop),
        };
        if let Some(fate) = fate {
            fates.insert(Rc::as_ptr(&visit.node), fate);
        }
    }

    media.sort_by_key(|(place, _)| *place);
    let mut seen = HashSet::new();
    let enclosures = media
        .into_iter()
        .map(|(_, enclosure)| enclosure)
        .filter(|enclosure| seen.insert(enclosure.url.clone()))
        .collect();

    (written(root), enclosures)
}

/// What is in a tree, written down as HTML.
fn written(root: Handle) -> String {
    let mut out = Vec::new();
    let root: SerializableHandle = root.into();
    if serialize(&mut out, &root, SerializeOpts::default()).is_err() {
        return String::new();
    }
    String::from_utf8(out).unwrap_or_default()
}

/// A cleaned text with other addresses for its images: `address` is asked for
/// each image's own and says what to put in its place. An image it has no
/// address for goes.
///
/// This is for a text as [`clean`] left it and puts nothing else right.
pub fn with_images(html: &str, address: impl Fn(&str) -> Option<String>) -> String {
    let root = read(html);
    let mut waiting = vec![root.clone()];
    while let Some(node) = waiting.pop() {
        let mut children = node.children.borrow_mut();
        children.retain(|child| match &child.data {
            NodeData::Element { name, attrs, .. } if &*name.local == "img" => {
                let mut attrs = attrs.borrow_mut();
                let new = attribute(&attrs, "src").and_then(|own| address(&own));
                match new {
                    Some(new) => {
                        set_attribute(&mut attrs, "src", &new);
                        true
                    }
                    None => false,
                }
            }
            _ => true,
        });
        waiting.extend(children.iter().cloned());
    }
    written(root)
}

/// The most image addresses [`images_in`] gives of one text.
pub const MAX_IMAGES: usize = 1000;

/// The addresses of the images in a cleaned text, in its order, each once and
/// at most [`MAX_IMAGES`]: for taking a blocked feed's images out of the cache.
///
/// This is for a text as [`clean`] left it.
pub fn images_in(html: &str) -> Vec<String> {
    // The root stays held, as in `first_image`.
    let root = read(html);
    let mut found: Vec<String> = Vec::new();
    let mut waiting = vec![root.clone()];
    while let Some(node) = waiting.pop() {
        if found.len() >= MAX_IMAGES {
            break;
        }
        if let NodeData::Element { name, attrs, .. } = &node.data
            && &*name.local == "img"
            && let Some(src) = attribute(&attrs.borrow(), "src")
            && !found.contains(&src)
        {
            found.push(src);
        }
        waiting.extend(node.children.borrow().iter().rev().cloned());
    }
    found
}

/// Below this many pixels wide or high, as a text says, an image is an icon or
/// a smiley, not a picture to show an article by.
const SMALLEST_PICTURE: u32 = 100;

/// The first picture in a cleaned text, to show the article by where the feed
/// names none: the address of the first image that does not say it is small.
///
/// This is for a text as [`clean`] left it.
pub fn first_image(html: &str) -> Option<String> {
    let small = |attrs: &[Attribute], name: &str| {
        attribute(attrs, name)
            .and_then(|value| value.trim().parse::<u32>().ok())
            .is_some_and(|pixels| pixels < SMALLEST_PICTURE)
    };
    // The root stays held: the tree library empties all under a node that goes,
    // held elsewhere or not.
    let root = read(html);
    // In the order of the text: the children of a node before its next sibling.
    let mut waiting = vec![root.clone()];
    while let Some(node) = waiting.pop() {
        if let NodeData::Element { name, attrs, .. } = &node.data
            && &*name.local == "img"
        {
            let attrs = attrs.borrow();
            if !small(&attrs, "width")
                && !small(&attrs, "height")
                && let Some(src) = attribute(&attrs, "src")
            {
                return Some(src);
            }
        }
        waiting.extend(node.children.borrow().iter().rev().cloned());
    }
    None
}

/// A feed a web page says it has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedLink {
    pub url: Url,
    /// The name the page gives it; often empty.
    pub title: String,
}

/// How many feeds are taken from one page.
pub const MAX_FEED_LINKS: usize = 20;
/// The longest name of a feed taken from a page, in characters.
const MAX_FEED_TITLE: usize = 200;
/// The longest address a page may give for itself, in bytes.
const MAX_PAGE_ADDRESS: usize = 2000;

/// What the search for feeds learns from a web page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeedPage {
    /// The feeds it points to, see [`feed_links`].
    pub links: Vec<FeedLink>,
    /// The address the page gives for itself in `<meta property="og:url">`,
    /// if it is one of the web.
    pub address: Option<Url>,
    /// The name it gives itself in `<meta property="og:title">`; often empty.
    pub title: String,
}

/// The feeds a web page points to with `<link rel="alternate">`, in the order
/// of the page and each address once. `base` is where the page came from. A
/// page beyond the limits of the parser has none.
pub fn feed_links(html: &str, base: &Url) -> Vec<FeedLink> {
    feed_page(html, base).links
}

/// One line of text, cut to what fits a name.
fn one_line(text: &str) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    text.chars().take(MAX_FEED_TITLE).collect()
}

/// What a web page says about its feeds and itself: the feeds it points to
/// (see [`feed_links`]), and the address and the name it gives itself, which
/// is how a site like YouTube says which channel a page is of.
pub fn feed_page(html: &str, base: &Url) -> FeedPage {
    const TYPES: [&str; 4] = [
        "application/rss+xml",
        "application/atom+xml",
        "application/feed+json",
        "application/rdf+xml",
    ];
    let Some(root) = read_within_limits(html) else {
        return FeedPage::default();
    };
    let mut page = FeedPage::default();
    let (mut seen_address, mut seen_title) = (false, false);
    let mut waiting = vec![root];
    while let Some(node) = waiting.pop() {
        if let NodeData::Element { name, attrs, .. } = &node.data {
            let attrs = attrs.borrow();
            match &*name.local {
                "link" if page.links.len() < MAX_FEED_LINKS => {
                    let is = |name: &str, wanted: &[&str]| {
                        attribute(&attrs, name).is_some_and(|value| {
                            let value = value.to_ascii_lowercase();
                            let mut words = value.split([' ', '\t', '\n', ';']);
                            words.any(|word| wanted.contains(&word))
                        })
                    };
                    let url = attribute(&attrs, "href")
                        .filter(|_| is("rel", &["alternate"]) && is("type", &TYPES))
                        .and_then(|href| base.join(href.trim()).ok())
                        .filter(is_web);
                    if let Some(url) = url
                        && page.links.iter().all(|link| link.url != url)
                    {
                        let title = attribute(&attrs, "title").unwrap_or_default();
                        page.links.push(FeedLink {
                            url,
                            title: one_line(&title),
                        });
                    }
                }
                // Only the first of each counts, as for the sites that read them.
                "meta" => {
                    let property = attribute(&attrs, "property").map(|p| p.to_ascii_lowercase());
                    let content = attribute(&attrs, "content");
                    match (property.as_deref(), content) {
                        (Some("og:url"), Some(content)) if !seen_address => {
                            seen_address = true;
                            page.address = (content.len() <= MAX_PAGE_ADDRESS)
                                .then(|| base.join(content.trim()).ok())
                                .flatten()
                                .filter(is_web);
                        }
                        (Some("og:title"), Some(content)) if !seen_title => {
                            seen_title = true;
                            page.title = one_line(&content);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        // The first child comes off the pile first: the order of the page.
        waiting.extend(node.children.borrow().iter().rev().cloned());
    }
    page
}

fn element_name(node: &Handle) -> Option<&str> {
    match &node.data {
        NodeData::Element { name, .. } => Some(&*name.local),
        _ => None,
    }
}

/// Gives an element the children it keeps: without those that go, with the
/// content of those that are unwrapped, and with one line break where several
/// followed each other. Pieces of text that now stand side by side become one,
/// as they would be on reading the text again, and unless the text is meant
/// `as_written`, every run of white space in it becomes one blank.
fn settle_children(node: &Handle, fates: &mut HashMap<*const Node, Fate>, as_written: bool) {
    let before = std::mem::take(&mut *node.children.borrow_mut());
    let mut after: Vec<Handle> = Vec::with_capacity(before.len());
    let add = |child: Handle, after: &mut Vec<Handle>| {
        if element_name(&child) == Some("br") {
            let last_said = after.iter().rev().find(|earlier| !is_blank_text(earlier));
            if last_said.is_some_and(|earlier| element_name(earlier) == Some("br")) {
                return;
            }
        }
        if let (NodeData::Text { contents }, Some(last)) = (&child.data, after.last())
            && let NodeData::Text { contents: earlier } = &last.data
        {
            earlier.borrow_mut().push_tendril(&contents.borrow());
            return;
        }
        after.push(child);
    };
    for child in before {
        match fates.remove(&Rc::as_ptr(&child)) {
            Some(Fate::Drop) => {}
            Some(Fate::Unwrap) => {
                for inner in std::mem::take(&mut *child.children.borrow_mut()) {
                    add(inner, &mut after);
                }
            }
            None => add(child, &mut after),
        }
    }
    if !as_written {
        for child in &after {
            if let NodeData::Text { contents } = &child.data {
                let single = single_blanks(&contents.borrow());
                if let Some(single) = single {
                    *contents.borrow_mut() = single.into();
                }
            }
        }
    }
    *node.children.borrow_mut() = after;
}

fn is_blank_text(node: &Handle) -> bool {
    match &node.data {
        NodeData::Text { contents } => contents.borrow().trim().is_empty(),
        _ => false,
    }
}

/// Whether an element has nothing in it but white space.
fn is_blank(node: &Handle) -> bool {
    node.children.borrow().iter().all(is_blank_text)
}

/// The text with every run of white space as one blank, or `None` if it has
/// only single blanks already. A browser shows both the same.
fn single_blanks(text: &str) -> Option<String> {
    let mut single = String::with_capacity(text.len());
    let mut in_run = false;
    for c in text.chars() {
        if c.is_ascii_whitespace() {
            if !in_run {
                single.push(' ');
            }
            in_run = true;
        } else {
            single.push(c);
            in_run = false;
        }
    }
    (single != text).then_some(single)
}

fn attribute(attrs: &[Attribute], name: &str) -> Option<String> {
    attrs
        .iter()
        .find(|attr| &*attr.name.local == name)
        .map(|attr| attr.value.to_string())
}

fn set_attribute(attrs: &mut Vec<Attribute>, name: &str, value: &str) {
    match attrs.iter_mut().find(|attr| &*attr.name.local == name) {
        Some(attr) => attr.value = value.into(),
        None => attrs.push(Attribute {
            name: QualName::new(None, ns!(), LocalName::from(name)),
            value: value.into(),
        }),
    }
}

/// A link keeps its whole address without tracking parameters. A share button
/// goes with its label; a link that leads nowhere on the web leaves its words.
fn link(attrs: &mut Vec<Attribute>, bases: &Bases, rules: &Rules) -> Option<Fate> {
    let address = attribute(attrs, "href").and_then(|href| bases.resolve(&href));
    let Some(mut url) = address else {
        return Some(Fate::Unwrap);
    };
    if rules.blocks(&url) {
        return Some(Fate::Drop);
    }
    rules.strip_tracking(&mut url);
    set_attribute(attrs, "href", url.as_str());
    None
}

/// An image keeps its whole address without tracking parameters. Images of
/// blocked sites and counting pixels go.
fn image(attrs: &mut Vec<Attribute>, bases: &Bases, rules: &Rules) -> Option<Fate> {
    let address = attribute(attrs, "src").and_then(|src| bases.resolve(&src));
    let Some(mut url) = address else {
        return Some(Fate::Drop);
    };
    // Some sites are kind enough to say how small their tracking pixel is.
    let tiny =
        |name: &str| attribute(attrs, name).is_some_and(|size| matches!(size.trim(), "0" | "1"));
    if rules.blocks(&url) || (tiny("width") && tiny("height")) {
        return Some(Fate::Drop);
    }
    rules.strip_tracking(&mut url);
    set_attribute(attrs, "src", url.as_str());
    // Where only the alternative text says what the image is about (as on
    // xkcd), show it on hovering too.
    let said = |name: &str| attribute(attrs, name).filter(|text| !text.trim().is_empty());
    if let (None, Some(alt)) = (said("title"), said("alt")) {
        set_attribute(attrs, "title", &alt);
    }
    None
}

/// What an `audio`, `video` or `source` tag points to.
fn enclosure(attrs: &[Attribute], bases: &Bases, rules: &Rules) -> Option<Enclosure> {
    let mut url = bases.resolve(&attribute(attrs, "src")?)?;
    if rules.blocks(&url) {
        return None;
    }
    rules.strip_tracking(&mut url);
    let kind = attribute(attrs, "type")
        .and_then(|kind| {
            let kind = kind.split(';').next()?.trim().to_ascii_lowercase();
            Some(kind)
        })
        .filter(|kind| !kind.is_empty() && kind.len() <= 100);
    Some(Enclosure {
        url: url.into(),
        kind,
    })
}
