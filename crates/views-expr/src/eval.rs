// Copyright 2026 Anud Inc.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Bounded, side-effect-free evaluation.

use std::{
    collections::{BTreeMap, HashMap},
    fmt,
};

use crate::{
    Value,
    parser::{BinOp, Expr, UnOp},
    stdlib,
};

/// Resource limits for one evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Total work units: one per node visited and per list element touched.
    pub max_steps: u64,
    /// Longest list an evaluation may build.
    pub max_list: usize,
    /// Longest text an evaluation may build, in bytes.
    pub max_text: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_steps: 20_000,
            max_list: 1_000,
            max_text: 64 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// The step budget ran out.
    Steps,
    /// A list or text grew past its limit.
    Size,
    /// An operation got a value of the wrong type.
    Type,
    /// A name is not defined.
    Name,
    /// An unknown function, or wrong arguments to a known one.
    Function,
    /// Division by zero and similar.
    Math,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvalError {
    pub kind: ErrorKind,
    pub message: String,
}

impl EvalError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        EvalError {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for EvalError {}

pub type EvalResult = Result<Value, EvalError>;

/// Where free names come from.
pub trait Scope {
    fn lookup(&self, name: &str) -> Option<Value>;
}

impl Scope for BTreeMap<String, Value> {
    fn lookup(&self, name: &str) -> Option<Value> {
        self.get(name).cloned()
    }
}

impl Scope for HashMap<String, Value> {
    fn lookup(&self, name: &str) -> Option<Value> {
        self.get(name).cloned()
    }
}

/// A scope with no names.
pub struct Empty;

impl Scope for Empty {
    fn lookup(&self, _: &str) -> Option<Value> {
        None
    }
}

/// Evaluate `expr` against `scope` within `limits`.
pub fn evaluate(expr: &Expr, scope: &dyn Scope, limits: Limits) -> EvalResult {
    Evaluator::new(scope, limits).eval(expr)
}

pub struct Evaluator<'a> {
    scope: &'a dyn Scope,
    pub(crate) limits: Limits,
    steps: u64,
    locals: Vec<(String, Value)>,
}

impl<'a> Evaluator<'a> {
    pub fn new(scope: &'a dyn Scope, limits: Limits) -> Self {
        Evaluator {
            scope,
            limits,
            steps: 0,
            locals: Vec::new(),
        }
    }

    /// Steps used so far.
    pub fn steps(&self) -> u64 {
        self.steps
    }

    pub(crate) fn tick(&mut self, n: u64) -> Result<(), EvalError> {
        self.steps += n;
        if self.steps > self.limits.max_steps {
            Err(EvalError::new(
                ErrorKind::Steps,
                "evaluation took too many steps",
            ))
        } else {
            Ok(())
        }
    }

    pub(crate) fn check_list(&self, len: usize) -> Result<(), EvalError> {
        if len > self.limits.max_list {
            Err(EvalError::new(
                ErrorKind::Size,
                format!("list longer than {}", self.limits.max_list),
            ))
        } else {
            Ok(())
        }
    }

    pub(crate) fn check_text(&self, len: usize) -> Result<(), EvalError> {
        if len > self.limits.max_text {
            Err(EvalError::new(ErrorKind::Size, "text too long"))
        } else {
            Ok(())
        }
    }

    pub fn eval(&mut self, e: &Expr) -> EvalResult {
        self.tick(1)?;
        match e {
            Expr::Lit(v) => Ok(v.clone()),
            Expr::Var(name) => self.var(name),
            Expr::Member(b, f) => Ok(self.eval(b)?.field(f)),
            Expr::Index(b, i) => {
                let b = self.eval(b)?;
                let i = self.eval(i)?;
                Ok(b.index(&i))
            }
            Expr::Call(name, args) => stdlib::call(self, name, args),
            Expr::Unary(op, x) => {
                let v = self.eval(x)?;
                match op {
                    UnOp::Not => Ok(Value::Bool(!v.truthy())),
                    UnOp::Neg => match v {
                        Value::Num(n) => Ok(Value::Num(-n)),
                        Value::Null => Ok(Value::Null),
                        other => Err(type_err("-", &other)),
                    },
                }
            }
            Expr::Binary(op, a, b) => self.binary(*op, a, b),
            Expr::Cond(c, a, b) => {
                if self.eval(c)?.truthy() {
                    self.eval(a)
                } else {
                    self.eval(b)
                }
            }
            Expr::List(items) => {
                self.check_list(items.len())?;
                let mut out = Vec::with_capacity(items.len());
                for it in items {
                    out.push(self.eval(it)?);
                }
                Ok(Value::list(out))
            }
            Expr::Record(fields) => {
                let mut m = BTreeMap::new();
                for (k, v) in fields {
                    let v = self.eval(v)?;
                    m.insert(k.clone(), v);
                }
                Ok(Value::map(m))
            }
            Expr::Lambda(..) => Err(EvalError::new(
                ErrorKind::Function,
                "a function (x => ...) can only be passed to another function",
            )),
        }
    }

