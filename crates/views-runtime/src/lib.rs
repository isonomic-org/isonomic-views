// Copyright 2026 Anud Inc.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Live state for Isonomic Views.
//!
//! Three scopes: `local` (per viewer, not saved), `shared` (saved with the view, identical for
//! every viewer, behind [`SharedStore`]) and `data` (read from the [`Host`]). Bindings are
//! cached with the state paths they read; a write invalidates only the bindings that depend
//! on it. Actions are declarative, and anything that reaches outside goes through the host,
//! which denies by default.

mod actions;
mod host;
mod runtime;
mod store;

pub use actions::{ACTIONS, Effect};
pub use host::{DenyAll, FixtureHost, Host, HostError};
pub use runtime::{Frame, Inst, Runtime};
pub use store::{MemoryStore, SharedHandle, SharedStore};
pub use views_expr::Value;
