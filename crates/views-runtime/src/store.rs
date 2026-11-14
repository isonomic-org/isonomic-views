//! Storage for the `shared` scope: saved with the view, identical for every viewer.

use std::{cell::RefCell, rc::Rc};

use views_expr::Value;

/// Where `shared` state lives. A collaborative store implements this later; every viewer of
/// a view holds the same store.
pub trait SharedStore {
    /// The whole `shared` record.
    fn get(&self) -> Value;
    /// Replace the value at `path` (relative to `shared`).
    fn set(&mut self, path: &[String], value: Value);
    /// Add any field of `defaults` that is not set yet. Existing values are kept.
    fn init(&mut self, defaults: &Value);
    /// Increases on every change, so viewers know when to refresh.
    fn version(&self) -> u64;
}

pub type SharedHandle = Rc<RefCell<dyn SharedStore>>;

/// In-memory implementation.
#[derive(Debug, Default)]
pub struct MemoryStore {
    value: Value,
    version: u64,
}

impl MemoryStore {
    pub fn new() -> Self {
        MemoryStore {
            value: Value::empty_map(),
            version: 0,
        }
    }

    pub fn handle() -> SharedHandle {
        Rc::new(RefCell::new(MemoryStore::new()))
    }
}

impl SharedStore for MemoryStore {
    fn get(&self) -> Value {
        self.value.clone()
    }

    fn set(&mut self, path: &[String], value: Value) {
        self.value = self.value.with_path(path, value);
        self.version += 1;
    }

    fn init(&mut self, defaults: &Value) {
        let Value::Map(d) = defaults else { return };
        let mut changed = false;
        for (k, v) in d.iter() {
            if self.value.field(k).is_null() {
                self.value = self.value.with_path(&[k.as_str()], v.clone());
                changed = true;
            }
        }
        if changed {
            self.version += 1;
        }
    }

    fn version(&self) -> u64 {
        self.version
    }
}
