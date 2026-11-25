// Copyright 2026 Anud Inc.
// SPDX-License-Identifier: GPL-3.0-or-later

//! The reconciler: walks the node map and builds elements. Layout, text and control nodes
//! live here; inputs are in `widgets`, display components in `display`.

use std::hash::{DefaultHasher, Hash, Hasher};

use ui::{
    AnyElement, Context, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, Window,
    component::{
        accordion::Accordion,
        h_flex,
        separator::Separator,
        tab::{Tab, TabBar},
        text::TextView,
        v_flex,
    },
    div, px,
};
use views_expr::Value;
use views_ir::{Kind, NodeId};
use views_runtime::Frame;

use crate::{ViewPane, style};

impl ViewPane {
    /// Elements for a list of sibling nodes. `If` and `For` expand in place.
    pub(crate) fn children_of_list(
        &mut self,
        ids: &[NodeId],
        frame: &Frame,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let mut out = Vec::new();
        for &id in ids {
            let Some(n) = self.rt.doc().node(id) else {
                continue;
            };
            match (n.kind, n.tag.as_str()) {
                (Kind::Element, "If") => {
                    if self.rt.eval(id, "when", frame).truthy() {
                        let kids = n.children.clone();
                        out.extend(self.children_of_list(&kids, frame, window, cx));
                    }
                }
                (Kind::Element, "For") => {
                    let kids = n.children.clone();
                    for f in self.rt.for_frames(id, frame) {
                        out.extend(self.children_of_list(&kids, &f, window, cx));
                    }
                }
                _ => out.push(self.element(id, frame, window, cx)),
            }
        }
        out
    }

    pub(crate) fn kids(
        &mut self,
        id: NodeId,
        frame: &Frame,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let kids = self
            .rt
            .doc()
            .node(id)
            .map(|n| n.children.clone())
            .unwrap_or_default();
        self.children_of_list(&kids, frame, window, cx)
    }

    /// A unique element ID for one instance of a node.
    pub(crate) fn eid(&self, id: NodeId, frame: &Frame) -> ElementId {
        let mut h = DefaultHasher::new();
        frame.inst.hash(&mut h);
        ElementId::Name(SharedString::from(format!("v{id}-{:x}", h.finish())))
    }

    pub(crate) fn key(&self, id: NodeId, frame: &Frame) -> crate::pane::StateKey {
        (id, frame.inst.clone())
    }

    /// A prop as text; None when missing or null.
    pub(crate) fn s(&self, id: NodeId, prop: &str, frame: &Frame) -> Option<String> {
        match self.rt.eval(id, prop, frame) {
            Value::Null => None,
            v => Some(v.to_text()),
        }
    }

    pub(crate) fn num(&self, id: NodeId, prop: &str, frame: &Frame) -> Option<f64> {
        match self.rt.eval(id, prop, frame) {
            Value::Num(n) => Some(n),
            Value::Str(s) => s.trim().parse().ok(),
            _ => None,
        }
    }

    pub(crate) fn flag(&self, id: NodeId, prop: &str, frame: &Frame) -> bool {
        match self.rt.eval(id, prop, frame) {
            Value::Str(s) => &*s == "true",
            v => v.truthy(),
        }
    }

    pub(crate) fn class(&self, id: NodeId) -> Option<String> {
        self.rt
            .doc()
            .node(id)
            .and_then(|n| n.prop("class"))
            .map(str::to_string)
    }

    pub(crate) fn note(&mut self, id: NodeId, frame: &Frame, text: &str) {
        if frame.inst.0.is_empty() || !self.shown.contains_key(&id) {
            self.shown.insert(id, text.to_string());
        }
    }

    /// Run an event prop's actions and redraw.
    pub(crate) fn fire(
        &mut self,
        id: NodeId,
        prop: &str,
        frame: &Frame,
        event: Value,
        cx: &mut Context<Self>,
    ) {
        if self.rt.has(id, prop) {
            self.rt.dispatch(id, prop, frame, event);
        }
        cx.notify();
    }

