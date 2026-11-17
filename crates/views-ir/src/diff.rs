// Copyright 2026 Anud Inc.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Patches between two node maps that share IDs.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::{Doc, NodeData, NodeId, Patch, PropValue};

/// The patches that turn `old` into `new`. Removes come first (only the top of each removed
/// subtree), then, in the new document order, inserts, moves and prop and text changes.
/// Moves are minimal: siblings that keep their relative order are not moved.
pub fn diff(old: &Doc, new: &Doc) -> Vec<Patch> {
    let mut out = Vec::new();
    for id in old.preorder() {
        if !new.nodes.contains_key(&id) {
            let parent_removed = old.nodes[&id]
                .parent
                .is_some_and(|p| !new.nodes.contains_key(&p));
            if !parent_removed {
                out.push(Patch::Remove { id });
            }
        }
    }
    let mut stays: HashSet<NodeId> = HashSet::new();
    for n in new.nodes.values() {
        let Some(o) = old.nodes.get(&n.id) else {
            continue;
        };
        let old_kids: Vec<NodeId> = o
            .children
            .iter()
            .copied()
            .filter(|c| new.nodes.get(c).is_some_and(|x| x.parent == Some(n.id)))
            .collect();
        let pos: HashMap<NodeId, usize> =
            old_kids.iter().enumerate().map(|(i, c)| (*c, i)).collect();
        let seq: Vec<(NodeId, usize)> = n
            .children
            .iter()
            .filter_map(|c| pos.get(c).map(|p| (*c, *p)))
            .collect();
        stays.extend(lis(&seq));
    }
    for id in new.preorder() {
        let n = &new.nodes[&id];
        let index = n
            .parent
            .and_then(|p| new.nodes[&p].children.iter().position(|c| *c == id))
            .unwrap_or(0);
        match old.nodes.get(&id) {
            None => out.push(Patch::Insert {
                parent: n.parent,
                index,
                node: data(n),
            }),
            Some(o) => {
                if (n.parent != o.parent || (n.parent.is_some() && !stays.contains(&id)))
                    && let Some(parent) = n.parent
                {
                    out.push(Patch::Move { id, parent, index });
                }
                prop_changes(id, &o.props, &n.props, &o.bindings, &n.bindings, &mut out);
                if o.text != n.text {
                    out.push(Patch::SetText {
                        id,
                        text: n.text.clone(),
                    });
                }
                if o.span != n.span || o.complete != n.complete {
                    out.push(Patch::SetMeta {
                        id,
                        span: n.span,
                        complete: n.complete,
                    });
                }
            }
        }
    }
    out
}

pub(crate) fn data(n: &crate::Node) -> NodeData {
    NodeData {
        id: n.id,
        kind: n.kind,
        tag: n.tag.clone(),
        props: n.props.clone(),
        bindings: n.bindings.clone(),
        text: n.text.clone(),
        span: n.span,
        complete: n.complete,
    }
}

pub(crate) fn prop_changes(
    id: NodeId,
    op: &BTreeMap<String, String>,
    np: &BTreeMap<String, String>,
    ob: &BTreeMap<String, String>,
    nb: &BTreeMap<String, String>,
    out: &mut Vec<Patch>,
) {
    let names: std::collections::BTreeSet<&String> = op
        .keys()
        .chain(np.keys())
        .chain(ob.keys())
        .chain(nb.keys())
        .collect();
    for name in names {
        let before = op
            .get(name)
            .map(|v| PropValue::Lit { value: v.clone() })
            .or_else(|| ob.get(name).map(|v| PropValue::Expr { source: v.clone() }));
        let after = np
            .get(name)
            .map(|v| PropValue::Lit { value: v.clone() })
            .or_else(|| nb.get(name).map(|v| PropValue::Expr { source: v.clone() }));
        if before != after {
            out.push(Patch::SetProp {
                id,
                name: name.clone(),
                value: after,
            });
        }
    }
}

/// IDs on a longest increasing subsequence of old positions.
fn lis(seq: &[(NodeId, usize)]) -> Vec<NodeId> {
    let mut tails: Vec<usize> = Vec::new();
    let mut prev = vec![usize::MAX; seq.len()];
    for i in 0..seq.len() {
        let k = tails.partition_point(|&t| seq[t].1 < seq[i].1);
        if k > 0 {
            prev[i] = tails[k - 1];
        }
        if k == tails.len() {
            tails.push(i);
        } else {
            tails[k] = i;
        }
    }
    let mut out = Vec::new();
    let mut cur = tails.last().copied();
    while let Some(i) = cur {
        out.push(seq[i].0);
        cur = (prev[i] != usize::MAX).then_some(prev[i]);
    }
    out
}

/// Apply patches to a map. Unknown IDs are ignored.
pub fn apply(doc: &mut Doc, patches: &[Patch]) {
    for p in patches {
        match p {
            Patch::Remove { id } => remove(doc, *id),
            Patch::Insert {
                parent,
                index,
                node,
            } => {
                let n = crate::Node {
                    id: node.id,
                    kind: node.kind,
                    tag: node.tag.clone(),
                    props: node.props.clone(),
                    bindings: node.bindings.clone(),
                    text: node.text.clone(),
                    children: vec![],
                    parent: *parent,
                    span: node.span,
                    complete: node.complete,
                };
                doc.nodes.insert(node.id, n);
                doc.next_id = doc.next_id.max(node.id + 1);
                match parent {
                    Some(par) => place(doc, node.id, *par, *index),
                    None => doc.root = Some(node.id),
                }
            }
            Patch::Move { id, parent, index } => {
                if let Some(old) = doc.nodes.get(id).and_then(|n| n.parent)
                    && let Some(o) = doc.nodes.get_mut(&old)
                {
                    o.children.retain(|c| c != id);
                }
                if let Some(n) = doc.nodes.get_mut(id) {
                    n.parent = Some(*parent);
                }
                place(doc, *id, *parent, *index);
            }
            Patch::SetProp { id, name, value } => {
                if let Some(n) = doc.nodes.get_mut(id) {
                    n.props.remove(name);
                    n.bindings.remove(name);
                    match value {
                        Some(PropValue::Lit { value }) => {
                            n.props.insert(name.clone(), value.clone());
                        }
                        Some(PropValue::Expr { source }) => {
                            n.bindings.insert(name.clone(), source.clone());
                        }
                        None => {}
                    }
                }
            }
            Patch::SetText { id, text } => {
                if let Some(n) = doc.nodes.get_mut(id) {
                    n.text = text.clone();
                }
            }
            Patch::SetMeta { id, span, complete } => {
                if let Some(n) = doc.nodes.get_mut(id) {
                    n.span = *span;
                    n.complete = *complete;
                }
            }
        }
    }
}

fn place(doc: &mut Doc, id: NodeId, parent: NodeId, index: usize) {
    if let Some(p) = doc.nodes.get_mut(&parent) {
        p.children.retain(|c| *c != id);
        let i = index.min(p.children.len());
        p.children.insert(i, id);
    }
}

fn remove(doc: &mut Doc, id: NodeId) {
    let Some(n) = doc.nodes.remove(&id) else {
        return;
    };
    if let Some(p) = n.parent.and_then(|p| doc.nodes.get_mut(&p)) {
        p.children.retain(|c| *c != id);
    }
    if doc.root == Some(id) {
        doc.root = None;
    }
    let mut stack = n.children;
    while let Some(c) = stack.pop() {
        if let Some(cn) = doc.nodes.remove(&c) {
            stack.extend(cn.children);
        }
    }
}
