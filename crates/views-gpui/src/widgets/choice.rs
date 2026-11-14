//! Choice inputs: sliders, selects, date pickers and option lists.

use ui::{
    AnyElement, AppContext, Context, IntoElement, ParentElement, SharedString, Styled, Window,
    component::{
        IndexPath,
        date_picker::{DatePicker, DatePickerEvent, DatePickerState},
        searchable_list::SearchableVec,
        select::{Select, SelectEvent, SelectState},
        slider::{Slider, SliderEvent, SliderState},
    },
    div,
};
use views_expr::Value;
use views_ir::NodeId;
use views_runtime::Frame;

use crate::{
    ViewPane,
    pane::{Held, Slot},
};

impl ViewPane {
    pub(super) fn slider_el(
        &mut self,
        id: NodeId,
        frame: &Frame,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let key = self.key(id, frame);
        let (lo, hi) = (
            self.num(id, "min", frame).unwrap_or(0.0),
            self.num(id, "max", frame).unwrap_or(100.0),
        );
        let step = self.num(id, "step", frame).unwrap_or(1.0);
        let cur = self
            .rt
            .eval(id, "bind", frame)
            .as_num()
            .unwrap_or(lo)
            .clamp(lo, hi);
        if !self.held.contains_key(&key) {
            let state = cx.new(|_| {
                SliderState::new()
                    .min(lo as f32)
                    .max(hi as f32)
                    .step(step as f32)
                    .default_value(cur as f32)
            });
            let f = frame.clone();
            let sub = cx.subscribe_in(&state, window, move |this, _, ev: &SliderEvent, _, cx| {
                let (SliderEvent::Change(v) | SliderEvent::Release(v)) = ev;
                let v = Value::Num(v.start() as f64);
                if this.rt.eval(id, "bind", &f) != v {
                    let _ = this.rt.write_bind(id, &f, v.clone());
                    this.fire(id, "onChange", &f, v, cx);
                }
            });
            self.held.insert(
                key.clone(),
                Held {
                    slot: Slot::Slider(state),
                    _subs: vec![sub],
                },
            );
        }
        let Some(Held {
            slot: Slot::Slider(state),
            ..
        }) = self.held.get(&key)
        else {
            return div().into_any_element();
        };
        let state = state.clone();
        if (state.read(cx).value().start() as f64 - cur).abs() > f64::EPSILON {
            state.update(cx, |s, cx| s.set_value(cur as f32, window, cx));
        }
        let text = views_expr::fmt_num(cur);
        self.note(id, frame, &text);
        ui::component::h_flex()
            .w_full()
            .gap_3()
            .child(
                div()
                    .flex_1()
                    .child(Slider::new(&state).disabled(self.flag(id, "disabled", frame))),
            )
            .child(
                div()
                    .min_w(ui::px(40.))
                    .text_sm()
                    .text_right()
                    .text_color(crate::style::tokens(cx).muted_foreground)
                    .child(text),
            )
            .into_any_element()
    }

