//! Declarative actions: set, toggle, append, remove, reset, and requests to the host.

use views_expr::{Expr, Value};
use views_ir::{Diagnostic, NodeId, Severity};

use crate::{Frame, Runtime};

/// What an action did. Returned for logging and tests.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Set {
        path: String,
        value: Value,
    },
    Host {
        request: String,
        ok: bool,
        message: Option<String>,
    },
    Error(String),
}

/// Names a view may use in an event handler, with their signature.
pub const ACTIONS: &[(&str, &str, &str)] = &[
    ("set", "set(path, value)", "Store a value."),
    ("toggle", "toggle(path)", "Flip a true/false value."),
    (
        "append",
        "append(path, item)",
        "Add an item to the end of a list.",
    ),
    (
        "remove",
        "remove(path, index | item => condition)",
        "Remove one item by position, or every item matching a condition.",
    ),
    (
        "reset",
        "reset(path?)",
        "Restore the starting value; with no path, all local state.",
    ),
    (
        "query",
        "query(name, args?, into)",
        "Ask the host to run a named query and store the result.",
    ),
    (
        "call",
        "call(name, args?, into?)",
        "Ask the host to run a named function, optionally storing the result.",
    ),
    (
        "open",
        "open(target)",
        "Ask the host to open something (a document, a record, a link).",
    ),
];

impl Runtime {
    /// Run the action list in event prop `prop` (`onClick`, `onChange`, ...) of `id`.
    /// `event` is available to the actions as `event`.
    pub fn dispatch(&self, id: NodeId, prop: &str, frame: &Frame, event: Value) -> Vec<Effect> {
        let Some(p) = self.parsed(id, prop) else {
            return vec![];
        };
        let calls = match &p.expr {
            Ok(Expr::List(calls)) => calls.clone(),
            Ok(_) => vec![],
            Err(e) => return vec![Effect::Error(e.message.clone())],
        };
        let mut out = Vec::new();
        for c in &calls {
            let eff = self.action(c, frame, &event);
            if let Effect::Error(msg)
            | Effect::Host {
                ok: false,
                message: Some(msg),
                ..
            } = &eff
            {
                let mut d = Diagnostic::new("action", Severity::Warning, msg.clone());
                if let Some(n) = self.doc.nodes.get(&id) {
                    d = d.on(n);
                }
                self.errors.borrow_mut().insert((id, prop.to_string()), d);
            }
            out.push(eff);
        }
        out
    }

    fn action(&self, call: &Expr, frame: &Frame, event: &Value) -> Effect {
        let Expr::Call(name, args) = call else {
            return Effect::Error("an action must be a call like set(local.x, 1)".into());
        };
        match self.action_inner(name, args, frame, event) {
            Ok(e) => e,
            Err(msg) => Effect::Error(format!("{name}: {msg}")),
        }
    }

    fn action_inner(
        &self,
        name: &str,
        args: &[Expr],
        frame: &Frame,
        event: &Value,
    ) -> Result<Effect, String> {
        let val = |i: usize| -> Result<Value, String> {
            args.get(i)
                .ok_or(format!("{name} needs argument {}", i + 1))
                .and_then(|e| self.run(e, frame, Some(event)))
        };
        let path = |i: usize| -> Result<Vec<String>, String> {
            let e = args.get(i).ok_or(format!("{name} needs a path"))?;
            let p = e
                .as_path()
                .ok_or("expected a state path like local.count")?;
            let p = self
                .resolve(&p, frame)
                .ok_or("expected local.* or shared.* state")?;
            if p[0] == "data" {
                return Err("data is read-only".into());
            }
            Ok(p)
        };
        let set = |p: Vec<String>, v: Value| -> Result<Effect, String> {
            self.set(&p, v.clone())?;
            Ok(Effect::Set {
                path: p.join("."),
                value: v,
            })
        };
        match name {
            "set" => set(path(0)?, val(1)?),
            "toggle" => {
                let p = path(0)?;
                let v = !self.get(&p).truthy();
                set(p, Value::Bool(v))
            }
            "append" => {
                let p = path(0)?;
                let mut l = self
                    .get(&p)
                    .as_list()
                    .map(<[Value]>::to_vec)
                    .unwrap_or_default();
                l.push(val(1)?);
                set(p, Value::list(l))
            }
            "remove" => {
                let p = path(0)?;
                let l = self
                    .get(&p)
                    .as_list()
                    .map(<[Value]>::to_vec)
                    .unwrap_or_default();
                let kept: Vec<Value> = match args.get(1) {
                    Some(f @ Expr::Lambda(..)) => {
                        let mut kept = Vec::new();
                        for it in l {
                            let wrap = Expr::Call(
                                "any".into(),
                                vec![Expr::List(vec![Expr::Lit(it.clone())]), f.clone()],
                            );
                            if !self.run(&wrap, frame, Some(event))?.truthy() {
                                kept.push(it);
                            }
                        }
                        kept
                    }
                    Some(_) => match val(1)? {
                        Value::Num(i) if i >= 0.0 => l
                            .into_iter()
                            .enumerate()
                            .filter(|(k, _)| *k as f64 != i.floor())
                            .map(|(_, v)| v)
                            .collect(),
                        other => l.into_iter().filter(|v| *v != other).collect(),
                    },
                    None => return Err("remove needs an index or a condition".into()),
                };
                set(p, Value::list(kept))
            }
            "reset" => {
                if args.is_empty() {
                    let init = self.local_init.borrow().clone();
                    *self.local.borrow_mut() = init.clone();
                    self.invalidate("local");
                    return Ok(Effect::Set {
                        path: "local".into(),
                        value: init,
                    });
                }
                let p = path(0)?;
                let init = if p[0] == "local" {
                    self.local_init.borrow().at_path(&p[1..])
                } else {
                    self.shared_init.borrow().at_path(&p[1..])
                };
                set(p, init)
            }
            "query" | "call" => {
                let req = val(0)?.to_text();
                let (call_args, into) = match (name, args.len()) {
                    ("query", 2) => (Value::Null, Some(path(1)?)),
                    ("query", 3) | ("call", 3) => (val(1)?, Some(path(2)?)),
                    ("call", 1) => (Value::Null, None),
                    ("call", 2) => (val(1)?, None),
                    _ => return Err("wrong number of arguments".into()),
                };
                let r = if name == "query" {
                    self.host.query(&req, &call_args)
                } else {
                    self.host.call(&req, &call_args)
                };
                match r {
                    Ok(v) => {
                        if let Some(p) = into {
                            self.set(&p, v)?;
                        }
                        Ok(Effect::Host {
                            request: format!("{name} {req}"),
                            ok: true,
                            message: None,
                        })
                    }
                    Err(e) => Ok(Effect::Host {
                        request: format!("{name} {req}"),
                        ok: false,
                        message: Some(e.to_string()),
                    }),
                }
            }
            "open" => {
                let target = val(0)?.to_text();
                match self.host.open(&target) {
                    Ok(()) => Ok(Effect::Host {
                        request: format!("open {target}"),
                        ok: true,
                        message: None,
                    }),
                    Err(e) => Ok(Effect::Host {
                        request: format!("open {target}"),
                        ok: false,
                        message: Some(e.to_string()),
                    }),
                }
            }
            other => Err(format!("unknown action '{other}'")),
        }
    }
}
