//! The two ways a view is written: created by streaming, updated by small text edits.

use serde::{Deserialize, Serialize};

use crate::{Doc, Limits, Patch};

/// One find-and-replace edit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edit {
    pub find: String,
    pub replace: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum EditError {
    /// Edit `index` found nothing.
    NotFound { index: usize },
    /// Edit `index` matched `count` places; it must match exactly one.
    Ambiguous { index: usize, count: usize },
    /// Edit `index` has an empty `find`.
    Empty { index: usize },
    /// The result would exceed the source limit.
    TooLarge { bytes: usize },
}

/// A view's source and node map, kept in step.
#[derive(Clone, Debug, Default)]
pub struct Artifact {
    source: String,
    doc: Doc,
    version: u64,
    limits: Limits,
}

impl Artifact {
    /// Start a view, empty or with initial text.
    pub fn create(initial: &str, limits: Limits) -> Artifact {
        Artifact {
            source: initial.to_string(),
            doc: Doc::parse(initial, &limits),
            version: 1,
            limits,
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn doc(&self) -> &Doc {
        &self.doc
    }

    /// Increases with every change.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Streaming: append a chunk and return the patches.
    pub fn append(&mut self, chunk: &str) -> Vec<Patch> {
        let mut next = std::mem::take(&mut self.source);
        next.push_str(chunk);
        self.replace_source(next)
    }

    /// Replace the whole source (the host re-sent it) and return the patches.
    pub fn replace_source(&mut self, source: String) -> Vec<Patch> {
        let (doc, patches) = self.doc.reparse(&source, &self.limits);
        self.source = source;
        self.doc = doc;
        self.version += 1;
        patches
    }

    /// Update: apply edits in order, each matching exactly once in the text as it stands
    /// after the previous edits. All or nothing: on error the artifact is unchanged.
    pub fn apply_edits(&mut self, edits: &[Edit]) -> Result<Vec<Patch>, EditError> {
        let mut text = self.source.clone();
        for (index, e) in edits.iter().enumerate() {
            if e.find.is_empty() {
                return Err(EditError::Empty { index });
            }
            match text.matches(e.find.as_str()).count() {
                0 => return Err(EditError::NotFound { index }),
                1 => text = text.replacen(e.find.as_str(), &e.replace, 1),
                count => return Err(EditError::Ambiguous { index, count }),
            }
        }
        if text.len() > self.limits.max_source {
            return Err(EditError::TooLarge { bytes: text.len() });
        }
        Ok(self.replace_source(text))
    }
}
