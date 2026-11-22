//! Reading HTML that somebody else wrote, within limits. The only place in
//! RSSifier that does; `clippy.toml` forbids the parser elsewhere.
//!
//! The limits and the counting are those of rueder's cleaning
//! (`server/src/cleaning.rs`), where they were measured: the parser looks
//! through all open tags for each new one, repeats formatting that is left
//! open in every paragraph that follows, and compares each attribute of a tag
//! with all before it. A fix to one belongs in the other too.

use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

use html5ever::{
    Attribute, ExpandedName, ParseOpts, QualName, parse_document,
    tendril::{StrTendril, TendrilSink},
    tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink},
};
use markup5ever_rcdom::{Handle, Node, RcDom};

/// How deep a page may nest its tags. Pages stay far below it; the parser's
/// work grows with the square of the depth.
pub const MAX_DEPTH: u32 = 256;
/// How many tags, pieces of text and comments a page may come to.
pub const MAX_NODES: usize = 200_000;
/// A page also comes to no more than one node for so many of its bytes, past
/// [`FREE_NODES`]. Formatting left open in every paragraph comes to more than
/// one a byte.
pub const BYTES_PER_NODE: usize = 2;
/// The nodes any page may come to, however short.
pub const FREE_NODES: usize = 1000;
/// How many attributes of one tag are read.
pub const MAX_ATTRIBUTES: usize = 256;
/// The page reaches the parser in pieces of this many bytes, and the limits
/// are looked at after each.
const PIECE: usize = 1024;

/// Builds the tree for the parser and keeps count of what it is asked to build.
struct Counting {
    dom: RcDom,
    nodes: Cell<usize>,
    deepest: Cell<u32>,
    /// How deep every element lies.
    depths: RefCell<HashMap<*const Node, u32>>,
    /// The most nodes this text may come to.
    most: usize,
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
        self.nodes.get() <= self.most && self.deepest.get() <= MAX_DEPTH
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

    /// The parser gives the attributes of each further `<html>` to the element
    /// that holds the text, looking for each among those it has: the square
    /// of their number again. They are never written out, so none are kept.
    fn add_attrs_if_missing(&self, _target: &Handle, _attrs: Vec<Attribute>) {}

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

/// Reads a page into a tree. `None` if the page nests too deep or comes to
/// too much (see [`MAX_DEPTH`], [`MAX_NODES`] and [`BYTES_PER_NODE`]): the
/// parser stops there, and a page like that is no page to make a feed of.
pub fn read(html: &str) -> Option<Handle> {
    let html = uncrowded(html);
    let sink = Counting {
        dom: RcDom::default(),
        nodes: Cell::new(0),
        deepest: Cell::new(0),
        depths: RefCell::default(),
        most: MAX_NODES.min(FREE_NODES + html.len() / BYTES_PER_NODE),
    };
    // The one place that reads HTML; `clippy.toml` forbids it elsewhere.
    #[expect(clippy::disallowed_methods)]
    let mut parser = parse_document(sink, ParseOpts::default());
    let mut rest = &*html;
    while !rest.is_empty() {
        let (piece, later) = rest.split_at(rest.floor_char_boundary(PIECE));
        parser.process(piece.into());
        if !parser.tokenizer.sink.sink.within_limits() {
            return None;
        }
        rest = later;
    }
    // What is still open is closed at the end, which can add to the tree.
    parser.finish()
}

/// The text without the attributes of a tag beyond [`MAX_ATTRIBUTES`]. Tags
/// are found as the parser finds them, as far as it matters for counting:
/// where a script or the like holds what looks like a tag, its text may lose
/// a little, and the cleaning drops it anyway.
fn uncrowded(html: &str) -> Cow<'_, str> {
    let bytes = html.as_bytes();
    let at_byte = |i: usize, test: &dyn Fn(u8) -> bool| bytes.get(i).is_some_and(|&b| test(b));
    let ends_name = |b: u8| b.is_ascii_whitespace() || b == b'/' || b == b'>';
    let mut kept = String::new();
    // Where the text still to be kept begins, and where to look on.
    let (mut from, mut at) = (0, 0);
    while let Some(found) = html.get(at..).and_then(|rest| rest.find('<')) {
        let mut i = at + found + 1;
        if html.get(i..).is_some_and(|rest| rest.starts_with("!--")) {
            at = html
                .get(i..)
                .and_then(|rest| rest.find("-->"))
                .map_or(html.len(), |end| i + end);
            continue;
        }
        i += usize::from(at_byte(i, &|b| b == b'/'));
        if !at_byte(i, &|b| b.is_ascii_alphabetic()) {
            at = i;
            continue;
        }
        while at_byte(i, &|b| !ends_name(b)) {
            i += 1;
        }
        let mut attributes = 0;
        loop {
            while at_byte(i, &|b| b.is_ascii_whitespace() || b == b'/') {
                i += 1;
            }
            if !at_byte(i, &|b| b != b'>') {
                break;
            }
            // A name, with an equals sign at its start, then maybe a value.
            let start = i;
            i += 1;
            while at_byte(i, &|b| !ends_name(b) && b != b'=') {
                i += 1;
            }
            let mut value = i;
            while at_byte(value, &|b| b.is_ascii_whitespace()) {
                value += 1;
            }
            if at_byte(value, &|b| b == b'=') {
                value += 1;
                while at_byte(value, &|b| b.is_ascii_whitespace()) {
                    value += 1;
                }
                i = match bytes.get(value) {
                    Some(&quote @ (b'"' | b'\'')) => html
                        .get(value + 1..)
                        .and_then(|rest| rest.find(char::from(quote)))
                        .map_or(html.len(), |end| value + end + 2),
                    _ => {
                        while at_byte(value, &|b| !b.is_ascii_whitespace() && b != b'>') {
                            value += 1;
                        }
                        value
                    }
                };
            }
            attributes += 1;
            if attributes > MAX_ATTRIBUTES {
                kept.push_str(html.get(from..start).unwrap_or_default());
                from = i;
            }
        }
        at = i;
    }
    if from == 0 {
        return Cow::Borrowed(html);
    }
    kept.push_str(html.get(from..).unwrap_or_default());
    Cow::Owned(kept)
}
