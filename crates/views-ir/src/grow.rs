//! Streaming: lowering a source that only grows, without touching what has settled.
//!
//! [`Grow`] keeps a [`markup::Stream`] and, for each open element, how far its lowering has
//! got. A step lowers only what the parser finished since the last step, updates the open
//! elements and the text still being written, and emits patches in the order [`crate::diff`]
//! does. The map and patches are the ones a full re-parse gives; when the parser's state is
//! not what this expects, the step returns `None` and the caller re-parses in full.

mod plan;

use std::collections::HashMap;

use crate::build;
use crate::markup::{self, Cut, Element, Stream};
use crate::{Diagnostic, Doc, Limits, NodeId, Patch, Span};
use plan::Plan;

#[derive(Clone, Debug, Default)]
pub(crate) struct Grow {
    parser: Stream,
    ir: Ir,
}

#[derive(Clone, Debug, Default)]
pub(super) struct Ir {
    /// Open elements, outermost first, then the opening tag still being written, if any.
    pub(super) spine: Vec<Entry>,
    /// Text still being written: where it starts and its node, if it has one.
    pub(super) tail: Option<(usize, Option<NodeId>)>,
    /// Node budget left after every node that stays.
    pub(super) remaining: usize,
    /// Final top-level nodes handled.
    pub(super) top_done: usize,
    pub(super) root: Root,
    /// Diagnostics for final top-level nodes other than the root.
    pub(super) top_diags: Vec<Diagnostic>,
    /// Lowering diagnostics of a complete root.
    pub(super) root_diags: Vec<Diagnostic>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Root {
    /// No top-level element yet.
    #[default]
    None,
    /// The root is the outermost open element.
    Open,
    /// The root is a final top-level node.
    Closed,
}

#[derive(Clone, Debug)]
pub(super) struct Entry {
    pub(super) start: usize,
    pub(super) mode: Mode,
    pub(super) depth: usize,
    /// Final children of the element handled so far.
    pub(super) done: usize,
    /// Diagnostics from lowering those children, in document order.
    pub(super) diags: Vec<Diagnostic>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mode {
    /// Not lowered: outside the root, or below an element whose children are dropped.
    Unseen,
    /// Reached after the node budget was spent.
    Dropped,
    Lowered(NodeId),
}

impl Entry {
    pub(super) fn id(&self) -> Option<NodeId> {
        match self.mode {
            Mode::Lowered(id) => Some(id),
            _ => None,
        }
    }

    pub(super) fn lowers_children(&self, limits: &Limits) -> bool {
        self.id().is_some() && self.depth < limits.max_depth
    }
}

/// An element's own lowering diagnostics, as `build` reports them at `span`.
pub(super) fn own_diags(
    e: &Entry,
    el: &Element,
    span: Span,
    has_children: bool,
    lim: &Limits,
) -> Vec<Diagnostic> {
    match e.mode {
        Mode::Unseen => Vec::new(),
        Mode::Dropped => vec![build::over_budget(
            lim,
            markup::Span::new(span.start, span.end),
        )],
        Mode::Lowered(_) => {
            let mut d = Vec::new();
            build::head(el, lim, &mut d);
            if e.depth >= lim.max_depth && has_children {
                d.push(build::too_deep(lim, el));
            }
            d
        }
    }
}

impl Grow {
    pub(crate) fn new(limits: &Limits) -> Grow {
        Grow {
            parser: Stream::new(),
            ir: Ir {
                remaining: limits.max_nodes,
                ..Ir::default()
            },
        }
    }

    /// Follow the source to `src`, which extends the source seen so far, updating `doc` in
    /// place. `None`: nothing was changed, and this state must be dropped.
    pub(crate) fn advance(&mut self, src: &str, doc: &mut Doc, lim: &Limits) -> Option<Vec<Patch>> {
        let (cut, _) = build::source_cut(src, lim);
        if cut < self.parser.len() {
            return None;
        }
        self.parser.advance(&src[..cut]);
        let old = std::mem::take(&mut self.ir);
        let (ir, patches) = Plan::new(&self.parser, old, &*doc, lim).run()?;
        self.ir = ir;
        crate::diff::apply(doc, &patches);
        doc.diagnostics = self.diagnostics(src, lim);
        Some(patches)
    }

