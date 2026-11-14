//! Text content and `For` expansion.

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use views_expr::{Expr, Value};
use views_ir::{Diagnostic, Kind, NodeId, Severity};

use super::{Frame, Runtime, Var};

impl Runtime {
    /// Concatenated text of a node's text and `{expression}` children.
    pub fn text(&self, id: NodeId, frame: &Frame) -> String {
        let Some(n) = self.doc.nodes.get(&id) else {
            return String::new();
        };
        let mut out = String::new();
        for c in &n.children {
            let Some(cn) = self.doc.nodes.get(c) else {
                continue;
            };
            match cn.kind {
                Kind::Text => out.push_str(&cn.text),
                Kind::Expr => out.push_str(&self.eval(*c, "value", frame).to_text()),
                Kind::Element => {}
            }
        }
        if out.len() > self.limits.max_text {
            let mut cut = self.limits.max_text;
            while !out.is_char_boundary(cut) {
                cut -= 1;
            }
            out.truncate(cut);
        }
        out.trim().to_string()
    }

    /// Like `text`, but keeps line breaks: common indentation is removed and blank lines at
    /// either end are dropped. For Markdown and Code.
    pub fn text_raw(&self, id: NodeId, frame: &Frame) -> String {
        let Some(n) = self.doc.nodes.get(&id) else {
            return String::new();
        };
        let mut out = String::new();
        for c in &n.children {
            match self.doc.nodes.get(c) {
                Some(cn) if cn.kind == Kind::Text => out.push_str(&cn.text),
                Some(cn) if cn.kind == Kind::Expr => {
                    out.push_str(&self.eval(*c, "value", frame).to_text())
                }
                _ => {}
            }
        }
        let lines: Vec<&str> = out.lines().collect();
        let first = lines
            .iter()
            .position(|l| !l.trim().is_empty())
            .unwrap_or(lines.len());
        let last = lines
            .iter()
            .rposition(|l| !l.trim().is_empty())
            .map_or(first, |i| i + 1);
        let body = &lines[first..last.max(first)];
        let indent = body
            .iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);
        body.iter()
            .map(|l| l.get(indent..).unwrap_or(l.trim_start()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Frames for the items of a `For` node: `each`, `as` (default `item`), `index`
    /// (default `index`), optional `key`. At most `max_list` items.
    pub fn for_frames(&self, id: NodeId, frame: &Frame) -> Vec<Frame> {
        let list = self.eval(id, "each", frame);
        let items = match &list {
            Value::List(l) => l.clone(),
            _ => return vec![],
        };
        let n = &self.doc.nodes[&id];
        let item_name = n.prop("as").unwrap_or("item").to_string();
        let index_name = n.prop("index").unwrap_or("index").to_string();
        let source = self
            .parsed(id, "each")
            .and_then(|p| p.expr.as_ref().ok().and_then(Expr::as_path))
            .and_then(|p| self.resolve(&p, frame));
        let mut deps = frame.deps.clone();
        if let Some(p) = self.parsed(id, "each") {
            deps.extend(self.state_deps(&p.deps, frame));
        }
        if items.len() > self.limits.max_list {
            let d = Diagnostic::new(
                "limit-list",
                Severity::Warning,
                format!("only the first {} items are shown", self.limits.max_list),
            )
            .on(n);
            self.errors.borrow_mut().insert((id, "each".into()), d);
        }
        items
            .iter()
            .take(self.limits.max_list)
            .enumerate()
            .map(|(i, it)| {
                let mut f = Frame {
                    inst: frame.inst.clone(),
                    vars: frame.vars.clone(),
                    deps: deps.clone(),
                };
                f.vars.push(Var {
                    name: item_name.clone(),
                    value: it.clone(),
                    source: source.as_ref().map(|s| {
                        let mut p = s.clone();
                        p.push(i.to_string());
                        p
                    }),
                });
                f.vars.push(Var {
                    name: index_name.clone(),
                    value: Value::Num(i as f64),
                    source: None,
                });
                let key = if n.has("key") {
                    self.eval_in(id, "key", &f).to_text()
                } else {
                    i.to_string()
                };
                let mut h = DefaultHasher::new();
                (id, key).hash(&mut h);
                f.inst.0.push(h.finish());
                f
            })
            .collect()
    }
}
