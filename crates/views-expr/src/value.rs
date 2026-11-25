// Copyright 2026 Anud Inc.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Runtime values of the formula language.

use std::{collections::BTreeMap, sync::Arc};

use crate::date;

/// A value. Cheap to clone: lists, maps and strings are shared.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum Value {
    #[default]
    Null,
    Bool(bool),
    Num(f64),
    Str(Arc<str>),
    List(Arc<Vec<Value>>),
    Map(Arc<BTreeMap<String, Value>>),
    /// Days since 1970-01-01.
    Date(i64),
}

impl Value {
    pub fn str(s: impl AsRef<str>) -> Self {
        Value::Str(Arc::from(s.as_ref()))
    }

    pub fn list(items: Vec<Value>) -> Self {
        Value::List(Arc::new(items))
    }

    pub fn map(entries: BTreeMap<String, Value>) -> Self {
        Value::Map(Arc::new(entries))
    }

    pub fn empty_map() -> Self {
        Value::Map(Arc::new(BTreeMap::new()))
    }

    pub fn truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Num(n) => *n != 0.0 && !n.is_nan(),
            Value::Str(s) => !s.is_empty(),
            Value::List(l) => !l.is_empty(),
            Value::Map(m) => !m.is_empty(),
            Value::Date(_) => true,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Num(_) => "number",
            Value::Str(_) => "text",
            Value::List(_) => "list",
            Value::Map(_) => "record",
            Value::Date(_) => "date",
        }
    }

    pub fn as_num(&self) -> Option<f64> {
        match self {
            Value::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Value::List(l) => Some(l),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Field of a record; null for anything else.
    pub fn field(&self, key: &str) -> Value {
        match self {
            Value::Map(m) => m.get(key).cloned().unwrap_or_default(),
            _ => Value::Null,
        }
    }

    /// Element of a list by position, or field of a record by name.
    pub fn index(&self, key: &Value) -> Value {
        match (self, key) {
            (Value::List(l), Value::Num(n)) if *n >= 0.0 && n.fract() == 0.0 => {
                l.get(*n as usize).cloned().unwrap_or_default()
            }
            (Value::Map(_), Value::Str(s)) => self.field(s),
            _ => Value::Null,
        }
    }

    /// Follow a dotted path. Numeric segments index lists.
    pub fn at_path<S: AsRef<str>>(&self, path: &[S]) -> Value {
        let mut cur = self.clone();
        for seg in path {
            let seg = seg.as_ref();
            cur = match &cur {
                Value::List(_) => match seg.parse::<usize>() {
                    Ok(i) => cur.index(&Value::Num(i as f64)),
                    Err(_) => Value::Null,
                },
                _ => cur.field(seg),
            };
        }
        cur
    }

    /// A copy with the value at `path` replaced. Missing records are created.
    pub fn with_path<S: AsRef<str>>(&self, path: &[S], new: Value) -> Value {
        let Some((head, rest)) = path.split_first() else {
            return new;
        };
        let head = head.as_ref();
        match self {
            Value::List(l) => {
                let Ok(i) = head.parse::<usize>() else {
                    return self.clone();
                };
                if i >= l.len() {
                    return self.clone();
                }
                let mut items = (**l).clone();
                items[i] = items[i].with_path(rest, new);
                Value::list(items)
            }
            Value::Map(m) => {
                let mut m = (**m).clone();
                let child = m.get(head).cloned().unwrap_or_default();
                m.insert(head.to_string(), child.with_path(rest, new));
                Value::map(m)
            }
            _ => {
                let mut m = BTreeMap::new();
                m.insert(head.to_string(), Value::Null.with_path(rest, new));
                Value::map(m)
            }
        }
    }

    /// Text shown to a person.
    pub fn to_text(&self) -> String {
        match self {
            Value::Null => String::new(),
            Value::Bool(b) => b.to_string(),
            Value::Num(n) => fmt_num(*n),
            Value::Str(s) => s.to_string(),
            Value::Date(d) => date::iso(*d),
            Value::List(l) => l.iter().map(|v| v.to_text()).collect::<Vec<_>>().join(", "),
            Value::Map(_) => self.to_json().to_string(),
        }
    }

    pub fn from_json(j: &serde_json::Value) -> Value {
        use serde_json::Value as J;
        match j {
            J::Null => Value::Null,
            J::Bool(b) => Value::Bool(*b),
            J::Number(n) => Value::Num(n.as_f64().unwrap_or(0.0)),
            J::String(s) => Value::str(s),
            J::Array(a) => Value::list(a.iter().map(Value::from_json).collect()),
            J::Object(o) => Value::map(
                o.iter()
                    .map(|(k, v)| (k.clone(), Value::from_json(v)))
                    .collect(),
            ),
        }
    }

    /// JSON form. Dates become ISO strings; non-finite numbers become null.
    pub fn to_json(&self) -> serde_json::Value {
        use serde_json::Value as J;
        match self {
            Value::Null => J::Null,
            Value::Bool(b) => J::Bool(*b),
            Value::Num(n) => serde_json::Number::from_f64(*n).map_or(J::Null, J::Number),
            Value::Str(s) => J::String(s.to_string()),
            Value::Date(d) => J::String(date::iso(*d)),
            Value::List(l) => J::Array(l.iter().map(|v| v.to_json()).collect()),
            Value::Map(m) => J::Object(m.iter().map(|(k, v)| (k.clone(), v.to_json())).collect()),
        }
    }
}

impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Value::Num(n)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::str(s)
    }
}

/// Plain number text: integers without a decimal point, others trimmed.
pub fn fmt_num(n: f64) -> String {
    if n.is_nan() {
        return "NaN".into();
    }
    if n.is_infinite() {
        return if n > 0.0 { "∞".into() } else { "-∞".into() };
    }
    if n.fract() == 0.0 && n.abs() < 1e15 {
        return format!("{}", n as i64);
    }
    let s = format!("{n:.10}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}
