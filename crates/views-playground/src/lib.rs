// Copyright 2026 Anud Inc.
// SPDX-License-Identifier: GPL-3.0-or-later

//! The playground: source on the left, the live view on the right, diagnostics below, and a
//! replay mode that feeds the source in a few characters at a time, like a model streaming.

mod app;

pub use app::{EXAMPLES, Playground};
