//! The one door to the outside world. Everything denies by default.

use std::{collections::BTreeMap, fmt, sync::Arc};

use views_expr::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostError {
    /// The host did not grant this.
    Denied(String),
    /// Granted, but it failed.
    Failed(String),
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HostError::Denied(what) => write!(f, "the host did not allow {what}"),
            HostError::Failed(why) => write!(f, "the host request failed: {why}"),
        }
    }
}

/// Everything a view may ask of the world. A view has no network, files or data of its
/// own; the host decides what to grant. Every method denies unless the host overrides it.
pub trait Host {
    /// Read the named source for the `data` scope (`data.sales`).
    fn data(&self, name: &str) -> Result<Value, HostError> {
        Err(HostError::Denied(format!("data '{name}'")))
    }

    /// Run a named query: the `query(name, args, into)` action.
    fn query(&self, name: &str, _args: &Value) -> Result<Value, HostError> {
        Err(HostError::Denied(format!("query '{name}'")))
    }

    /// Call a named function: the `call(name, args, into?)` action.
    fn call(&self, name: &str, _args: &Value) -> Result<Value, HostError> {
        Err(HostError::Denied(format!("function '{name}'")))
    }

    /// Open something (a document, a link, a record): the `open(target)` action.
    fn open(&self, target: &str) -> Result<(), HostError> {
        Err(HostError::Denied(format!("opening '{target}'")))
    }

    /// Image bytes for an asset the host knows. Views cannot name URLs.
    fn image(&self, asset: &str) -> Result<Arc<[u8]>, HostError> {
        Err(HostError::Denied(format!("image '{asset}'")))
    }
}

/// The default host: denies everything.
#[derive(Clone, Copy, Debug, Default)]
pub struct DenyAll;

impl Host for DenyAll {}

/// An in-memory host with fixed data, used by tests, screenshots and the playground.
/// Queries and calls named in `results` return that value; `open` records the target.
#[derive(Debug, Default)]
pub struct FixtureHost {
    pub data: BTreeMap<String, Value>,
    pub results: BTreeMap<String, Value>,
    pub opened: std::cell::RefCell<Vec<String>>,
}

impl FixtureHost {
    pub fn with_data(mut self, name: &str, v: Value) -> Self {
        self.data.insert(name.into(), v);
        self
    }

    pub fn with_result(mut self, name: &str, v: Value) -> Self {
        self.results.insert(name.into(), v);
        self
    }
}

impl Host for FixtureHost {
    fn data(&self, name: &str) -> Result<Value, HostError> {
        self.data
            .get(name)
            .cloned()
            .ok_or_else(|| HostError::Denied(format!("data '{name}'")))
    }

    fn query(&self, name: &str, _: &Value) -> Result<Value, HostError> {
        self.results
            .get(name)
            .cloned()
            .ok_or_else(|| HostError::Denied(format!("query '{name}'")))
    }

    fn call(&self, name: &str, _: &Value) -> Result<Value, HostError> {
        self.results
            .get(name)
            .cloned()
            .ok_or_else(|| HostError::Denied(format!("function '{name}'")))
    }

    fn open(&self, target: &str) -> Result<(), HostError> {
        self.opened.borrow_mut().push(target.to_string());
        Ok(())
    }
}
