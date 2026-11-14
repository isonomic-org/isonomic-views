//! Checks a view against the catalog. Diagnostics are machine-readable so a model can repair
//! its own output.

use serde::Serialize;
use views_expr::{Expr, FUNCTIONS};
use views_ir::{Diagnostic, Doc, Kind, Limits, Node, Severity};

use crate::{
    Children, Component, Prop, Type, UNIVERSAL, VERSION, classes, closest, component, tokens,
};

/// Result of checking a source document.
#[derive(Clone, Debug, Serialize)]
pub struct Report {
    /// No errors (warnings allowed).
    pub ok: bool,
    pub catalog: &'static str,
    pub diagnostics: Vec<Diagnostic>,
}

/// Parse and validate a complete source.
pub fn check(src: &str) -> Report {
    let doc = Doc::parse(src, &Limits::default());
    let diagnostics = validate(&doc);
    Report {
        ok: !diagnostics.iter().any(|d| d.severity == Severity::Error),
        catalog: VERSION,
        diagnostics,
    }
}

/// All diagnostics for a document: parser and limit problems first, then catalog checks.
pub fn validate(doc: &Doc) -> Vec<Diagnostic> {
    let mut out = doc.diagnostics.clone();
    let Some(root) = doc.root_node() else {
        out.push(
            Diagnostic::new("empty", Severity::Error, "the view is empty")
                .hint("start with <View version=\"0.1\">"),
        );
        return out;
    };
    if !root.is("View") {
        out.push(
            Diagnostic::new(
                "root-not-view",
                Severity::Error,
                format!("the root is <{}>, not <View>", root.tag),
            )
            .on(root)
            .hint("wrap everything in <View version=\"0.1\">...</View>"),
        );
    } else {
        match root.prop("version") {
            None => out.push(
                Diagnostic::new(
                    "missing-version",
                    Severity::Warning,
                    "the view does not declare a catalog version",
                )
                .on(root)
                .hint(format!("add version=\"{VERSION}\" to <View>")),
            ),
            Some(v) if v != VERSION => out.push(
                Diagnostic::new(
                    "unsupported-version",
                    Severity::Error,
                    format!("catalog version {v} is not supported; this host supports {VERSION}"),
                )
                .on(root),
            ),
            _ => {}
        }
    }
    for id in doc.preorder() {
        let n = &doc.nodes[&id];
        if n.kind != Kind::Element {
            check_inline(doc, n, &mut out);
            continue;
        }
        let Some(c) = component(&n.tag) else {
            let mut d = Diagnostic::new(
                "unknown-component",
                Severity::Error,
                format!("<{}> is not in the catalog", n.tag),
            )
            .on(n);
            if let Some(s) = closest(&n.tag, crate::COMPONENTS.iter().map(|c| c.tag)) {
                d = d.hint(format!("did you mean <{s}>?"));
            }
            out.push(d);
            continue;
        };
        if n.id != root.id && c.tag == "View" {
            out.push(
                Diagnostic::new(
                    "nested-view",
                    Severity::Error,
                    "<View> may only be the root",
                )
                .on(n),
            );
        }
        let parent_tag = n
            .parent
            .and_then(|p| doc.nodes.get(&p))
            .map(|p| p.tag.as_str());
        if let Some(want) = c.parent
            && parent_tag != Some(want)
        {
            out.push(
                Diagnostic::new(
                    "misplaced",
                    Severity::Error,
                    format!("<{}> must be directly inside <{want}>", c.tag),
                )
                .on(n),
            );
        }
        check_children(doc, n, c, &mut out);
        let vars = scope_vars(doc, n);
        for prop in c.props.iter().filter(|p| p.required) {
            if !n.has(prop.name) {
                out.push(
                    Diagnostic::new(
                        "missing-prop",
                        Severity::Error,
                        format!("<{}> needs {}", c.tag, prop.name),
                    )
                    .on(n)
                    .hint(format!("{}: {}", prop.name, prop.doc)),
                );
            }
        }
        for (name, value) in n.props.iter() {
            match c.prop(name) {
                Some(p) => check_literal(n, p, value, &mut out),
                None => unknown_prop(n, c, name, &mut out),
            }
        }
        let mut key_vars = vars.clone();
        if n.is("For") {
            key_vars.push(n.prop("as").unwrap_or("item").to_string());
            key_vars.push(n.prop("index").unwrap_or("index").to_string());
        }
        for (name, src) in n.bindings.iter() {
            match c.prop(name) {
                Some(p) => check_binding(
                    n,
                    p,
                    src,
                    if name == "key" { &key_vars } else { &vars },
                    &mut out,
                ),
                None => unknown_prop(n, c, name, &mut out),
            }
        }
    }
    out
}