    fn element(
        &mut self,
        id: NodeId,
        frame: &Frame,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.clock.elapsed_us() > self.rt.limits().max_render_us {
            self.truncated = true;
            return div().into_any_element();
        }
        self.seen.insert(self.key(id, frame));
        let Some(n) = self.rt.doc().node(id).cloned() else {
            return div().into_any_element();
        };
        if n.kind != Kind::Element {
            // Bare text inside a container: show it as body text.
            let t = match n.kind {
                Kind::Expr => self.rt.eval(id, "value", frame).to_text(),
                _ => n.text.clone(),
            };
            return div().child(t).into_any_element();
        }
        let class = self.class(id);
        let theme = style::tokens(cx);
        let el: AnyElement = match n.tag.as_str() {
            "View" => {
                let kids = self.kids(id, frame, window, cx);
                style::apply(v_flex().w_full().gap_3().p_4(), class.as_deref(), cx)
                    .children(kids)
                    .into_any_element()
            }
            "Row" => {
                let kids = self.kids(id, frame, window, cx);
                let mut e = h_flex().w_full().gap_2().items_center();
                e = match self.s(id, "align", frame).as_deref() {
                    Some("start") => e.items_start(),
                    Some("end") => e.items_end(),
                    Some("stretch") => e.items_stretch(),
                    _ => e,
                };
                e = justify(e, self.s(id, "justify", frame).as_deref());
                if self.flag(id, "wrap", frame) {
                    e = e.flex_wrap();
                }
                style::apply(e, class.as_deref(), cx)
                    .children(kids)
                    .into_any_element()
            }
            "Col" => {
                let kids = self.kids(id, frame, window, cx);
                let mut e = v_flex().gap_2();
                e = match self.s(id, "align", frame).as_deref() {
                    Some("start") => e.items_start(),
                    Some("center") => e.items_center(),
                    Some("end") => e.items_end(),
                    _ => e,
                };
                e = justify(e, self.s(id, "justify", frame).as_deref());
                style::apply(e, class.as_deref(), cx)
                    .children(kids)
                    .into_any_element()
            }
            "Grid" => {
                let kids = self.kids(id, frame, window, cx);
                let cols = self.num(id, "cols", frame).unwrap_or(2.0).clamp(1.0, 6.0) as u16;
                style::apply(
                    div().grid().grid_cols(cols).gap_3().w_full(),
                    class.as_deref(),
                    cx,
                )
                .children(kids)
                .into_any_element()
            }
            "Card" => {
                let kids = self.kids(id, frame, window, cx);
                let mut e = v_flex()
                    .gap_3()
                    .p_4()
                    .border_1()
                    .border_color(theme.border)
                    .rounded(theme.radius_lg)
                    .bg(theme.background);
                if let Some(t) = self.s(id, "title", frame) {
                    let mut head = v_flex()
                        .gap_0p5()
                        .child(div().font_weight(FontWeight::SEMIBOLD).child(t));
                    if let Some(d) = self.s(id, "description", frame) {
                        head =
                            head.child(div().text_sm().text_color(theme.muted_foreground).child(d));
                    }
                    e = e.child(head);
                }
                style::apply(e, class.as_deref(), cx)
                    .children(kids)
                    .into_any_element()
            }
            "Tabs" => self.tabs_el(id, frame, window, cx),
            "Accordion" => self.accordion_el(id, frame, window, cx),
            "Tab" | "Section" => {
                let kids = self.kids(id, frame, window, cx);
                v_flex().gap_2().children(kids).into_any_element()
            }
            "Divider" => if self.flag(id, "vertical", frame) {
                Separator::vertical()
            } else {
                Separator::horizontal()
            }
            .into_any_element(),
            "Spacer" => div().flex_1().into_any_element(),
            "ScrollArea" => {
                let kids = self.kids(id, frame, window, cx);
                let h =
                    views_catalog::size_px(self.s(id, "height", frame).as_deref().unwrap_or("md"));
                let e = div()
                    .id(self.eid(id, frame))
                    .h(px(h))
                    .w_full()
                    .overflow_y_scroll()
                    .child(v_flex().gap_2().children(kids));
                style::apply(e, class.as_deref(), cx).into_any_element()
            }
            "Heading" => {
                let t = self.rt.text(id, frame);
                self.note(id, frame, &t);
                let e = div().font_weight(FontWeight::SEMIBOLD).child(t);
                let e = match self.num(id, "level", frame).unwrap_or(2.0) as u8 {
                    1 => e.text_2xl().font_weight(FontWeight::BOLD),
                    2 => e.text_xl(),
                    3 => e.text_lg(),
                    _ => e.text_base(),
                };
                style::apply(e, class.as_deref(), cx).into_any_element()
            }
            "Text" => {
                let t = self.rt.text(id, frame);
                self.note(id, frame, &t);
                let mut e = div().child(t);
                if let Some(tn) = self.s(id, "tone", frame).filter(|t| t != "default") {
                    e = e.text_color(style::tone(&theme, &tn));
                }
                if let Some(sz) = self.s(id, "size", frame) {
                    e = style::text_size(e, &sz);
                }
                if let Some(w) = self.s(id, "weight", frame) {
                    e = style::weight(e, &w);
                }
                style::apply(e, class.as_deref(), cx).into_any_element()
            }
            "Markdown" => {
                let t = self.rt.text_raw(id, frame);
                self.note(id, frame, &t);
                style::apply(
                    div()
                        .w_full()
                        .child(TextView::markdown(self.eid(id, frame), t)),
                    class.as_deref(),
                    cx,
                )
                .into_any_element()
            }
            "Code" => {
                let t = self.rt.text_raw(id, frame);
                self.note(id, frame, &t);
                let mut e = v_flex()
                    .w_full()
                    .gap_1()
                    .p_3()
                    .rounded(theme.radius)
                    .bg(theme.muted)
                    .text_sm()
                    .font_family(theme.mono_font_family.clone());
                if let Some(l) = self.s(id, "lang", frame) {
                    e = e.child(div().text_xs().text_color(theme.muted_foreground).child(l));
                }
                style::apply(e.child(t), class.as_deref(), cx).into_any_element()
            }
            _ => self.widget(&n, frame, window, cx),
        };
        el
    }

