//! Lowering markup to a node map, keeping IDs from the previous map.

use std::{
    collections::{BTreeMap, HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
};

use crate::{Diagnostic, Doc, Kind, Limits, Node, NodeId, Severity, Span, markup};

/// A lowered node before IDs are assigned.
pub(crate) struct Tmp {
    pub kind: Kind,
    pub tag: String,
    pub key: Option<String>,
    pub props: BTreeMap<String, String>,
    pub bindings: BTreeMap<String, String>,
    pub text: String,
    pub span: Span,
    pub complete: bool,
    pub children: Vec<Tmp>,
    pub fp: u64,
}

/// How much of the source is read, and the diagnostic when that is not all of it.
pub(crate) fn source_cut(src: &str, limits: &Limits) -> (usize, Option<Diagnostic>) {
    let mut cut = src.len().min(limits.max_source);
    while !src.is_char_boundary(cut) {
        cut -= 1;
    }
    let diag = (cut < src.len()).then(|| {
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
        })
    });
    (cut, diag)
}

pub(crate) fn from_markup(d: &markup::Diagnostic) -> Diagnostic {
    Diagnostic::new(d.code, d.severity.into(), d.message.clone()).at(d.span.into())
}

/// The diagnostic for a top-level node that is not the root.
pub(crate) fn outside_root(n: &markup::Node) -> Diagnostic {
    match n {
        markup::Node::Element(e) => outside_root_el(e),
        other => Diagnostic::new(
            "outside-root",
            Severity::Warning,
            "content outside the root element is ignored",
        )
        .at(other.span().into()),
    }
}

pub(crate) fn outside_root_el(e: &markup::Element) -> Diagnostic {
    Diagnostic::new(
        "extra-root",
        Severity::Warning,
        format!("<{}> is outside the root element and is ignored", e.tag),
    )
    .at(e.span.into())
}

