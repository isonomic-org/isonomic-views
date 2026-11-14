//! State scopes, bindings and the reactive cache.

use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet, HashMap},
    hash::Hash,
    rc::Rc,
};

use views_expr::{self as expr, Expr, Scope, Value};
use views_ir::{Diagnostic, Doc, Limits, NodeId, Severity};

use crate::{Host, SharedHandle};

mod text;

/// Identifies one rendered instance of a node: empty outside `For`, one entry per
/// enclosing `For` item otherwise.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Inst(pub Vec<u64>);

#[derive(Clone, Debug)]
struct Var {
    name: String,
    value: Value,
    /// The state path this variable stands for, when it is an item of a stored list.
    source: Option<Vec<String>>,
}

/// Loop variables in scope at a point of the tree.
#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub inst: Inst,
    vars: Vec<Var>,
    deps: BTreeSet<String>,
}

impl Frame {
    pub fn root() -> Frame {
        Frame::default()
    }

    pub fn var(&self, name: &str) -> Option<&Value> {
        self.vars
            .iter()
            .rev()
            .find(|v| v.name == name)
            .map(|v| &v.value)
    }
}

pub(crate) struct Parsed {
    src: String,
    pub(crate) expr: Result<Expr, expr::ParseError>,
    deps: BTreeSet<String>,
}

struct Cached {
    src: String,
    value: Value,
    deps: BTreeSet<String>,
}

type Key = (NodeId, String, Inst);

/// One view's live state. Shared viewers of the same view use separate runtimes over the
/// same store.
pub struct Runtime {
    pub(crate) doc: Doc,
    pub(crate) host: Rc<dyn Host>,
    pub(crate) store: SharedHandle,
    pub(crate) limits: Limits,
    pub(crate) local: RefCell<Value>,
    pub(crate) local_init: RefCell<Value>,
    pub(crate) shared_init: RefCell<Value>,
    data: RefCell<BTreeMap<String, Value>>,
    parsed: RefCell<HashMap<(NodeId, String), Rc<Parsed>>>,
    cache: RefCell<HashMap<Key, Cached>>,
    seen_version: Cell<u64>,
    evals: Cell<u64>,
    pub(crate) errors: RefCell<BTreeMap<(NodeId, String), Diagnostic>>,
}

pub(crate) const STATE_ROOTS: [&str; 3] = ["local", "shared", "data"];

impl Runtime {
    pub fn new(doc: Doc, host: Rc<dyn Host>, store: SharedHandle, limits: Limits) -> Runtime {
        let rt = Runtime {
            doc: Doc::default(),
            host,
            store,
            limits,
            local: RefCell::new(Value::empty_map()),
            local_init: RefCell::new(Value::empty_map()),
            shared_init: RefCell::new(Value::empty_map()),
            data: RefCell::new(BTreeMap::new()),
            parsed: RefCell::new(HashMap::new()),
            cache: RefCell::new(HashMap::new()),
            seen_version: Cell::new(0),
            evals: Cell::new(0),
            errors: RefCell::new(BTreeMap::new()),
        };
        let mut rt = rt;
        rt.set_doc(doc);
        rt
    }

    /// Replace the document (after streaming or an edit). State is kept; new defaults are
    /// added; cached values whose source changed are recomputed on next use.
    pub fn set_doc(&mut self, doc: Doc) {
        self.doc = doc;
        self.errors
            .borrow_mut()
            .retain(|(id, _), _| self.doc.nodes.contains_key(id));
        let removed: Vec<NodeId> = self
            .parsed
            .borrow()
            .keys()
            .map(|k| k.0)
            .filter(|id| !self.doc.nodes.contains_key(id))
            .collect();
        if !removed.is_empty() {
            self.parsed
                .borrow_mut()
                .retain(|k, _| !removed.contains(&k.0));
            self.cache
                .borrow_mut()
                .retain(|k, _| !removed.contains(&k.0));
        }
        if let Some(root) = self.doc.root {
            let li = self.eval_root_init(root, "local");
            if li != *self.local_init.borrow() {
                if let Value::Map(m) = &li {
                    let mut local = self.local.borrow_mut();
                    for (k, v) in m.iter() {
                        if local.field(k).is_null() {
                            *local = local.with_path(&[k.as_str()], v.clone());
                        }
                    }
                }
                *self.local_init.borrow_mut() = li;
                self.invalidate("local");
            }
            let si = self.eval_root_init(root, "shared");
            if si != *self.shared_init.borrow() {
                self.store.borrow_mut().init(&si);
                *self.shared_init.borrow_mut() = si;
            }
        }
        self.fetch_data();
        self.sync();
    }

    fn eval_root_init(&self, root: NodeId, prop: &str) -> Value {
        match self.doc.nodes[&root].binding(prop) {
            Some(_) => match self.eval(root, prop, &Frame::root()) {
                v @ Value::Map(_) => v,
                _ => Value::empty_map(),
            },
            None => Value::empty_map(),
        }
    }