    fn tabs_el(
        &mut self,
        id: NodeId,
        frame: &Frame,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let tabs: Vec<(NodeId, String, String)> = self
            .rt
            .doc()
            .node(id)
            .map(|n| n.children.clone())
            .unwrap_or_default()
            .into_iter()
            .filter(|c| self.rt.doc().node(*c).is_some_and(|n| n.is("Tab")))
            .map(|c| {
                let label = self.s(c, "label", frame).unwrap_or_default();
                let value = self.s(c, "value", frame).unwrap_or_else(|| label.clone());
                (c, label, value)
            })
            .collect();
        let key = self.key(id, frame);
        let bound = self
            .rt
            .has(id, "bind")
            .then(|| self.rt.eval(id, "bind", frame).to_text());
        let selected = bound
            .and_then(|b| tabs.iter().position(|t| t.2 == b))
            .or_else(|| self.tabs.get(&key).copied())
            .unwrap_or(0)
            .min(tabs.len().saturating_sub(1));
        let values: Vec<String> = tabs.iter().map(|t| t.2.clone()).collect();
        let f = frame.clone();
        let bar = TabBar::new(self.eid(id, frame))
            .selected_index(selected)
            .children(tabs.iter().map(|t| Tab::new().label(t.1.clone())))
            .on_click(cx.listener(move |this, ix: &usize, _, cx| {
                this.tabs.insert((id, f.inst.clone()), *ix);
                if let Some(v) = values.get(*ix) {
                    let v = Value::str(v);
                    if this.rt.has(id, "bind") {
                        let _ = this.rt.write_bind(id, &f, v.clone());
                    }
                    this.fire(id, "onChange", &f, v, cx);
                }
                cx.notify();
            }));
        let body = match tabs.get(selected) {
            Some((c, _, _)) => self.element(*c, frame, window, cx),
            None => div().into_any_element(),
        };
        let class = self.class(id);
        style::apply(v_flex().w_full().gap_3(), class.as_deref(), cx)
            .child(bar)
            .child(body)
            .into_any_element()
    }

    fn accordion_el(
        &mut self,
        id: NodeId,
        frame: &Frame,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let sections: Vec<NodeId> = self
            .rt
            .doc()
            .node(id)
            .map(|n| n.children.clone())
            .unwrap_or_default()
            .into_iter()
            .filter(|c| self.rt.doc().node(*c).is_some_and(|n| n.is("Section")))
            .collect();
        let key = self.key(id, frame);
        if !self.open.contains_key(&key) {
            let initial = sections
                .iter()
                .enumerate()
                .filter(|(_, s)| self.flag(**s, "open", frame))
                .map(|(i, _)| i)
                .collect();
            self.open.insert(key.clone(), initial);
        }
        let open = self.open.get(&key).cloned().unwrap_or_default();
        let mut acc =
            Accordion::new(self.eid(id, frame)).multiple(self.flag(id, "multiple", frame));
        for (i, s) in sections.iter().enumerate() {
            let title = self.s(*s, "title", frame).unwrap_or_default();
            let is_open = open.contains(&i);
            let body = if is_open {
                Some(self.element(*s, frame, window, cx))
            } else {
                None
            };
            acc = acc.item(move |item| {
                let item = item.title(title).open(is_open);
                match body {
                    Some(b) => item.child(b),
                    None => item,
                }
            });
        }
        let k = key.clone();
        acc = acc.on_toggle_click(cx.listener(move |this, open: &[usize], _, cx| {
            this.open.insert(k.clone(), open.to_vec());
            cx.notify();
        }));
        let class = self.class(id);
        style::apply(div().w_full(), class.as_deref(), cx)
            .child(acc)
            .into_any_element()
    }
}

fn justify<E: Styled>(e: E, j: Option<&str>) -> E {
    match j {
        Some("center") => e.justify_center(),
        Some("end") => e.justify_end(),
        Some("between") => e.justify_between(),
        _ => e,
    }
}