    /// A state for `src` whose IDs are those of `doc`, a map of the same source.
    pub(crate) fn resync(src: &str, doc: &Doc, lim: &Limits) -> Option<Grow> {
        let mut g = Grow::new(lim);
        let mut scratch = Doc {
            next_id: 1,
            ..Doc::default()
        };
        g.advance(src, &mut scratch, lim)?;
        let (a, b) = (scratch.preorder(), doc.preorder());
        if a.len() != b.len()
            || a.len() != doc.nodes.len()
            || scratch.diagnostics != doc.diagnostics
        {
            return None;
        }
        let mut map = HashMap::with_capacity(a.len());
        for (x, y) in a.iter().zip(&b) {
            let (n, m) = (&scratch.nodes[x], &doc.nodes[y]);
            let same = n.kind == m.kind
                && n.tag == m.tag
                && n.props == m.props
                && n.bindings == m.bindings
                && n.text == m.text
                && n.span == m.span
                && n.complete == m.complete
                && n.children.len() == m.children.len();
            if !same {
                return None;
            }
            map.insert(*x, *y);
        }
        for e in &mut g.ir.spine {
            if let Mode::Lowered(id) = e.mode {
                e.mode = Mode::Lowered(*map.get(&id)?);
            }
        }
        if let Some((_, Some(id))) = &mut g.ir.tail {
            *id = *map.get(id)?;
        }
        Some(g)
    }

    /// Open elements as the tree shows them: the parser's stack, then a partial tag.
    fn spine_elements(&self) -> Vec<&Element> {
        let mut els: Vec<&Element> = self.parser.open().iter().collect();
        if let Cut::Element(e) = self.parser.cut() {
            els.push(e);
        }
        els
    }

    /// The diagnostics `build` reports for the same source, in the same order.
    fn diagnostics(&self, src: &str, lim: &Limits) -> Vec<Diagnostic> {
        let p = &self.parser;
        let (_, cut_diag) = build::source_cut(src, lim);
        let mut d: Vec<Diagnostic> = cut_diag.into_iter().collect();
        d.extend(p.diagnostics().iter().map(build::from_markup));
        d.extend(p.cut_diagnostics().iter().map(build::from_markup));
        let els = self.spine_elements();
        for e in els.iter().rev() {
            d.push(build::from_markup(&markup::unclosed(e)));
        }
        d.extend(self.ir.top_diags.iter().cloned());
        let consumed = p.consumed();
        let span = |e: &Element| Span {
            start: e.span.start,
            end: consumed,
        };
        let text = match p.cut() {
            Cut::Text(t) => Some(t),
            _ => None,
        };
        match (els.first(), text) {
            (Some(e), _) if self.ir.root == Root::Closed => {
                d.push(build::outside_root_el(e).at(span(e)));
            }
            (None, Some(t)) => d.push(build::outside_root(&markup::Node::Text(t.clone()))),
            _ => {}
        }
        match self.ir.root {
            Root::Closed => d.extend(self.ir.root_diags.iter().cloned()),
            Root::Open => {
                for (k, (entry, el)) in self.ir.spine.iter().zip(&els).enumerate() {
                    let last = k + 1 == els.len();
                    let kids = !el.children.is_empty() || !last || text.is_some();
                    d.extend(own_diags(entry, el, span(el), kids, lim));
                    d.extend(entry.diags.iter().cloned());
                }
                if let (Some(t), Some(e)) = (text, self.ir.spine.last())
                    && e.lowers_children(lim)
                {
                    let mut budget = self.ir.remaining;
                    let node = markup::Node::Text(t.clone());
                    build::lower(&node, e.depth + 1, lim, &mut budget, &mut d);
                }
            }
            Root::None => {}
        }
        d
    }
}