fn check_inline(doc: &Doc, n: &Node, out: &mut Vec<Diagnostic>) {
    if n.kind == Kind::Expr {
        let vars = scope_vars(doc, n);
        check_expr(
            n,
            "text",
            n.binding("value").unwrap_or(""),
            &vars,
            false,
            out,
        );
    }
    let Some(parent) = n.parent.and_then(|p| doc.nodes.get(&p)) else {
        return;
    };
    if let Some(c) = component(&parent.tag)
        && matches!(c.children, Children::Any)
        && !matches!(c.tag, "If" | "For" | "Tab" | "Section" | "Tooltip")
    {
        out.push(
            Diagnostic::new(
                "bare-text",
                Severity::Warning,
                format!("text directly inside <{}>", c.tag),
            )
            .on(n)
            .hint("wrap it in <Text>"),
        );
    }
}

fn check_children(doc: &Doc, n: &Node, c: &Component, out: &mut Vec<Diagnostic>) {
    for k in n.children.iter().filter_map(|k| doc.nodes.get(k)) {
        let bad = match c.children {
            Children::None => Some(format!("<{}> takes no children", c.tag)),
            Children::Text if k.kind == Kind::Element => Some(format!(
                "<{}> takes only text; <{}> is ignored",
                c.tag, k.tag
            )),
            Children::Only(tags) if k.kind != Kind::Element || !tags.contains(&k.tag.as_str()) => {
                Some(format!(
                    "<{}> takes only {}",
                    c.tag,
                    tags.iter()
                        .map(|t| format!("<{t}>"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            }
            _ => None,
        };
        if let Some(msg) = bad {
            out.push(Diagnostic::new("invalid-child", Severity::Error, msg).on(k));
        }
    }
}

fn unknown_prop(n: &Node, c: &Component, name: &str, out: &mut Vec<Diagnostic>) {
    let names = c.props.iter().chain(UNIVERSAL.iter()).map(|p| p.name);
    let mut d = Diagnostic::new(
        "unknown-prop",
        Severity::Warning,
        format!("<{}> has no prop '{name}'; it is ignored", c.tag),
    )
    .on(n);
    if let Some(s) = closest(name, names) {
        d = d.hint(format!("did you mean '{s}'?"));
    }
    out.push(d);
}

fn check_literal(n: &Node, p: &Prop, v: &str, out: &mut Vec<Diagnostic>) {
    let bad = |out: &mut Vec<Diagnostic>, msg: String, hint: String| {
        out.push(
            Diagnostic::new("bad-value", Severity::Error, format!("{}: {msg}", p.name))
                .on(n)
                .hint(hint),
        );
    };
    match p.ty {
        Type::Number if v.trim().parse::<f64>().is_err() => bad(
            out,
            format!("'{v}' is not a number"),
            format!("write {}={{{v}}} or a number", p.name),
        ),
        Type::Bool if v != "true" && v != "false" => bad(
            out,
            format!("'{v}' is not true or false"),
            format!("write {} alone for true", p.name),
        ),
        Type::Enum(opts) if !opts.contains(&v) => bad(
            out,
            format!("'{v}' is not allowed"),
            format!("use one of: {}", opts.join(", ")),
        ),
        Type::Tone if !tokens::TONES.contains(&v) => bad(
            out,
            format!("'{v}' is not a tone"),
            format!("use one of: {}", tokens::TONES.join(", ")),
        ),
        Type::Color if !tokens::PALETTE.contains(&v) => bad(
            out,
            format!("'{v}' is not a palette color; raw colors are not allowed"),
            format!("use one of: {}", tokens::PALETTE.join(", ")),
        ),
        Type::Icon if !tokens::ICONS.contains(&v) => {
            let hint = closest(v, tokens::ICONS.iter().copied())
                .map_or("see the icon list".into(), |s| {
                    format!("did you mean '{s}'?")
                });
            bad(out, format!("'{v}' is not a known icon"), hint)
        }
        Type::Date if views_expr::parse_date(v).is_none() => bad(
            out,
            format!("'{v}' is not a date"),
            "write YYYY-MM-DD".into(),
        ),
        Type::Class => {
            let (_, words) = classes::parse_class(v);
            for w in words {
                out.push(Diagnostic::new("bad-class", Severity::Error, format!("'{w}' is not an allowed utility; arbitrary values are rejected")).on(n).hint("use token utilities such as gap-3 p-4 w-full text-muted (see Class utilities)"));
            }
        }
        Type::Asset
            if v.contains("://")
                || v.starts_with('/')
                || v.starts_with("data:")
                || v.starts_with("www.") =>
        {
            out.push(
                Diagnostic::new(
                    "url-not-allowed",
                    Severity::Error,
                    format!("{}: a view cannot name a URL or path", p.name),
                )
                .on(n)
                .hint("use a name the host provides, or leave it out"),
            );
        }
        Type::Action => bad(
            out,
            "expects actions in braces".into(),
            format!("{}={{set(local.x, 1)}}", p.name),
        ),
        Type::Bind(_) => bad(
            out,
            "expects a state path in braces".into(),
            format!("{}={{local.value}}", p.name),
        ),
        Type::List | Type::Record => bad(
            out,
            format!("expects a {} in braces", p.ty.describe()),
            format!("{}={{...}}", p.name),
        ),
        _ => {}
    }
}

fn check_binding(n: &Node, p: &Prop, src: &str, vars: &[String], out: &mut Vec<Diagnostic>) {
    if p.literal_only {
        out.push(
            Diagnostic::new(
                "literal-required",
                Severity::Error,
                format!("{} must be a literal, like {}=\"...\"", p.name, p.name),
            )
            .on(n),
        );
        return;
    }
    match p.ty {
        Type::Action => check_actions(n, p.name, src, vars, out),
        Type::Bind(_) => {
            if check_expr(n, p.name, src, vars, false, out)
                && let Ok(e) = views_expr::parse(src)
            {
                let root = e.as_path().and_then(|p| p.first().cloned());
                if !root.is_some_and(|r| r == "local" || r == "shared" || vars.contains(&r)) {
                    out.push(
                        Diagnostic::new(
                            "bad-bind",
                            Severity::Error,
                            format!(
                                "{} must name writable state, like local.x or shared.x",
                                p.name
                            ),
                        )
                        .on(n)
                        .hint("data is read-only; computed values cannot be bound"),
                    );
                }
            }
        }
        _ => {
            check_expr(n, p.name, src, vars, false, out);
        }
    }
}

const ACTION_NAMES: &[&str] = &[
    "set", "toggle", "append", "remove", "reset", "query", "call", "open",
];

fn check_actions(n: &Node, prop: &str, src: &str, vars: &[String], out: &mut Vec<Diagnostic>) {
    let seq = match views_expr::parse_seq(src) {
        Ok(s) => s,
        Err(e) => {
            out.push(
                Diagnostic::new(
                    "expr-syntax",
                    Severity::Error,
                    format!("{prop}: {}", e.message),
                )
                .on(n),
            );
            return;
        }
    };
    for call in &seq {
        let Expr::Call(name, args) = call else {
            out.push(
                Diagnostic::new(
                    "bad-action",
                    Severity::Error,
                    format!("{prop}: each action must be a call like set(local.x, 1)"),
                )
                .on(n),
            );
            continue;
        };
        if !ACTION_NAMES.contains(&name.as_str()) {
            let mut d = Diagnostic::new(
                "bad-action",
                Severity::Error,
                format!("{prop}: '{name}' is not an action"),
            )
            .on(n);
            if let Some(s) = closest(name, ACTION_NAMES.iter().copied()) {
                d = d.hint(format!("did you mean {s}()?"));
            }
            out.push(d);
            continue;
        }
        let path_arg = match (name.as_str(), args.len()) {
            ("set" | "toggle" | "append" | "remove" | "reset", n) if n > 0 => Some(&args[0]),
            ("query", 2) => Some(&args[1]),
            ("query" | "call", 3) => Some(&args[2]),
            _ => None,
        };
        if let Some(a) = path_arg {
            let root = a.as_path().and_then(|p| p.first().cloned());
            if !root.is_some_and(|r| r == "local" || r == "shared" || vars.contains(&r)) {
                out.push(
                    Diagnostic::new(
                        "bad-action",
                        Severity::Error,
                        format!("{prop}: {name}() needs a local.* or shared.* path"),
                    )
                    .on(n),
                );
            }
        }
        for a in args {
            names_in(n, prop, a, vars, true, out);
        }
    }
}

/// Parse an expression and check the names and functions it uses. True if it parsed.
fn check_expr(
    n: &Node,
    prop: &str,
    src: &str,
    vars: &[String],
    event: bool,
    out: &mut Vec<Diagnostic>,
) -> bool {
    match views_expr::parse(src) {
        Ok(e) => {
            names_in(n, prop, &e, vars, event, out);
            true
        }
        Err(e) => {
            out.push(
                Diagnostic::new(
                    "expr-syntax",
                    Severity::Error,
                    format!("{prop}: {}", e.message),
                )
                .on(n)
                .hint("expressions use JavaScript-like syntax inside { }"),
            );
            false
        }
    }
}

fn names_in(
    n: &Node,
    prop: &str,
    e: &Expr,
    vars: &[String],
    event: bool,
    out: &mut Vec<Diagnostic>,
) {
    let mut bound: Vec<String> = vec![];
    walk(e, &mut bound, &mut |kind, name, bound| {
        let known = match kind {
            Use::Var => {
                ["local", "shared", "data"].contains(&name)
                    || vars.iter().any(|v| v == name)
                    || bound.iter().any(|b| b == name)
                    || (event && name == "event")
            }
            Use::Call => FUNCTIONS.iter().any(|(f, _, _)| *f == name),
        };
        if !known {
            let (code, what, hint) = match kind {
                Use::Var => (
                    "unknown-name",
                    "name",
                    closest(
                        name,
                        ["local", "shared", "data"]
                            .into_iter()
                            .chain(vars.iter().map(String::as_str)),
                    ),
                ),
                Use::Call => (
                    "unknown-function",
                    "function",
                    closest(name, FUNCTIONS.iter().map(|f| f.0)),
                ),
            };
            let mut d = Diagnostic::new(
                code,
                Severity::Error,
                format!("{prop}: unknown {what} '{name}'"),
            )
            .on(n);
            if let Some(h) = hint {
                d = d.hint(format!("did you mean '{h}'?"));
            }
            out.push(d);
        }
    });
}

enum Use {
    Var,
    Call,
}

fn walk(e: &Expr, bound: &mut Vec<String>, f: &mut dyn FnMut(Use, &str, &[String])) {
    match e {
        Expr::Lit(_) => {}
        Expr::Var(v) => f(Use::Var, v, bound),
        Expr::Member(b, _) => walk(b, bound, f),
        Expr::Index(b, i) => {
            walk(b, bound, f);
            walk(i, bound, f);
        }
        Expr::Call(name, args) => {
            f(Use::Call, name, bound);
            args.iter().for_each(|a| walk(a, bound, f));
        }
        Expr::Unary(_, x) => walk(x, bound, f),
        Expr::Binary(_, a, b) => {
            walk(a, bound, f);
            walk(b, bound, f);
        }
        Expr::Cond(c, a, b) => {
            walk(c, bound, f);
            walk(a, bound, f);
            walk(b, bound, f);
        }
        Expr::List(items) => items.iter().for_each(|i| walk(i, bound, f)),
        Expr::Record(fields) => fields.iter().for_each(|(_, v)| walk(v, bound, f)),
        Expr::Lambda(params, body) => {
            let k = bound.len();
            bound.extend(params.iter().cloned());
            walk(body, bound, f);
            bound.truncate(k);
        }
    }
}

/// Loop variables visible at a node: `as` and `index` names of enclosing `For`s.
fn scope_vars(doc: &Doc, n: &Node) -> Vec<String> {
    doc.ancestors(n.id)
        .filter(|a| a.is("For"))
        .flat_map(|f| {
            [
                f.prop("as").unwrap_or("item").to_string(),
                f.prop("index").unwrap_or("index").to_string(),
            ]
        })
        .collect()
}
