//! Lowering markup to a node map, keeping IDs from the previous map.

use std::{
    collections::{BTreeMap, HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
};

use crate::{Diagnostic, Doc, Kind, Limits, Node, NodeId, Severity, Span, markup};

/// A lowered node before IDs are assigned.
struct Tmp {
    kind: Kind,
    tag: String,
    key: Option<String>,
    props: BTreeMap<String, String>,
    bindings: BTreeMap<String, String>,
    text: String,
    span: Span,
    complete: bool,
    children: Vec<Tmp>,
    fp: u64,
}

pub fn build(src: &str, prev: Option<&Doc>, limits: &Limits) -> Doc {
    let mut diagnostics = Vec::new();
    let mut cut = src.len().min(limits.max_source);
    while !src.is_char_boundary(cut) {
        cut -= 1;
    }
    if cut < src.len() {
        diagnostics.push(
            Diagnostic::new(
                "limit-source",
                Severity::Error,
                format!(
                    "source is longer than {} bytes; the rest is ignored",
                    limits.max_source
                ),
            )
            .at(Span {
                start: cut,
                end: src.len(),
            }),
        );
    }
    let md = markup::parse(&src[..cut]);
    diagnostics.extend(
        md.diagnostics.iter().map(|d| {
            Diagnostic::new(d.code, d.severity.into(), d.message.clone()).at(d.span.into())
        }),
    );

    let mut budget = limits.max_nodes;
    let mut root_el = None;
    for n in &md.nodes {
        match n {
            markup::Node::Element(e) if root_el.is_none() => root_el = Some(e),
            markup::Node::Element(e) => diagnostics.push(
                Diagnostic::new(
                    "extra-root",
                    Severity::Warning,
                    format!("<{}> is outside the root element and is ignored", e.tag),
                )
                .at(e.span.into()),
            ),
            other => diagnostics.push(
                Diagnostic::new(
                    "outside-root",
                    Severity::Warning,
                    "content outside the root element is ignored",
                )
                .at(other.span().into()),
            ),
        }
    }
    let root_tmp = root_el.map(|e| {
        lower(
            &markup::Node::Element(e.clone()),
            0,
            limits,
            &mut budget,
            &mut diagnostics,
        )
    });

    let mut doc = Doc {
        root: None,
        nodes: BTreeMap::new(),
        next_id: prev.map_or(1, |p| p.next_id),
        diagnostics,
    };
    if let Some(t) = root_tmp.flatten() {
        let old_root = prev
            .and_then(|p| p.root_node())
            .filter(|r| r.kind == t.kind && r.tag == t.tag);
        let id = insert(&mut doc, prev, t, old_root.map(|r| r.id), None);
        doc.root = Some(id);
    }
    doc
}

fn lower(
    n: &markup::Node,
    depth: usize,
    limits: &Limits,
    budget: &mut usize,
    diags: &mut Vec<Diagnostic>,
) -> Option<Tmp> {
    if *budget == 0 {
        diags.push(
            Diagnostic::new(
                "limit-nodes",
                Severity::Error,
                format!("more than {} nodes; the rest are dropped", limits.max_nodes),
            )
            .at(n.span().into()),
        );
        return None;
    }
    *budget -= 1;
    let clip = |s: &str, diags: &mut Vec<Diagnostic>, span: markup::Span| -> String {
        if s.len() <= limits.max_text {
            return s.to_string();
        }
        diags.push(
            Diagnostic::new(
                "limit-text",
                Severity::Error,
                format!("text longer than {} bytes is cut", limits.max_text),
            )
            .at(span.into()),
        );
        let mut c = limits.max_text;
        while !s.is_char_boundary(c) {
            c -= 1;
        }
        s[..c].to_string()
    };
    let mut t = match n {
        markup::Node::Text(x) => Tmp {
            kind: Kind::Text,
            tag: "#text".into(),
            key: None,
            props: BTreeMap::new(),
            bindings: BTreeMap::new(),
            text: clip(&x.text, diags, x.span),
            span: x.span.into(),
            complete: true,
            children: vec![],
            fp: 0,
        },
        markup::Node::Expr(x) => Tmp {
            kind: Kind::Expr,
            tag: "#expr".into(),
            key: None,
            props: BTreeMap::new(),
            bindings: BTreeMap::from([("value".to_string(), clip(&x.src, diags, x.span))]),
            text: String::new(),
            span: x.span.into(),
            complete: true,
            children: vec![],
            fp: 0,
        },
        markup::Node::Element(e) => {
            let mut props = BTreeMap::new();
            let mut bindings = BTreeMap::new();
            for a in &e.attrs {
                match &a.value {
                    markup::AttrValue::Str(s) => {
                        props.insert(a.name.clone(), clip(s, diags, a.span));
                    }
                    markup::AttrValue::Flag => {
                        props.insert(a.name.clone(), "true".into());
                    }
                    markup::AttrValue::Expr(s) => {
                        bindings.insert(a.name.clone(), clip(s, diags, a.span));
                    }
                }
            }
            let key = props.get("key").or(bindings.get("key")).cloned();
            let mut children = Vec::new();
            if depth >= limits.max_depth && !e.children.is_empty() {
                diags.push(
                    Diagnostic::new(
                        "limit-depth",
                        Severity::Error,
                        format!("nesting deeper than {} is dropped", limits.max_depth),
                    )
                    .at(e.name_span.into()),
                );
            } else {
                for c in &e.children {
                    if let Some(ct) = lower(c, depth + 1, limits, budget, diags) {
                        children.push(ct);
                    }
                }
            }
            Tmp {
                kind: Kind::Element,
                tag: e.tag.clone(),
                key,
                props,
                bindings,
                text: String::new(),
                span: e.span.into(),
                complete: e.closed,
                children,
                fp: 0,
            }
        }
    };
    t.fp = {
        let mut h = DefaultHasher::new();
        (t.kind, &t.tag, &t.props, &t.bindings, &t.text).hash(&mut h);
        for c in &t.children {
            c.fp.hash(&mut h);
        }
        h.finish()
    };
    Some(t)
}

/// Content fingerprint of a node subtree in a map; equal to the fingerprint the builder
/// computes for the same content.
pub fn fingerprint(doc: &Doc, id: NodeId) -> u64 {
    let mut memo = HashMap::new();
    fp_of(doc, id, &mut memo)
}

fn fp_of(doc: &Doc, id: NodeId, memo: &mut HashMap<NodeId, u64>) -> u64 {
    if let Some(f) = memo.get(&id) {
        return *f;
    }
    let Some(n) = doc.nodes.get(&id) else {
        return 0;
    };
    let mut h = DefaultHasher::new();
    (n.kind, &n.tag, &n.props, &n.bindings, &n.text).hash(&mut h);
    for c in &n.children {
        fp_of(doc, *c, memo).hash(&mut h);
    }
    let f = h.finish();
    memo.insert(id, f);
    f
}

fn insert(
    doc: &mut Doc,
    prev: Option<&Doc>,
    t: Tmp,
    reuse: Option<NodeId>,
    parent: Option<NodeId>,
) -> NodeId {
    let id = reuse.unwrap_or_else(|| {
        let id = doc.next_id;
        doc.next_id += 1;
        id
    });
    let old_children: Vec<&Node> = match (prev, reuse) {
        (Some(p), Some(r)) => p.nodes.get(&r).map_or(vec![], |n| {
            n.children.iter().filter_map(|c| p.nodes.get(c)).collect()
        }),
        _ => vec![],
    };
    let matches = align(prev, &old_children, &t.children);
    let mut kids = Vec::with_capacity(t.children.len());
    let tmp_children = t.children;
    let node = Node {
        id,
        kind: t.kind,
        tag: t.tag,
        props: t.props,
        bindings: t.bindings,
        text: t.text,
        children: vec![],
        parent,
        span: t.span,
        complete: t.complete,
    };
    doc.nodes.insert(id, node);
    for (c, m) in tmp_children.into_iter().zip(matches) {
        kids.push(insert(doc, prev, c, m, Some(id)));
    }
    if let Some(n) = doc.nodes.get_mut(&id) {
        n.children = kids;
    }
    id
}

/// Match new children to old ones. Explicit keys match first; the rest are aligned in order,
/// preferring identical subtrees (score 3) over same-tag nodes (score 1).
fn align(prev: Option<&Doc>, old: &[&Node], new: &[Tmp]) -> Vec<Option<NodeId>> {
    let mut out = vec![None; new.len()];
    let Some(prev) = prev else { return out };
    if old.is_empty() {
        return out;
    }
    let mut used = vec![false; old.len()];
    for (j, n) in new.iter().enumerate() {
        if let Some(k) = &n.key
            && let Some(i) = old.iter().enumerate().position(|(i, o)| {
                !used[i]
                    && o.tag == n.tag
                    && (o.prop("key").or(o.binding("key")) == Some(k.as_str()))
            })
        {
            used[i] = true;
            out[j] = Some(old[i].id);
        }
    }
    let oi: Vec<usize> = (0..old.len())
        .filter(|&i| !used[i] && old[i].prop("key").or(old[i].binding("key")).is_none())
        .collect();
    let nj: Vec<usize> = (0..new.len())
        .filter(|&j| out[j].is_none() && new[j].key.is_none())
        .collect();
    if oi.is_empty() || nj.is_empty() {
        return out;
    }
    let mut memo = HashMap::new();
    let ofp: Vec<u64> = oi
        .iter()
        .map(|&i| fp_of(prev, old[i].id, &mut memo))
        .collect();
    let score = |a: usize, b: usize| -> i64 {
        let (o, n) = (old[oi[a]], &new[nj[b]]);
        if o.kind != n.kind || o.tag != n.tag {
            -1
        } else if ofp[a] == n.fp {
            3
        } else {
            1
        }
    };
    // Weighted longest common subsequence.
    let (h, w) = (oi.len(), nj.len());
    let mut dp = vec![0i64; (h + 1) * (w + 1)];
    for a in 1..=h {
        for b in 1..=w {
            let mut best = dp[(a - 1) * (w + 1) + b].max(dp[a * (w + 1) + b - 1]);
            let s = score(a - 1, b - 1);
            if s > 0 {
                best = best.max(dp[(a - 1) * (w + 1) + b - 1] + s);
            }
            dp[a * (w + 1) + b] = best;
        }
    }
    let (mut a, mut b) = (h, w);
    while a > 0 && b > 0 {
        let s = score(a - 1, b - 1);
        let here = dp[a * (w + 1) + b];
        if s > 0 && here == dp[(a - 1) * (w + 1) + b - 1] + s {
            out[nj[b - 1]] = Some(old[oi[a - 1]].id);
            a -= 1;
            b -= 1;
        } else if here == dp[(a - 1) * (w + 1) + b] {
            a -= 1;
        } else {
            b -= 1;
        }
    }
    // Moved subtrees: identical content out of order.
    let taken: std::collections::HashSet<NodeId> = out.iter().flatten().copied().collect();
    let mut free: Vec<usize> = (0..h)
        .filter(|&a| !taken.contains(&old[oi[a]].id))
        .collect();
    for &j in &nj {
        if out[j].is_none()
            && let Some(k) = free
                .iter()
                .position(|&a| ofp[a] == new[j].fp && old[oi[a]].tag == new[j].tag)
        {
            out[j] = Some(old[oi[free.remove(k)]].id);
        }
    }
    out
}
