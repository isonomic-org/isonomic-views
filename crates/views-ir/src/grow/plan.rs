// Copyright 2026 Anud Inc.
// SPDX-License-Identifier: GPL-3.0-or-later

//! One streaming step: what the parser finished since the last step, as patches.

use std::collections::{HashMap, VecDeque};

use super::{Entry, Ir, Mode, Root, own_diags};
use crate::build::{self, Tmp};
use crate::diff::prop_changes;
use crate::markup::{self, Cut, Element, Stream};
use crate::{Doc, Limits, NodeData, NodeId, Patch, Span};

/// One step: the patches, and the state after them.
pub(super) struct Plan<'a> {
    p: &'a Stream,
    doc: &'a Doc,
    lim: &'a Limits,
    ir: Ir,
    old_spine: Vec<Entry>,
    /// Elements open last time that are now complete, outermost first.
    closed: VecDeque<Entry>,
    /// Last time's text still being written, until it is found again.
    tail: Option<(usize, Option<NodeId>)>,
    next_id: NodeId,
    patches: Vec<Patch>,
    /// Children added to each node in this step.
    added: HashMap<NodeId, usize>,
}

impl<'a> Plan<'a> {
    pub(super) fn new(p: &'a Stream, mut old: Ir, doc: &'a Doc, lim: &'a Limits) -> Plan<'a> {
        let old_spine = std::mem::take(&mut old.spine);
        let tail = old.tail.take();
        Plan {
            p,
            doc,
            lim,
            ir: old,
            old_spine,
            closed: VecDeque::new(),
            tail,
            next_id: doc.next_id,
            patches: Vec::new(),
            added: HashMap::new(),
        }
    }

    pub(super) fn run(mut self) -> Option<(Ir, Vec<Patch>)> {
        let p = self.p;
        let mut els: Vec<&Element> = p.open().iter().collect();
        if let Cut::Element(e) = p.cut() {
            els.push(e);
        }
        let common = self
            .old_spine
            .iter()
            .zip(&els)
            .take_while(|(e, el)| e.start == el.span.start)
            .count();
        let mut spine: Vec<Entry> = self.old_spine.drain(..common).collect();
        self.closed = self.old_spine.drain(..).collect();
        let consumed = p.consumed();
        // Open elements still open: their span grows; a partial tag may have gained attributes.
        for (k, (e, el)) in spine.iter().zip(&els).enumerate() {
            if k + 1 < common && el.children.len() != e.done {
                return None;
            }
            if let Some(id) = e.id() {
                self.head(
                    id,
                    el,
                    Span {
                        start: el.span.start,
                        end: consumed,
                    },
                    false,
                )?;
            }
        }
        if common == 0 {
            self.top_level()?;
        } else if p.top().len() != self.ir.top_done {
            return None;
        }
        if let Some(e) = spine.last_mut() {
            self.catch_up(e, els[common - 1])?;
        }
        for el in &els[common..] {
            let entry = self.open(spine.last(), el)?;
            spine.push(entry);
            let e = spine.last_mut()?;
            self.catch_up(e, el)?;
        }
        self.ir.spine = spine;
        self.text()?;
        if self.tail.is_some() || !self.closed.is_empty() {
            return None;
        }
        Some((self.ir, self.patches))
    }

    fn fresh(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn index(&mut self, parent: NodeId) -> usize {
        let have = self.doc.nodes.get(&parent).map_or(0, |n| n.children.len());
        let added = self.added.entry(parent).or_default();
        *added += 1;
        have + *added - 1
    }

    /// Insert a lowered subtree with fresh IDs, in document order.
    fn insert(&mut self, t: Tmp, parent: Option<NodeId>) -> NodeId {
        let index = parent.map_or(0, |p| self.index(p));
        let id = self.fresh();
        self.patches.push(Patch::Insert {
            parent,
            index,
            node: NodeData {
                id,
                kind: t.kind,
                tag: t.tag,
                props: t.props,
                bindings: t.bindings,
                text: t.text,
                span: t.span,
                complete: t.complete,
            },
        });
        for c in t.children {
            self.insert(c, Some(id));
        }
        id
    }

    /// Patches that bring an existing element up to date.
    fn head(&mut self, id: NodeId, el: &Element, span: Span, complete: bool) -> Option<()> {
        let node = self.doc.nodes.get(&id)?;
        if node.tag != el.tag {
            return None;
        }
        let (props, bindings) = build::head(el, self.lim, &mut Vec::new());
        let new_key = props.get("key").or(bindings.get("key"));
        if let (Some(a), Some(b)) = (node.prop("key").or(node.binding("key")), new_key)
            && a != b
        {
            return None;
        }
        prop_changes(
            id,
            &node.props,
            &props,
            &node.bindings,
            &bindings,
            &mut self.patches,
        );
        if node.span != span || node.complete != complete {
            self.patches.push(Patch::SetMeta { id, span, complete });
        }
        Some(())
    }

    /// Patches that bring an existing text node up to date.
    fn retext(&mut self, id: NodeId, t: &Tmp) -> Option<()> {
        let node = self.doc.nodes.get(&id)?;
        if node.kind != t.kind {
            return None;
        }
        if node.text != t.text {
            self.patches.push(Patch::SetText {
                id,
                text: t.text.clone(),
            });
        }
        if node.span != t.span || node.complete != t.complete {
            self.patches.push(Patch::SetMeta {
                id,
                span: t.span,
                complete: t.complete,
            });
        }
        Some(())
    }

    /// Take last time's text still being written if it starts at `start`.
    fn take_tail(&mut self, start: usize) -> Option<Option<NodeId>> {
        match self.tail {
            Some((s, id)) if s == start => {
                self.tail = None;
                Some(id)
            }
            _ => None,
        }
    }

    /// Lower the final children of `el` that `e` has not handled yet.
    fn catch_up(&mut self, e: &mut Entry, el: &Element) -> Option<()> {
        while e.done < el.children.len() {
            let c = &el.children[e.done];
            if let markup::Node::Element(ce) = c
                && self
                    .closed
                    .front()
                    .is_some_and(|f| f.start == ce.span.start)
            {
                let f = self.closed.pop_front()?;
                self.close(e, f, ce)?;
                continue;
            }
            let reuse = match c {
                markup::Node::Text(t) => self.take_tail(t.span.start),
                _ => None,
            };
            if e.lowers_children(self.lim) {
                let t = build::lower(
                    c,
                    e.depth + 1,
                    self.lim,
                    &mut self.ir.remaining,
                    &mut e.diags,
                );
                match (t, reuse) {
                    (Some(t), Some(Some(id))) => self.retext(id, &t)?,
                    (Some(t), None) => {
                        self.insert(t, e.id());
                    }
                    (None, None | Some(None)) => {}
                    _ => return None,
                }
            } else if let Some(Some(_)) = reuse {
                return None;
            }
            e.done += 1;
        }
        Some(())
    }

    /// An element open last time is now a final child of `parent`.
    fn close(&mut self, parent: &mut Entry, mut f: Entry, el: &Element) -> Option<()> {
        if let Some(id) = f.id() {
            self.head(id, el, el.span.into(), el.closed)?;
        }
        self.catch_up(&mut f, el)?;
        let own = own_diags(&f, el, el.span.into(), !el.children.is_empty(), self.lim);
        parent.diags.extend(own);
        parent.diags.append(&mut f.diags);
        parent.done += 1;
        Some(())
    }

    /// New final top-level nodes: the root completing, or nodes outside it.
    fn top_level(&mut self) -> Option<()> {
        let top = self.p.top();
        while self.ir.top_done < top.len() {
            let n = &top[self.ir.top_done];
            match n {
                markup::Node::Element(e) if self.ir.root == Root::Open => {
                    let mut f = self.closed.pop_front()?;
                    if f.start != e.span.start {
                        return None;
                    }
                    if let Some(id) = f.id() {
                        self.head(id, e, e.span.into(), e.closed)?;
                    }
                    self.catch_up(&mut f, e)?;
                    let mut d = own_diags(&f, e, e.span.into(), !e.children.is_empty(), self.lim);
                    d.append(&mut f.diags);
                    self.ir.root_diags = d;
                    self.ir.root = Root::Closed;
                }
                markup::Node::Element(e) if self.ir.root == Root::None => {
                    let mut d = Vec::new();
                    if let Some(t) = build::lower_el(e, 0, self.lim, &mut self.ir.remaining, &mut d)
                    {
                        self.insert(t, None);
                    }
                    self.ir.root_diags = d;
                    self.ir.root = Root::Closed;
                }
                other => {
                    match other {
                        markup::Node::Text(t) => {
                            self.take_tail(t.span.start);
                        }
                        markup::Node::Element(e)
                            if self.closed.front().is_some_and(|f| f.start == e.span.start) =>
                        {
                            // An element outside the root that was open: nothing in it is lowered.
                            let mut f = self.closed.pop_front()?;
                            self.catch_up(&mut f, e)?;
                        }
                        _ => {}
                    }
                    self.ir.top_diags.push(build::outside_root(other));
                }
            }
            self.ir.top_done += 1;
        }
        Some(())
    }

    /// An element opened since last time, under `parent` (none: at the top level).
    fn open(&mut self, parent: Option<&Entry>, el: &Element) -> Option<Entry> {
        let (visited, depth, parent_id) = match parent {
            None => {
                let is_root = self.ir.root == Root::None;
                if is_root {
                    self.ir.root = Root::Open;
                }
                (is_root, 0, None)
            }
            Some(p) => (p.lowers_children(self.lim), p.depth + 1, p.id()),
        };
        let mode = if !visited {
            Mode::Unseen
        } else if self.ir.remaining == 0 {
            Mode::Dropped
        } else {
            self.ir.remaining -= 1;
            let (props, bindings) = build::head(el, self.lim, &mut Vec::new());
            let t = Tmp {
                kind: crate::Kind::Element,
                tag: el.tag.clone(),
                key: None,
                props,
                bindings,
                text: String::new(),
                span: Span {
                    start: el.span.start,
                    end: self.p.consumed(),
                },
                complete: false,
                children: Vec::new(),
                fp: 0,
            };
            Mode::Lowered(self.insert(t, parent_id))
        };
        Some(Entry {
            start: el.span.start,
            mode,
            depth,
            done: 0,
            diags: Vec::new(),
        })
    }

    /// The text still being written, as the last child of the innermost open element.
    fn text(&mut self) -> Option<()> {
        let Cut::Text(t) = self.p.cut() else {
            return Some(());
        };
        let start = t.span.start;
        let reuse = self.take_tail(start);
        let lowered = match self.ir.spine.last() {
            Some(e) if e.lowers_children(self.lim) => {
                let mut budget = self.ir.remaining;
                let node = markup::Node::Text(t.clone());
                build::lower(&node, e.depth + 1, self.lim, &mut budget, &mut Vec::new())
                    .map(|tmp| (tmp, e.id()))
            }
            _ => None,
        };
        let id = match (lowered, reuse) {
            (Some((tmp, _)), Some(Some(id))) => {
                self.retext(id, &tmp)?;
                Some(id)
            }
            (Some((tmp, parent)), None) => Some(self.insert(tmp, parent)),
            (None, None | Some(None)) => None,
            _ => return None,
        };
        self.ir.tail = Some((start, id));
        Some(())
    }
}
