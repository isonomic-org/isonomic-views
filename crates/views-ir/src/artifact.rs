//! The two ways a view is written: created by streaming, updated by small text edits.

use serde::{Deserialize, Serialize};

use crate::grow::Grow;
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
///
/// While the source only grows, each change re-parses only from the construct the previous
/// end of the source cut off; settled subtrees are not looked at again. Any other change
/// re-parses in full, aligning against the previous map. Both give the same map and patches.
#[derive(Clone, Debug, Default)]
pub struct Artifact {
    source: String,
    doc: Doc,
    version: u64,
    limits: Limits,
    /// Streaming state for `source`, when one could be built.
    grow: Option<Grow>,
    full_reparses: u64,
}

impl Artifact {
    /// Start a view, empty or with initial text.
    pub fn create(initial: &str, limits: Limits) -> Artifact {
        let mut doc = Doc {
            next_id: 1,
            ..Doc::default()
        };
        let mut grow = Grow::new(&limits);
        let grow = match grow.advance(initial, &mut doc, &limits) {
            Some(_) => Some(grow),
            None => {
                doc = Doc::parse(initial, &limits);
                Grow::resync(initial, &doc, &limits)
            }
        };
        Artifact {
            source: initial.to_string(),
            doc,
            version: 1,
            limits,
            grow,
            full_reparses: 0,
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

    /// Changes that re-parsed the whole source: anything but text added at the end, or, while
    /// streaming, a parser state the incremental path does not expect.
    pub fn full_reparses(&self) -> u64 {
        self.full_reparses
    }

    /// Streaming: append a chunk and return the patches.
    pub fn append(&mut self, chunk: &str) -> Vec<Patch> {
        self.source.push_str(chunk);
        self.refresh(true)
    }

    /// Replace the whole source (the host re-sent it) and return the patches.
    pub fn replace_source(&mut self, source: String) -> Vec<Patch> {
        let grown = source.starts_with(self.source.as_str());
        self.source = source;
        self.refresh(grown)
    }

    /// Bring the map up to date with the source. `grown`: the source only gained text at the
    /// end since the last refresh.
    fn refresh(&mut self, grown: bool) -> Vec<Patch> {
        self.version += 1;
        if grown
            && let Some(g) = &mut self.grow
            && let Some(patches) = g.advance(&self.source, &mut self.doc, &self.limits)
        {
            return patches;
        }
        self.full_reparses += 1;
        let (doc, patches) = self.doc.reparse(&self.source, &self.limits);
        self.doc = doc;
        self.grow = Grow::resync(&self.source, &self.doc, &self.limits);
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
