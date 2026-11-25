// Copyright 2026 Anud Inc.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Renders an Isonomic View into native components.
//!
//! [`ViewPane`] is a component: give it a source (whole, streamed, or as edits) and it renders
//! the view with real components from the `ui` crate. A keyed reconciler maps the node map to
//! elements; component state (input text, slider position, selection, open sections) is kept
//! per node ID and loop instance, so it survives re-renders, streaming and edits.

mod dates;
mod display;
pub mod fixtures;
mod pane;
mod render;
pub mod style;
mod widgets;

#[cfg(feature = "headless")]
pub mod headless;

pub use pane::{FrameClock, ViewPane};
