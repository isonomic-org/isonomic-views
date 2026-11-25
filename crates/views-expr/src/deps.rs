// Copyright 2026 Anud Inc.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Static dependency extraction for the reactive graph.

use std::collections::BTreeSet;

use crate::{Value, parser::Expr};

/// The state paths an expression reads, as dotted strings (`shared.seats`, `local.rows`).
/// Paths are cut at the first dynamic index, so `shared.items[i].done` yields `shared.items`.
/// Names bound by lambdas are excluded.
pub fn deps(e: &Expr) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    walk(e, &mut Vec::new(), &mut out);
    out
}

fn walk(e: &Expr, bound: &mut Vec<String>, out: &mut BTreeSet<String>) {
    match e {
        Expr::Lit(_) => {}
        Expr::Var(_) | Expr::Member(..) | Expr::Index(..) => {
            let (path, rest) = static_prefix(e);
            if let Some(root) = path.first()
                && !bound.contains(root)
            {
                out.insert(path.join("."));
            }
            for r in rest {
                walk(r, bound, out);
            }
        }
        Expr::Call(_, args) | Expr::List(args) => args.iter().for_each(|a| walk(a, bound, out)),
        Expr::Unary(_, x) => walk(x, bound, out),
        Expr::Binary(_, a, b) => {
            walk(a, bound, out);
            walk(b, bound, out);
        }
        Expr::Cond(c, a, b) => {
            walk(c, bound, out);
            walk(a, bound, out);
            walk(b, bound, out);
        }
        Expr::Record(fields) => fields.iter().for_each(|(_, v)| walk(v, bound, out)),
        Expr::Lambda(params, body) => {
            let n = bound.len();
            bound.extend(params.iter().cloned());
            walk(body, bound, out);
            bound.truncate(n);
        }
    }
}

/// The longest static path at the head of `e`, plus sub-expressions that still need walking
/// (dynamic index expressions and anything under them).
fn static_prefix(e: &Expr) -> (Vec<String>, Vec<&Expr>) {
    match e {
        Expr::Var(v) => (vec![v.clone()], vec![]),
        Expr::Member(b, f) => {
            let (mut p, rest) = static_prefix(b);
            if rest.is_empty() && !p.is_empty() {
                p.push(f.clone());
            }
            (p, rest)
        }
        Expr::Index(b, i) => {
            let (mut p, mut rest) = static_prefix(b);
            let lit = match &**i {
                Expr::Lit(Value::Num(n)) if *n >= 0.0 && n.fract() == 0.0 => {
                    Some((*n as usize).to_string())
                }
                Expr::Lit(Value::Str(s)) => Some(s.to_string()),
                _ => None,
            };
            match lit {
                Some(seg) if rest.is_empty() && !p.is_empty() => p.push(seg),
                _ => rest.push(i),
            }
            (p, rest)
        }
        other => (vec![], vec![other]),
    }
}