    fn var(&self, name: &str) -> EvalResult {
        if let Some((_, v)) = self.locals.iter().rev().find(|(n, _)| n == name) {
            return Ok(v.clone());
        }
        self.scope
            .lookup(name)
            .ok_or_else(|| EvalError::new(ErrorKind::Name, format!("unknown name '{name}'")))
    }

    /// Call a lambda with `args` bound to its parameters.
    pub(crate) fn apply(&mut self, f: &Expr, args: &[Value]) -> EvalResult {
        let Expr::Lambda(params, body) = f else {
            return Err(EvalError::new(
                ErrorKind::Function,
                "expected a function like x => x",
            ));
        };
        let base = self.locals.len();
        for (i, p) in params.iter().enumerate() {
            self.locals
                .push((p.clone(), args.get(i).cloned().unwrap_or_default()));
        }
        let r = self.eval(body);
        self.locals.truncate(base);
        r
    }

    fn binary(&mut self, op: BinOp, a: &Expr, b: &Expr) -> EvalResult {
        match op {
            BinOp::And => {
                let l = self.eval(a)?;
                return Ok(Value::Bool(l.truthy() && self.eval(b)?.truthy()));
            }
            BinOp::Or => {
                let l = self.eval(a)?;
                return Ok(Value::Bool(l.truthy() || self.eval(b)?.truthy()));
            }
            BinOp::Coalesce => {
                let l = self.eval(a)?;
                return if l.is_null() { self.eval(b) } else { Ok(l) };
            }
            _ => {}
        }
        let l = self.eval(a)?;
        let r = self.eval(b)?;
        self.binary_values(op, l, r)
    }

    fn binary_values(&mut self, op: BinOp, l: Value, r: Value) -> EvalResult {
        use Value::*;
        Ok(match (op, &l, &r) {
            (BinOp::Eq, _, _) => Bool(l == r),
            (BinOp::Ne, _, _) => Bool(l != r),
            (BinOp::Add, Num(x), Num(y)) => Num(x + y),
            (BinOp::Add, Date(d), Num(n)) => Date(d + *n as i64),
            (BinOp::Add, List(x), List(y)) => {
                self.check_list(x.len() + y.len())?;
                self.tick((x.len() + y.len()) as u64)?;
                Value::list(x.iter().chain(y.iter()).cloned().collect())
            }
            (BinOp::Add, Str(_), _) | (BinOp::Add, _, Str(_)) => {
                let s = l.to_text() + &r.to_text();
                self.check_text(s.len())?;
                Value::str(s)
            }
            (BinOp::Sub, Num(x), Num(y)) => Num(x - y),
            (BinOp::Sub, Date(x), Date(y)) => Num((x - y) as f64),
            (BinOp::Sub, Date(d), Num(n)) => Date(d - *n as i64),
            (BinOp::Mul, Num(x), Num(y)) => Num(x * y),
            (BinOp::Div, Num(_), Num(y)) if *y == 0.0 => {
                return Err(EvalError::new(ErrorKind::Math, "division by zero"));
            }
            (BinOp::Div, Num(x), Num(y)) => Num(x / y),
            (BinOp::Rem, Num(_), Num(y)) if *y == 0.0 => {
                return Err(EvalError::new(ErrorKind::Math, "division by zero"));
            }
            (BinOp::Rem, Num(x), Num(y)) => Num(x % y),
            (BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge, _, _) => {
                let ord = compare(&l, &r).ok_or_else(|| {
                    EvalError::new(
                        ErrorKind::Type,
                        format!("cannot compare {} with {}", l.type_name(), r.type_name()),
                    )
                })?;
                Bool(match op {
                    BinOp::Lt => ord.is_lt(),
                    BinOp::Le => ord.is_le(),
                    BinOp::Gt => ord.is_gt(),
                    _ => ord.is_ge(),
                })
            }
            // Arithmetic on a missing value stays missing, so a half-built view shows blanks.
            (_, Null, _) | (_, _, Null) => Null,
            _ => {
                return Err(EvalError::new(
                    ErrorKind::Type,
                    format!(
                        "cannot apply {op:?} to {} and {}",
                        l.type_name(),
                        r.type_name()
                    ),
                ));
            }
        })
    }
}

/// Ordering for numbers, text and dates.
pub fn compare(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Value::Num(x), Value::Num(y)) => x.partial_cmp(y),
        (Value::Str(x), Value::Str(y)) => Some(x.cmp(y)),
        (Value::Date(x), Value::Date(y)) => Some(x.cmp(y)),
        (Value::Bool(x), Value::Bool(y)) => Some(x.cmp(y)),
        _ => None,
    }
}

pub(crate) fn type_err(what: &str, v: &Value) -> EvalError {
    EvalError::new(
        ErrorKind::Type,
        format!("{what} does not accept {}", v.type_name()),
    )
}