pub fn build(src: &str, prev: Option<&Doc>, limits: &Limits) -> Doc {
    let (cut, cut_diag) = source_cut(src, limits);
    let mut diagnostics: Vec<Diagnostic> = cut_diag.into_iter().collect();
    let md = markup::parse(&src[..cut]);
    diagnostics.extend(md.diagnostics.iter().map(from_markup));

    let mut budget = limits.max_nodes;
    let mut root_el = None;
    for n in &md.nodes {
        match n {
            markup::Node::Element(e) if root_el.is_none() => root_el = Some(e),
            other => diagnostics.push(outside_root(other)),
        }
    }
    let root_tmp = root_el.map(|e| lower_el(e, 0, limits, &mut budget, &mut diagnostics));

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

/// The diagnostic for a node dropped because the node budget is spent.
pub(crate) fn over_budget(limits: &Limits, span: markup::Span) -> Diagnostic {
    Diagnostic::new(
        "limit-nodes",
        Severity::Error,
        format!("more than {} nodes; the rest are dropped", limits.max_nodes),
    )
    .at(span.into())
}

/// The diagnostic for an element whose children are dropped because it is nested too deeply.
pub(crate) fn too_deep(limits: &Limits, e: &markup::Element) -> Diagnostic {
    Diagnostic::new(
        "limit-depth",
        Severity::Error,
        format!("nesting deeper than {} is dropped", limits.max_depth),
    )
    .at(e.name_span.into())
}

fn clip(s: &str, limits: &Limits, diags: &mut Vec<Diagnostic>, span: markup::Span) -> String {
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
}

pub(crate) type Attrs = (BTreeMap<String, String>, BTreeMap<String, String>);

/// An element's literal props and bindings, with diagnostics for values that are too long.
pub(crate) fn head(e: &markup::Element, limits: &Limits, diags: &mut Vec<Diagnostic>) -> Attrs {
    let mut props = BTreeMap::new();
    let mut bindings = BTreeMap::new();
    for a in &e.attrs {
        match &a.value {
            markup::AttrValue::Str(s) => {
                props.insert(a.name.clone(), clip(s, limits, diags, a.span));
            }
            markup::AttrValue::Flag => {
                props.insert(a.name.clone(), "true".into());
            }
            markup::AttrValue::Expr(s) => {
                bindings.insert(a.name.clone(), clip(s, limits, diags, a.span));
            }
        }
    }
    (props, bindings)
}

pub(crate) fn lower_el(
    e: &markup::Element,
    depth: usize,
    limits: &Limits,
    budget: &mut usize,
    diags: &mut Vec<Diagnostic>,
) -> Option<Tmp> {
    if *budget == 0 {
        diags.push(over_budget(limits, e.span));
        return None;
    }
    *budget -= 1;
    let (props, bindings) = head(e, limits, diags);
    let key = props.get("key").or(bindings.get("key")).cloned();
    let mut children = Vec::new();
    if depth >= limits.max_depth && !e.children.is_empty() {
        diags.push(too_deep(limits, e));
    } else {
        for c in &e.children {
            if let Some(ct) = lower(c, depth + 1, limits, budget, diags) {
                children.push(ct);
            }
        }
    }
    Some(finish(Tmp {
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
    }))
}

pub(crate) fn lower(
    n: &markup::Node,
    depth: usize,
    limits: &Limits,
    budget: &mut usize,
    diags: &mut Vec<Diagnostic>,
) -> Option<Tmp> {
    let leaf = |budget: &mut usize, diags: &mut Vec<Diagnostic>, kind, tag: &str| {
        if *budget == 0 {
            diags.push(over_budget(limits, n.span()));
            return None;
        }
        *budget -= 1;
        Some(Tmp {
            kind,
            tag: tag.into(),
            key: None,
            props: BTreeMap::new(),
            bindings: BTreeMap::new(),
            text: String::new(),
            span: n.span().into(),
            complete: true,
            children: vec![],
            fp: 0,
        })
    };
    let t = match n {
        markup::Node::Element(e) => return lower_el(e, depth, limits, budget, diags),
        markup::Node::Text(x) => {
            let mut t = leaf(budget, diags, Kind::Text, "#text")?;
            t.text = clip(&x.text, limits, diags, x.span);
            t.complete = x.complete;
            t
        }
        markup::Node::Expr(x) => {
            let mut t = leaf(budget, diags, Kind::Expr, "#expr")?;
            let src = clip(&x.src, limits, diags, x.span);
            t.bindings.insert("value".into(), src);
            t
        }
    };
    Some(finish(t))
}

fn finish(mut t: Tmp) -> Tmp {
    t.fp = {
        let mut h = DefaultHasher::new();
        (t.kind, &t.tag, &t.props, &t.bindings, &t.text).hash(&mut h);
        for c in &t.children {
            c.fp.hash(&mut h);
        }
        h.finish()
    };
    t
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
/// preferring identical subtrees (score 3) over same-tag nodes (score 1). Only a node whose
/// source has fully arrived counts as identical, and ties go to the earliest match, so a
/// source that only grew keeps every node where it was.
fn align(prev: Option<&Doc>, old: &[&Node], new: &[Tmp]) -> Vec<Option<NodeId>> {
    let mut out = vec![None; new.len()];
    let Some(prev) = prev else { return out };
    if old.is_empty() {
        return out;
    }
    let key_of = |o: &Node| o.prop("key").or(o.binding("key")).map(str::to_string);
    let old_keys: Vec<Option<String>> = old.iter().map(|o| key_of(o)).collect();
    let mut used = vec![false; old.len()];
    for (j, n) in new.iter().enumerate() {
        if let Some(k) = &n.key
            && let Some(i) = (0..old.len())
                .position(|i| !used[i] && old[i].tag == n.tag && old_keys[i].as_ref() == Some(k))
        {
            used[i] = true;
            out[j] = Some(old[i].id);
        }
    }
    let oi: Vec<usize> = (0..old.len()).filter(|&i| !used[i]).collect();
    let nj: Vec<usize> = (0..new.len()).filter(|&j| out[j].is_none()).collect();
    if oi.is_empty() || nj.is_empty() {
        return out;
    }
    let mut memo = HashMap::new();
    let ofp: Vec<u64> = oi
        .iter()
        .map(|&i| fp_of(prev, old[i].id, &mut memo))
        .collect();
    let compatible = |a: usize, b: usize| -> bool {
        let (o, n) = (old[oi[a]], &new[nj[b]]);
        let keys_clash = matches!((&old_keys[oi[a]], &n.key), (Some(x), Some(y)) if x != y);
        o.kind == n.kind && o.tag == n.tag && !keys_clash
    };
    let identical = |a: usize, b: usize| old[oi[a]].complete && ofp[a] == new[nj[b]].fp;
    let score = |a: usize, b: usize| -> i64 {
        if !compatible(a, b) {
            -1
        } else if identical(a, b) {
            3
        } else {
            1
        }
    };
    // Weighted longest common subsequence over suffixes, read forward so ties keep the
    // earliest pairs.
    let (h, w) = (oi.len(), nj.len());
    let at = |a: usize, b: usize| a * (w + 1) + b;
    let mut dp = vec![0i64; (h + 1) * (w + 1)];
    for a in (0..h).rev() {
        for b in (0..w).rev() {
            let mut best = dp[at(a + 1, b)].max(dp[at(a, b + 1)]);
            let s = score(a, b);
            if s > 0 {
                best = best.max(dp[at(a + 1, b + 1)] + s);
            }
            dp[at(a, b)] = best;
        }
    }
    let (mut a, mut b) = (0, 0);
    while a < h && b < w {
        let s = score(a, b);
        let here = dp[at(a, b)];
        if s > 0 && here == dp[at(a + 1, b + 1)] + s {
            out[nj[b]] = Some(old[oi[a]].id);
            a += 1;
            b += 1;
        } else if here == dp[at(a, b + 1)] {
            b += 1;
        } else {
            a += 1;
        }
    }
    // Moved subtrees: identical content out of order.
    let taken: std::collections::HashSet<NodeId> = out.iter().flatten().copied().collect();
    let mut free: Vec<usize> = (0..h)
        .filter(|&a| !taken.contains(&old[oi[a]].id))
        .collect();
    for (b, &j) in nj.iter().enumerate() {
        if out[j].is_none()
            && let Some(k) = free
                .iter()
                .position(|&a| compatible(a, b) && identical(a, b))
        {
            out[j] = Some(old[oi[free.remove(k)]].id);
        }
    }
    out
}
