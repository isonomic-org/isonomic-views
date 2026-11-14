//! Inputs. Stateful components keep their state entity per node instance; bound values flow
//! in on every render and user changes flow out through `write_bind`.

use ui::{
    AnyElement, AppContext, Context, IntoElement, ParentElement, SharedString, Styled, Window,
    component::{
        ActiveTheme, Disableable, Sizable,
        button::{Button, ButtonVariants},
        checkbox::Checkbox,
        input::{
            Input, InputEvent, InputState, NumberInput, NumberInputEvent, StepAction, Textarea,
            TextareaState,
        },
        link::Link,
        searchable_list::SearchableVec,
        select::SelectState,
        switch::Switch,
        tab::{Tab, TabBar},
        v_flex,
    },
    div,
};
use views_expr::Value;
use views_ir::{Node, NodeId};
use views_runtime::Frame;

mod choice;

pub(crate) use choice::options;

use crate::{
    ViewPane,
    pane::{Held, Slot},
    style,
};

pub type SelectEntity = SelectState<SearchableVec<SharedString>>;

impl ViewPane {
    pub(crate) fn widget(
        &mut self,
        n: &Node,
        frame: &Frame,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = n.id;
        let class = self.class(id);
        let el = match n.tag.as_str() {
            "Button" => self.button(id, frame, cx),
            "Link" => {
                let t = self.rt.text(id, frame);
                self.note(id, frame, &t);
                let f = frame.clone();
                Link::new(self.eid(id, frame))
                    .child(t)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.rt.has(id, "onClick") {
                            this.fire(id, "onClick", &f, Value::Null, cx);
                        } else if let Some(to) = this.s(id, "to", &f) {
                            let _ = this.rt.host().open(&to);
                        }
                    }))
                    .into_any_element()
            }
            "TextField" | "TextArea" => self.text_input(id, frame, n.tag == "TextArea", window, cx),
            "NumberInput" => self.number_input(id, frame, window, cx),
            "Checkbox" | "Switch" => {
                let checked = self.rt.eval(id, "bind", frame).truthy();
                let label = self.s(id, "label", frame).unwrap_or_default();
                self.note(id, frame, &format!("{label}: {checked}"));
                let f = frame.clone();
                let on = cx.listener(move |this, v: &bool, _, cx| {
                    let _ = this.rt.write_bind(id, &f, Value::Bool(*v));
                    this.fire(id, "onChange", &f, Value::Bool(*v), cx);
                });
                let disabled = self.flag(id, "disabled", frame);
                if n.tag == "Checkbox" {
                    Checkbox::new(self.eid(id, frame))
                        .checked(checked)
                        .label(label)
                        .disabled(disabled)
                        .on_click(on)
                        .into_any_element()
                } else {
                    Switch::new(self.eid(id, frame))
                        .checked(checked)
                        .label(label)
                        .disabled(disabled)
                        .on_click(on)
                        .into_any_element()
                }
            }
            "Slider" => self.slider_el(id, frame, window, cx),
            "Select" => self.select_el(id, frame, window, cx),
            "SegmentedControl" => {
                let (labels, values) = options(&self.rt.eval(id, "options", frame));
                let cur = self.rt.eval(id, "bind", frame);
                let sel = values
                    .iter()
                    .position(|v| *v == cur || v.to_text() == cur.to_text())
                    .unwrap_or(0);
                let f = frame.clone();
                TabBar::new(self.eid(id, frame))
                    .segmented()
                    .selected_index(sel)
                    .children(labels.into_iter().map(|l| Tab::new().label(l)))
                    .on_click(cx.listener(move |this, ix: &usize, _, cx| {
                        let (_, values) = options(&this.rt.eval(id, "options", &f));
                        if let Some(v) = values.get(*ix).cloned() {
                            let _ = this.rt.write_bind(id, &f, v.clone());
                            this.fire(id, "onChange", &f, v, cx);
                        }
                    }))
                    .into_any_element()
            }
            "DatePicker" => self.date_el(id, frame, window, cx),
            _ => return self.display(n, frame, window, cx),
        };
        let field = matches!(
            n.tag.as_str(),
            "TextField" | "TextArea" | "NumberInput" | "Select" | "DatePicker" | "Slider"
        );
        let el = match self.s(id, "label", frame).filter(|_| field) {
            Some(l) => v_flex()
                .gap_1p5()
                .w_full()
                .child(div().text_sm().font_weight(ui::FontWeight::MEDIUM).child(l))
                .child(el)
                .into_any_element(),
            None => el,
        };
        // Fields fill the width of a column and share the width of a row; buttons and toggles
        // keep their natural size.
        let in_row = n
            .parent
            .and_then(|p| self.rt.doc().node(p))
            .is_some_and(|p| p.is("Row"));
        let wrap = match (field, in_row) {
            (true, true) => div().flex_1().min_w_0(),
            (true, false) => div().w_full(),
            _ => div(),
        };
        style::apply(wrap, class.as_deref(), cx)
            .child(el)
            .into_any_element()
    }

    fn button(&mut self, id: NodeId, frame: &Frame, cx: &mut Context<Self>) -> AnyElement {
        let label = self.rt.text(id, frame);
        self.note(id, frame, &label);
        let f = frame.clone();
        let mut b = Button::new(self.eid(id, frame))
            .label(label)
            .disabled(self.flag(id, "disabled", frame));
        b = match self.s(id, "variant", frame).as_deref() {
            Some("primary") => b.primary(),
            Some("danger") => b.danger(),
            Some("ghost") => b.ghost(),
            Some("outline") => b.outline(),
            _ => b,
        };
        b = match self.s(id, "size", frame).as_deref() {
            Some("sm") => b.small(),
            Some("lg") => b.large(),
            _ => b,
        };
        if let Some(icon) = self
            .s(id, "icon", frame)
            .filter(|n| views_catalog::ICONS.contains(&n.as_str()))
        {
            b = b.icon(ui::component::Icon::empty().path(format!("icons/{icon}.svg")));
        }
        b.on_click(cx.listener(move |this, _, _, cx| this.fire(id, "onClick", &f, Value::Null, cx)))
            .into_any_element()
    }

    fn text_input(
        &mut self,
        id: NodeId,
        frame: &Frame,
        multi: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let key = self.key(id, frame);
        let want = self.rt.eval(id, "bind", frame).to_text();
        let placeholder = self.s(id, "placeholder", frame).unwrap_or_default();
        let disabled = self.flag(id, "disabled", frame);
        self.note(id, frame, &want);
        if multi {
            if !self.held.contains_key(&key) {
                let rows = self.num(id, "rows", frame).unwrap_or(3.0).clamp(1.0, 20.0) as usize;
                let w = want.clone();
                let state = cx.new(|cx| {
                    TextareaState::new(window, cx)
                        .rows(rows)
                        .placeholder(placeholder.clone())
                        .default_value(w)
                });
                let f = frame.clone();
                let sub =
                    cx.subscribe_in(&state, window, move |this, st, ev: &InputEvent, _, cx| {
                        if let InputEvent::Change = ev {
                            let v = Value::str(st.read(cx).value());
                            this.text_changed(id, &f, v, cx);
                        }
                    });
                self.held.insert(
                    key.clone(),
                    Held {
                        slot: Slot::Area(state),
                        _subs: vec![sub],
                    },
                );
            }
            let Some(Held {
                slot: Slot::Area(state),
                ..
            }) = self.held.get(&key)
            else {
                return div().into_any_element();
            };
            let state = state.clone();
            if state.read(cx).value().as_ref() != want {
                state.update(cx, |s, cx| s.set_value(want.clone(), window, cx));
            }
            return Textarea::new(&state).disabled(disabled).into_any_element();
        }
        if !self.held.contains_key(&key) {
            let w = want.clone();
            let state = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(placeholder.clone())
                    .default_value(w)
            });
            let f = frame.clone();
            let sub = cx.subscribe_in(&state, window, move |this, st, ev: &InputEvent, _, cx| {
                if let InputEvent::Change = ev {
                    let v = Value::str(st.read(cx).value());
                    this.text_changed(id, &f, v, cx);
                }
            });
            self.held.insert(
                key.clone(),
                Held {
                    slot: Slot::Input(state),
                    _subs: vec![sub],
                },
            );
        }
        let Some(Held {
            slot: Slot::Input(state),
            ..
        }) = self.held.get(&key)
        else {
            return div().into_any_element();
        };
        let state = state.clone();
        if state.read(cx).value().as_ref() != want {
            state.update(cx, |s, cx| s.set_value(want.clone(), window, cx));
        }
        Input::new(&state).disabled(disabled).into_any_element()
    }

    fn text_changed(&mut self, id: NodeId, f: &Frame, v: Value, cx: &mut Context<Self>) {
        if self.rt.eval(id, "bind", f) != v {
            let _ = self.rt.write_bind(id, f, v.clone());
            self.fire(id, "onChange", f, v, cx);
        }
    }

    fn number_input(
        &mut self,
        id: NodeId,
        frame: &Frame,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let key = self.key(id, frame);
        let cur = self.rt.eval(id, "bind", frame).as_num();
        let want = cur.map(views_expr::fmt_num).unwrap_or_default();
        if !self.held.contains_key(&key) {
            let w = want.clone();
            let state = cx.new(|cx| InputState::new(window, cx).default_value(w));
            let f = frame.clone();
            let change =
                cx.subscribe_in(&state, window, move |this, st, ev: &InputEvent, _, cx| {
                    if let InputEvent::Change = ev {
                        let text = st.read(cx).value().replace(',', "");
                        if let Ok(n) = text.trim().parse::<f64>() {
                            this.set_number(id, &f, n, cx);
                        }
                    }
                });
            let f = frame.clone();
            let step = cx.subscribe_in(
                &state,
                window,
                move |this, _, ev: &NumberInputEvent, _, cx| {
                    let NumberInputEvent::Step(a) = ev;
                    let s = this.num(id, "step", &f).unwrap_or(1.0);
                    let cur = this.rt.eval(id, "bind", &f).as_num().unwrap_or(0.0);
                    let next = if *a == StepAction::Increment {
                        cur + s
                    } else {
                        cur - s
                    };
                    this.set_number(id, &f, next, cx);
                },
            );
            self.held.insert(
                key.clone(),
                Held {
                    slot: Slot::Number(state),
                    _subs: vec![change, step],
                },
            );
        }
        let Some(Held {
            slot: Slot::Number(state),
            ..
        }) = self.held.get(&key)
        else {
            return div().into_any_element();
        };
        let state = state.clone();
        let shown = state.read(cx).value().replace(',', "");
        if shown.trim().parse::<f64>().ok() != cur && !(cur.is_none() && shown.is_empty()) {
            state.update(cx, |s, cx| s.set_value(want.clone(), window, cx));
        }
        self.note(id, frame, &want);
        let mut e = NumberInput::new(&state);
        if let Some(p) = self.s(id, "prefix", frame) {
            e = e.prefix(div().text_color(cx.theme().muted_foreground).child(p));
        }
        if let Some(sx) = self.s(id, "suffix", frame) {
            e = e.suffix(div().text_color(cx.theme().muted_foreground).child(sx));
        }
        e.disabled(self.flag(id, "disabled", frame))
            .into_any_element()
    }

    fn set_number(&mut self, id: NodeId, f: &Frame, n: f64, cx: &mut Context<Self>) {
        let lo = self.num(id, "min", f).unwrap_or(f64::MIN);
        let hi = self.num(id, "max", f).unwrap_or(f64::MAX);
        let v = Value::Num(n.clamp(lo, hi));
        if self.rt.eval(id, "bind", f) != v {
            let _ = self.rt.write_bind(id, f, v.clone());
            self.fire(id, "onChange", f, v, cx);
        }
    }
}