    /// Ask the host for every `data.<name>` the view reads that is not loaded yet.
    fn fetch_data(&self) {
        let mut names = BTreeSet::new();
        for n in self.doc.nodes.values() {
            for prop in n.bindings.keys() {
                for d in &self
                    .parsed(n.id, prop)
                    .map_or_else(BTreeSet::new, |p| p.deps.clone())
                {
                    if let Some(rest) = d.strip_prefix("data.") {
                        names.insert(rest.split('.').next().unwrap_or(rest).to_string());
                    }
                }
            }
        }
        for name in names {
            if self.data.borrow().contains_key(&name) {
                continue;
            }
            let v = match self.host.data(&name) {
                Ok(v) => v,
                Err(e) => {
                    let d = Diagnostic::new("host-denied", Severity::Warning, e.to_string());
                    self.errors
                        .borrow_mut()
                        .insert((0, format!("data.{name}")), d);
                    Value::Null
                }
            };
            self.data.borrow_mut().insert(name, v);
            self.invalidate("data");
        }
    }

    pub fn doc(&self) -> &Doc {
        &self.doc
    }

    pub fn limits(&self) -> &Limits {
        &self.limits
    }

    pub fn host(&self) -> &Rc<dyn Host> {
        &self.host
    }

    /// Pick up changes other viewers made to shared state. True if anything changed.
    pub fn sync(&self) -> bool {
        let v = self.store.borrow().version();
        if v != self.seen_version.get() {
            self.seen_version.set(v);
            self.invalidate("shared");
            true
        } else {
            false
        }
    }

    pub fn local(&self) -> Value {
        self.local.borrow().clone()
    }

    pub fn shared(&self) -> Value {
        self.store.borrow().get()
    }

    /// Number of expression evaluations so far (cache misses). For tests and the report.
    pub fn evals(&self) -> u64 {
        self.evals.get()
    }

