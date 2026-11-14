//! Streaming, never-failing parser for Isonomic Views markup.
//!
//! `parse` accepts any text, including any prefix of a document that is still being written,
//! and always returns a tree. Unclosed tags are closed; an incomplete tag name, attribute,
//! expression or closing tag at the end of the input is held back; malformed input becomes
//! diagnostics with positions. Nothing panics.

mod scan;

pub use scan::{line_col, parse};

/// Byte range in the source.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Element(Element),
    Text(Text),
    Expr(ExprChild),
}

impl Node {
    pub fn span(&self) -> Span {
        match self {
            Node::Element(e) => e.span,
            Node::Text(t) => t.span,
            Node::Expr(x) => x.span,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub tag: String,
    pub attrs: Vec<Attr>,
    pub children: Vec<Node>,
    /// From `<` to the end of the closing tag, or to the end of the input.
    pub span: Span,
    /// Where the tag name sits.
    pub name_span: Span,
    /// Closed explicitly (`</Tag>` or `/>`), rather than by the end of the input.
    pub closed: bool,
}

impl Element {
    pub fn attr(&self, name: &str) -> Option<&Attr> {
        self.attrs.iter().find(|a| a.name == name)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Attr {
    pub name: String,
    pub value: AttrValue,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AttrValue {
    /// `name="text"`
    Str(String),
    /// `name={expression}`; the source between the braces.
    Expr(String),
    /// `name` with no value: true.
    Flag,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    /// Whitespace runs collapsed to one space; entities decoded.
    pub text: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExprChild {
    pub src: String,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    pub span: Span,
}

/// A parsed document: top-level nodes plus diagnostics.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Document {
    pub nodes: Vec<Node>,
    pub diagnostics: Vec<Diagnostic>,
    /// Number of bytes that produced the tree; anything after was held back.
    pub consumed: usize,
}

impl Document {
    /// The first top-level element, which a view expects to be `<View>`.
    pub fn root(&self) -> Option<&Element> {
        self.nodes.iter().find_map(|n| match n {
            Node::Element(e) => Some(e),
            _ => None,
        })
    }
}

/// Elements whose text keeps its line breaks and indentation.
pub const PRESERVE_WHITESPACE: &[&str] = &["Markdown", "Code"];

/// Elements whose content is literal text up to the closing tag: no tags, no expressions.
pub const LITERAL: &[&str] = &["Code"];

/// Deepest element nesting the parser builds. Deeper tags are ignored with a diagnostic.
pub const MAX_DEPTH: usize = 128;
