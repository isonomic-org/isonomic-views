//! The Isonomic Views formula language.
//!
//! Expressions are pure and total: no side effects, no I/O, no unbounded loops, and every
//! evaluation runs under a step limit. This crate depends on no other Views crate so it can
//! move to its own repository when the data grid needs it.

mod date;
mod deps;
mod eval;
mod format;
mod lexer;
mod parser;
mod stdlib;
mod value;

use std::fmt;

pub use date::{format as format_date, iso as iso_date, parse_iso as parse_date};
pub use deps::deps;
pub use eval::{
    Empty, ErrorKind, EvalError, EvalResult, Evaluator, Limits, Scope, compare, evaluate,
};
pub use format::{compact, currency, grouped, percent};
pub use parser::{BinOp, Expr, MAX_DEPTH, UnOp, parse, parse_seq};
pub use stdlib::FUNCTIONS;
pub use value::{Value, fmt_num};

/// A syntax error, with the byte offset where it was found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub offset: usize,
    /// True when the input simply ended too early. A streaming caller holds such an
    /// expression back instead of reporting it.
    pub incomplete: bool,
}

impl ParseError {
    pub fn new(message: impl Into<String>, offset: usize) -> Self {
        ParseError {
            message: message.into(),
            offset,
            incomplete: false,
        }
    }

    pub fn incomplete(message: impl Into<String>, offset: usize) -> Self {
        ParseError {
            message: message.into(),
            offset,
            incomplete: true,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}", self.message, self.offset)
    }
}

impl std::error::Error for ParseError {}

/// Parse and evaluate in one step.
pub fn eval_str(src: &str, scope: &dyn Scope, limits: Limits) -> Result<Value, String> {
    let e = parse(src).map_err(|e| e.to_string())?;
    evaluate(&e, scope, limits).map_err(|e| e.to_string())
}