    /// Runtime problems: expression errors and denied host requests.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.errors.borrow().values().cloned().collect()
    }

    pub(crate) fn parsed(&self, id: NodeId, prop: &str) -> Option<Rc<Parsed>> {
        let src = self.doc.nodes.get(&id)?.bindings.get(prop)?;
        let key = (id, prop.to_string());
        if let Some(p) = self.parsed.borrow().get(&key)
            && p.src == *src
        {
            return Some(p.clone());
        }
        let e = if prop.starts_with("on") {
            expr::parse_seq(src).map(Expr::List)
        } else {
            expr::parse(src)
        };
        let deps = e.as_ref().map(expr::deps).unwrap_or_default();
        let p = Rc::new(Parsed {
            src: src.clone(),
            expr: e,
            deps,
        });
        self.parsed.borrow_mut().insert(key, p.clone());
        Some(p)
    }

    /// Is the prop present, as a literal or a binding?
    pub fn has(&self, id: NodeId, prop: &str) -> bool {
        self.doc.nodes.get(&id).is_some_and(|n| n.has(prop))
    }

    /// The value of a prop: literal text as a text value, a binding evaluated. Missing → null.
    pub fn eval(&self, id: NodeId, prop: &str, frame: &Frame) -> Value {
        let Some(node) = self.doc.nodes.get(&id) else {
            return Value::Null;
        };
        if let Some(lit) = node.props.get(prop) {
            return Value::str(lit);
        }
        let Some(p) = self.parsed(id, prop) else {
            return Value::Null;
        };
        let key: Key = (id, prop.to_string(), frame.inst.clone());
        if let Some(c) = self.cache.borrow().get(&key)
            && c.src == p.src
        {
            return c.value.clone();
        }
        let err_key = (id, prop.to_string());
        let value = match &p.expr {
            Err(e) if e.incomplete => Value::Null,
            Err(e) => {
                let d = Diagnostic::new(
                    "expr-syntax",
                    Severity::Error,
                    format!("{prop}: {}", e.message),
                )
                .on(node);
                self.errors.borrow_mut().insert(err_key, d);
                Value::Null
            }
            Ok(e) => {
                self.evals.set(self.evals.get() + 1);
                match self.run(e, frame, None) {
                    Ok(v) => {
                        self.errors.borrow_mut().remove(&err_key);
                        v
                    }
                    Err(msg) => {
                        let d =
                            Diagnostic::new("expr-eval", Severity::Error, format!("{prop}: {msg}"))
                                .on(node);
                        self.errors.borrow_mut().insert(err_key, d);
                        Value::Null
                    }
                }
            }
        };
        let mut deps = self.state_deps(&p.deps, frame);
        deps.extend(frame.deps.iter().cloned());
        let mut cache = self.cache.borrow_mut();
        if cache.len() > 50_000 {
            cache.clear();
        }
        cache.insert(
            key,
            Cached {
                src: p.src.clone(),
                value: value.clone(),
                deps,
            },
        );
        value
    }

    pub(crate) fn run(
        &self,
        e: &Expr,
        frame: &Frame,
        event: Option<&Value>,
    ) -> Result<Value, String> {
        let env = Env {
            rt: self,
            frame,
            event,
        };
        let lim = expr::Limits {
            max_steps: self.limits.max_expr_steps,
            max_list: self.limits.max_list,
            max_text: self.limits.max_text,
        };
        expr::evaluate(e, &env, lim).map_err(|e| e.to_string())
    }

    /// Map expression dependencies to state paths, through loop variables when they stand
    /// for stored items.
    fn state_deps(&self, deps: &BTreeSet<String>, frame: &Frame) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for d in deps {
            let segs: Vec<&str> = d.split('.').collect();
            if STATE_ROOTS.contains(&segs[0]) {
                out.insert(d.clone());
            } else if let Some(var) = frame.vars.iter().rev().find(|v| v.name == segs[0])
                && let Some(src) = &var.source
            {
                let mut p = src.clone();
                p.extend(segs[1..].iter().map(|s| s.to_string()));
                out.insert(p.join("."));
            }
        }
        out
    }

    /// Drop cached values that depend on `path` (or on anything under or above it).
    pub(crate) fn invalidate(&self, path: &str) {
        self.cache
            .borrow_mut()
            .retain(|_, c| !c.deps.iter().any(|d| overlaps(d, path)));
    }

    /// Evaluate a prop in a frame without caching (used for per-item keys).
    fn eval_in(&self, id: NodeId, prop: &str, frame: &Frame) -> Value {
        match self.parsed(id, prop).as_deref().map(|p| &p.expr) {
            Some(Ok(e)) => self.run(e, frame, None).unwrap_or_default(),
            _ => self
                .doc
                .nodes
                .get(&id)
                .and_then(|n| n.prop(prop))
                .map(Value::str)
                .unwrap_or_default(),
        }
    }

    /// Turn a path written in the view into a state path: loop variables are replaced by the
    /// stored item they stand for. None if the path does not name writable state.
    pub(crate) fn resolve(&self, path: &[String], frame: &Frame) -> Option<Vec<String>> {
        let root = path.first()?;
        if root == "local" || root == "shared" || root == "data" {
            return Some(path.to_vec());
        }
        let var = frame.vars.iter().rev().find(|v| &v.name == root)?;
        let mut p = var.source.clone()?;
        p.extend(path[1..].iter().cloned());
        Some(p)
    }

    /// Read state at a path (`["shared", "seats"]`).
    pub fn get(&self, path: &[String]) -> Value {
        match path.first().map(String::as_str) {
            Some("local") => self.local.borrow().at_path(&path[1..]),
            Some("shared") => self.store.borrow().get().at_path(&path[1..]),
            Some("data") => Value::map(self.data.borrow().clone()).at_path(&path[1..]),
            _ => Value::Null,
        }
    }

    /// Write state at a path. Only `local` and `shared` are writable.
    pub fn set(&self, path: &[String], value: Value) -> Result<(), String> {
        let list_len = value.as_list().map_or(0, <[Value]>::len);
        if list_len > self.limits.max_list {
            return Err(format!(
                "a list may hold at most {} items",
                self.limits.max_list
            ));
        }
        match path.first().map(String::as_str) {
            Some("local") => {
                let next = self.local.borrow().with_path(&path[1..], value);
                *self.local.borrow_mut() = next;
            }
            Some("shared") => {
                self.store.borrow_mut().set(&path[1..], value);
                self.seen_version.set(self.store.borrow().version());
            }
            _ => {
                return Err(format!(
                    "'{}' is not writable state; use local.* or shared.*",
                    path.join(".")
                ));
            }
        }
        self.invalidate(&path.join("."));
        Ok(())
    }

    /// Two-way binding: write a new value through a node's `bind` prop.
    pub fn write_bind(&self, id: NodeId, frame: &Frame, value: Value) -> Result<(), String> {
        let p = self.parsed(id, "bind").ok_or("this node has no bind")?;
        let e = p.expr.as_ref().map_err(|e| e.message.clone())?;
        let path = e
            .as_path()
            .ok_or("bind must name state, like shared.count")?;
        let path = self
            .resolve(&path, frame)
            .ok_or("bind must name local.* or shared.* state")?;
        self.set(&path, value)
    }
}

/// Paths overlap when one is a segment prefix of the other.
fn overlaps(a: &str, b: &str) -> bool {
    let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    long.starts_with(short) && (long.len() == short.len() || long.as_bytes()[short.len()] == b'.')
}

struct Env<'a> {
    rt: &'a Runtime,
    frame: &'a Frame,
    event: Option<&'a Value>,
}

impl Scope for Env<'_> {
    fn lookup(&self, name: &str) -> Option<Value> {
        if let Some(v) = self.frame.var(name) {
            return Some(v.clone());
        }
        match name {
            "local" => Some(self.rt.local.borrow().clone()),
            "shared" => Some(self.rt.store.borrow().get()),
            "data" => Some(Value::map(self.rt.data.borrow().clone())),
            "event" => self.event.cloned(),
            _ => None,
        }
    }
}
