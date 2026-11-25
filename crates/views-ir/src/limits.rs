// Copyright 2026 Anud Inc.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Hard limits. A hostile view can waste its own rectangle and nothing else.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limits {
    /// Nodes in the map, including text nodes. Extra nodes are dropped.
    pub max_nodes: usize,
    /// Element nesting below the root. Deeper subtrees are dropped.
    pub max_depth: usize,
    /// Source size in bytes. Only this much of the source is read.
    pub max_source: usize,
    /// Longest text a single text node or literal may carry, in bytes.
    pub max_text: usize,
    /// Longest list an expression may build, and most items a `For` or table renders.
    pub max_list: usize,
    /// Work units per expression evaluation.
    pub max_expr_steps: u64,
    /// Rendering time per frame, in microseconds. Past it the rest of the view is skipped.
    pub max_render_us: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_nodes: 2_000,
            max_depth: 32,
            max_source: 256 * 1024,
            max_text: 16 * 1024,
            max_list: 500,
            max_expr_steps: 20_000,
            max_render_us: 8_000,
        }
    }
}