    pub(super) fn select_el(
        &mut self,
        id: NodeId,
        frame: &Frame,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let key = self.key(id, frame);
        let (labels, values) = options(&self.rt.eval(id, "options", frame));
        let cur = self.rt.eval(id, "bind", frame);
        let sel = values
            .iter()
            .position(|v| *v == cur || v.to_text() == cur.to_text());
        let stale = match self.held.get(&key) {
            Some(Held {
                slot: Slot::Select(_, v),
                ..
            }) => *v != values,
            _ => true,
        };
        if stale {
            let items = SearchableVec::new(
                labels
                    .iter()
                    .map(|l| SharedString::from(l.clone()))
                    .collect::<Vec<_>>(),
            );
            let state = cx.new(|cx| SelectState::new(items, sel.map(IndexPath::new), window, cx));
            let f = frame.clone();
            let sub = cx.subscribe_in(
                &state,
                window,
                move |this, _, ev: &SelectEvent<SearchableVec<SharedString>>, _, cx| {
                    let SelectEvent::Confirm(choice) = ev;
                    let (labels, values) = options(&this.rt.eval(id, "options", &f));
                    let v = choice
                        .as_ref()
                        .and_then(|c| labels.iter().position(|l| l.as_str() == c.as_ref()))
                        .and_then(|i| values.get(i).cloned())
                        .unwrap_or_default();
                    let _ = this.rt.write_bind(id, &f, v.clone());
                    this.fire(id, "onChange", &f, v, cx);
                },
            );
            self.held.insert(
                key.clone(),
                Held {
                    slot: Slot::Select(state, values.clone()),
                    _subs: vec![sub],
                },
            );
        }
        let Some(Held {
            slot: Slot::Select(state, _),
            ..
        }) = self.held.get(&key)
        else {
            return div().into_any_element();
        };
        let state = state.clone();
        if state.read(cx).selected_index(cx).map(|i| i.row) != sel
            && let Some(i) = sel
        {
            state.update(cx, |s, cx| {
                s.set_selected_index(Some(IndexPath::new(i)), window, cx)
            });
        }
        self.note(
            id,
            frame,
            &sel.and_then(|i| labels.get(i).cloned()).unwrap_or_default(),
        );
        let mut e = Select::new(&state).disabled(self.flag(id, "disabled", frame));
        if let Some(p) = self.s(id, "placeholder", frame) {
            e = e.placeholder(p);
        }
        e.into_any_element()
    }

    pub(super) fn date_el(
        &mut self,
        id: NodeId,
        frame: &Frame,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let key = self.key(id, frame);
        let cur = match self.rt.eval(id, "bind", frame) {
            Value::Date(d) => Some(d),
            Value::Str(s) => views_expr::parse_date(&s),
            _ => None,
        };
        if !self.held.contains_key(&key) {
            let state = cx.new(|cx| DatePickerState::new(window, cx));
            let f = frame.clone();
            let sub = cx.subscribe_in(
                &state,
                window,
                move |this, _, ev: &DatePickerEvent, _, cx| {
                    let DatePickerEvent::Change(dt) = ev;
                    let v = crate::dates::from_picker(dt).map_or(Value::Null, Value::Date);
                    if this.rt.eval(id, "bind", &f) != v {
                        let _ = this.rt.write_bind(id, &f, v.clone());
                        this.fire(id, "onChange", &f, v, cx);
                    }
                },
            );
            self.held.insert(
                key.clone(),
                Held {
                    slot: Slot::Date(state),
                    _subs: vec![sub],
                },
            );
        }
        let Some(Held {
            slot: Slot::Date(state),
            ..
        }) = self.held.get(&key)
        else {
            return div().into_any_element();
        };
        let state = state.clone();
        if let Some(d) = cur
            && crate::dates::of_state(state.read(cx)) != Some(d)
        {
            state.update(cx, |s, cx| crate::dates::set_state(s, d, window, cx));
        }
        self.note(
            id,
            frame,
            &cur.map(views_expr::iso_date).unwrap_or_default(),
        );
        let mut e = DatePicker::new(&state);
        if let Some(p) = self.s(id, "placeholder", frame) {
            e = e.placeholder(p);
        }
        e.into_any_element()
    }
}

/// Labels and values of an `options` prop: texts, or records with label and value.
pub(crate) fn options(v: &Value) -> (Vec<String>, Vec<Value>) {
    let mut labels = Vec::new();
    let mut values = Vec::new();
    for o in v.as_list().unwrap_or(&[]) {
        match o {
            Value::Map(_) => {
                let val = o.field("value");
                let label = o.field("label");
                labels.push(if label.is_null() {
                    val.to_text()
                } else {
                    label.to_text()
                });
                values.push(if val.is_null() { label } else { val });
            }
            other => {
                labels.push(other.to_text());
                values.push(other.clone());
            }
        }
    }
    (labels, values)
}
