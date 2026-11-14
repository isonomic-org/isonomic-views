//! The Isonomic Views node map.
//!
//! A view is a flat map of nodes keyed by stable ID. Each node has a type, literal props,
//! bindings (expression props), children and a parent. IDs survive streaming (the source
//! growing) and edits elsewhere in the source, because each re-parse is aligned against the
//! previous map before IDs are assigned. Changes between two maps are expressed as patches.

mod artifact;
mod build;
mod diff;
mod limits;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub use artifact::{Artifact, Edit, EditError};
pub use build::fingerprint;
pub use diff::{apply, diff};
pub use limits::Limits;
pub use views_markup as markup;

pub type NodeId = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Element,
    /// Literal text.
    Text,
    /// `{expression}` inside text; the source is the `value` binding.
    Expr,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl From<markup::Span> for Span {
    fn from(s: markup::Span) -> Self {
        Span {
            start: s.start,
            end: s.end,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub kind: Kind,
    /// Element tag; `#text` or `#expr` otherwise.
    pub tag: String,
    /// Literal attributes (`name="text"`; flags are `"true"`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub props: BTreeMap<String, String>,
    /// Expression attributes (`name={source}`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub bindings: BTreeMap<String, String>,
    /// Text of a text node.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<NodeId>,
    pub parent: Option<NodeId>,
    pub span: Span,
    /// The source for this node has fully arrived (element explicitly closed).
    pub complete: bool,
}

impl Node {
    pub fn is(&self, tag: &str) -> bool {
        self.kind == Kind::Element && self.tag == tag
    }

    pub fn prop(&self, name: &str) -> Option<&str> {
        self.props.get(name).map(String::as_str)
    }

    pub fn binding(&self, name: &str) -> Option<&str> {
        self.bindings.get(name).map(String::as_str)
    }

    pub fn has(&self, name: &str) -> bool {
        self.props.contains_key(name) || self.bindings.contains_key(name)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl From<markup::Severity> for Severity {
    fn from(s: markup::Severity) -> Self {
        match s {
            markup::Severity::Info => Severity::Info,
            markup::Severity::Warning => Severity::Warning,
            markup::Severity::Error => Severity::Error,
        }
    }
}

/// A machine-readable problem report. Shared by the parser, the limits and the validator.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<NodeId>,
    /// How to fix it, when there is a clear answer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl Diagnostic {
    pub fn new(code: impl Into<String>, severity: Severity, message: impl Into<String>) -> Self {
        Diagnostic {
            code: code.into(),
            severity,
            message: message.into(),
            span: None,
            node: None,
            hint: None,
        }
    }

    pub fn at(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn on(mut self, node: &Node) -> Self {
        self.node = Some(node.id);
        if self.span.is_none() {
            self.span = Some(node.span);
        }
        self
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

/// A view as a node map.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Doc {
    pub root: Option<NodeId>,
    pub nodes: BTreeMap<NodeId, Node>,
    /// Next ID to hand out. IDs are never reused within one document's life.
    pub next_id: NodeId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
}

impl Doc {
    /// Parse a complete or partial source with fresh IDs.
    pub fn parse(src: &str, limits: &Limits) -> Doc {
        build::build(src, None, limits)
    }

    /// Parse a new version of the source, keeping the IDs of nodes that still correspond,
    /// and return the patches that turn `self` into the result.
    pub fn reparse(&self, src: &str, limits: &Limits) -> (Doc, Vec<Patch>) {
        let next = build::build(src, Some(self), limits);
        let patches = diff(self, &next);
        (next, patches)
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    pub fn root_node(&self) -> Option<&Node> {
        self.root.and_then(|r| self.nodes.get(&r))
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Node IDs in document order (pre-order from the root).
    pub fn preorder(&self) -> Vec<NodeId> {
        let mut out = Vec::with_capacity(self.nodes.len());
        let mut stack: Vec<NodeId> = self.root.into_iter().collect();
        while let Some(id) = stack.pop() {
            out.push(id);
            if let Some(n) = self.nodes.get(&id) {
                stack.extend(n.children.iter().rev());
            }
        }
        out
    }

    /// Depth of a node (root = 0).
    pub fn depth(&self, id: NodeId) -> usize {
        let mut d = 0;
        let mut cur = self.nodes.get(&id).and_then(|n| n.parent);
        while let Some(p) = cur {
            d += 1;
            cur = self.nodes.get(&p).and_then(|n| n.parent);
        }
        d
    }

    /// Ancestors from the parent upward.
    pub fn ancestors(&self, id: NodeId) -> impl Iterator<Item = &Node> {
        let mut cur = self.nodes.get(&id).and_then(|n| n.parent);
        std::iter::from_fn(move || {
            let n = self.nodes.get(&cur?)?;
            cur = n.parent;
            Some(n)
        })
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(s: &str) -> Result<Doc, String> {
        serde_json::from_str(s).map_err(|e| e.to_string())
    }
}

/// The node content carried by an insert patch (children arrive as their own inserts).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeData {
    pub id: NodeId,
    pub kind: Kind,
    pub tag: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub props: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub bindings: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    pub span: Span,
    pub complete: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PropValue {
    /// A literal attribute.
    Lit { value: String },
    /// An expression attribute.
    Expr { source: String },
}

/// One change to a node map.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Patch {
    /// Add a node under `parent` (none: it becomes the root) at `index`.
    Insert {
        parent: Option<NodeId>,
        index: usize,
        node: NodeData,
    },
    /// Remove a node and everything under it.
    Remove { id: NodeId },
    /// Move a node to `index` under `parent`.
    Move {
        id: NodeId,
        parent: NodeId,
        index: usize,
    },
    /// Set or clear a prop. `None` clears it.
    SetProp {
        id: NodeId,
        name: String,
        value: Option<PropValue>,
    },
    /// Replace the text of a text node.
    SetText { id: NodeId, text: String },
    /// Update bookkeeping that carries no visible change (source span, completeness).
    SetMeta {
        id: NodeId,
        span: Span,
        complete: bool,
    },
}

impl Patch {
    /// Patches that change what a person sees (everything but bookkeeping).
    pub fn is_visible(&self) -> bool {
        !matches!(self, Patch::SetMeta { .. })
    }
}
