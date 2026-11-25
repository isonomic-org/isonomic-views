// Copyright 2026 Anud Inc.
// SPDX-License-Identifier: GPL-3.0-or-later

//! The view pane: owns the source, the runtime and per-node component state.

use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    time::Instant,
};

use ui::{
    AnyElement, Context, Entity, IntoElement, ParentElement, Render, Styled, Subscription, Window,
    component::{
        date_picker::DatePickerState, input::InputState, input::TextareaState, slider::SliderState,
    },
    div,
};
use views_ir::{Artifact, Diagnostic, Edit, EditError, Limits, NodeId, Patch};
use views_runtime::{DenyAll, Frame, Host, Inst, MemoryStore, Runtime, SharedHandle};

use crate::widgets::SelectEntity;

/// Identifies the component state of one rendered instance of a node.
pub type StateKey = (NodeId, Inst);

pub(crate) enum Slot {
    Input(Entity<InputState>),
    Area(Entity<TextareaState>),
    Number(Entity<InputState>),
    Slider(Entity<SliderState>),
    Select(Entity<SelectEntity>, Vec<views_expr::Value>),
    Date(Entity<DatePickerState>),
}

pub(crate) struct Held {
    pub slot: Slot,
    pub _subs: Vec<Subscription>,
}

/// Measures render time against the per-frame budget. Tests inject a fake.
pub trait FrameClock {
    fn start(&mut self);
    fn elapsed_us(&self) -> u64;
}

struct RealClock(Option<Instant>);

impl FrameClock for RealClock {
    fn start(&mut self) {
        self.0 = Some(Instant::now());
    }

    fn elapsed_us(&self) -> u64 {
        self.0.map_or(0, |t| t.elapsed().as_micros() as u64)
    }
}

/// A live view. Feed it source (all at once, streamed, or as edits); it renders native
/// components and keeps their state across updates, keyed by node ID.
pub struct ViewPane {
    pub(crate) artifact: Artifact,
    pub(crate) rt: Runtime,
    pub(crate) held: HashMap<StateKey, Held>,
    pub(crate) seen: HashSet<StateKey>,
    /// UI-only state: selected tab index and open accordion sections, per instance.
    pub(crate) tabs: HashMap<StateKey, usize>,
    pub(crate) open: HashMap<StateKey, Vec<usize>>,
    /// Text each node showed in the last frame (first instance). For tests and the report.
    pub(crate) shown: HashMap<NodeId, String>,
    pub(crate) clock: Box<dyn FrameClock>,
    pub(crate) truncated: bool,
    frames: u64,
    build_us: u64,
}

impl ViewPane {
    /// A pane with the default host (denies everything) and its own shared store.
    pub fn new(source: &str) -> ViewPane {
        ViewPane::with_host(
            source,
            Rc::new(DenyAll),
            MemoryStore::handle(),
            Limits::default(),
        )
    }

    pub fn with_host(
        source: &str,
        host: Rc<dyn Host>,
        store: SharedHandle,
        limits: Limits,
    ) -> ViewPane {
        let artifact = Artifact::create(source, limits);
        let rt = Runtime::new(artifact.doc().clone(), host, store, limits);
        ViewPane {
            artifact,
            rt,
            held: HashMap::new(),
            seen: HashSet::new(),
            tabs: HashMap::new(),
            open: HashMap::new(),
            shown: HashMap::new(),
            clock: Box::new(RealClock(None)),
            truncated: false,
            frames: 0,
            build_us: 0,
        }
    }

    /// Replace the render clock (tests).
    pub fn set_clock(&mut self, clock: Box<dyn FrameClock>) {
        self.clock = clock;
    }

    pub fn source(&self) -> &str {
        self.artifact.source()
    }

    pub fn runtime(&self) -> &Runtime {
        &self.rt
    }

    /// Streaming: append a chunk of source.
    pub fn append(&mut self, chunk: &str, cx: &mut Context<Self>) -> Vec<Patch> {
        let p = self.artifact.append(chunk);
        self.refresh(cx);
        p
    }

    /// Replace the whole source.
    pub fn set_source(&mut self, source: &str, cx: &mut Context<Self>) -> Vec<Patch> {
        let p = self.artifact.replace_source(source.to_string());
        self.refresh(cx);
        p
    }

    /// Update: apply find/replace edits (all or nothing).
    pub fn apply_edits(
        &mut self,
        edits: &[Edit],
        cx: &mut Context<Self>,
    ) -> Result<Vec<Patch>, EditError> {
        let p = self.artifact.apply_edits(edits)?;
        self.refresh(cx);
        Ok(p)
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.rt.set_doc(self.artifact.doc().clone());
        cx.notify();
    }

    /// Everything wrong with the view: parse and limit problems, catalog checks, and runtime
    /// errors.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut d = views_catalog::validate(self.artifact.doc());
        d.extend(self.rt.diagnostics());
        d
    }

    /// The text a node showed in the last frame.
    pub fn shown(&self, id: NodeId) -> Option<&str> {
        self.shown.get(&id).map(String::as_str)
    }

    /// The first node with this tag, in document order.
    pub fn find(&self, tag: &str) -> Option<NodeId> {
        let doc = self.artifact.doc();
        doc.preorder()
            .into_iter()
            .find(|id| doc.nodes[id].tag == tag)
    }

    /// The entity holding a slider's state, if that node is rendered.
    pub fn slider(&self, id: NodeId) -> Option<Entity<SliderState>> {
        self.held
            .iter()
            .find(|((n, _), _)| *n == id)
            .and_then(|(_, h)| match &h.slot {
                Slot::Slider(e) => Some(e.clone()),
                _ => None,
            })
    }

    /// The entity holding a text or number input's state.
    pub fn input(&self, id: NodeId) -> Option<Entity<InputState>> {
        self.held
            .iter()
            .find(|((n, _), _)| *n == id)
            .and_then(|(_, h)| match &h.slot {
                Slot::Input(e) | Slot::Number(e) => Some(e.clone()),
                _ => None,
            })
    }

    /// Number of live component-state entities.
    pub fn state_count(&self) -> usize {
        self.held.len()
    }

    /// The last frame ran out of its time budget and skipped part of the view.
    pub fn truncated(&self) -> bool {
        self.truncated
    }

    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// Time the last frame spent building elements, in microseconds (layout and paint follow).
    pub fn build_us(&self) -> u64 {
        self.build_us
    }
}

impl Render for ViewPane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.frames += 1;
        self.rt.sync();
        self.seen.clear();
        self.shown.clear();
        self.truncated = false;
        self.clock.start();
        let body: Vec<AnyElement> = match self.rt.doc().root {
            Some(root) => self.children_of_list(&[root], &Frame::root(), window, cx),
            None => vec![],
        };
        // Keep state for nodes that still exist; drop loop instances not rendered this frame.
        let doc = self.rt.doc();
        let seen = &self.seen;
        self.held
            .retain(|k, _| doc.nodes.contains_key(&k.0) && (k.1.0.is_empty() || seen.contains(k)));
        self.tabs.retain(|k, _| doc.nodes.contains_key(&k.0));
        self.open.retain(|k, _| doc.nodes.contains_key(&k.0));
        let theme = crate::style::tokens(cx);
        let mut root = div()
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            .font_family(theme.font_family.clone());
        if self.truncated {
            root = root.child(
                div()
                    .px_2()
                    .py_1()
                    .text_xs()
                    .text_color(theme.warning)
                    .child("This view took too long to draw; part of it is hidden."),
            );
        }
        self.build_us = self.clock.elapsed_us();
        root.children(body)
    }
}
