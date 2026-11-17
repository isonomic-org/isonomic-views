// Copyright 2026 Anud Inc.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Headless UI tests for the renderer: live bindings, scopes across viewers, state that
//! survives edits and streaming, and the render-time limit.

use std::{cell::Cell, rc::Rc, sync::Arc};

use ui::{
    AnyWindowHandle, AppContext, Bounds, Entity, HeadlessAppContext, WindowBounds, WindowOptions,
    assets::Assets,
    component::{
        input::InputEvent,
        slider::{SliderEvent, SliderValue},
    },
    px, size,
    test::TestWindowExt,
};
use views_expr::Value;
use views_gpui::{FrameClock, ViewPane};
use views_ir::{Edit, Limits};
use views_runtime::{DenyAll, Frame, MemoryStore, SharedHandle};

const CALC: &str = r#"<View version="0.1" shared={{ seats: 25, price: 49 }} local={{ clicks: 0 }}>
  <Col class="gap-3 p-4">
    <Heading>Seat cost calculator</Heading>
    <Slider bind={shared.seats} min={1} max={500} />
    <TextField bind={local.note} placeholder="Note" />
    <Stat label="Annual cost" value={currency(shared.seats * shared.price * 12)} />
    <Text>Clicks: {local.clicks}</Text>
    <Button onClick={set(shared.price, shared.price + 1); set(local.clicks, local.clicks + 1)}>Raise price</Button>
  </Col>
</View>"#;

fn context() -> HeadlessAppContext {
    let mut cx = HeadlessAppContext::with_platform(
        ui::platform::current_platform(true).text_system(),
        Arc::new(Assets),
        ui::platform::current_headless_renderer,
    );
    cx.update(ui::init);
    cx
}

fn open(cx: &mut HeadlessAppContext, pane: ViewPane) -> (AnyWindowHandle, Entity<ViewPane>) {
    let (h, e) = cx
        .update(|cx| {
            ui::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Default::default(),
                        size: size(px(720.), px(900.)),
                    })),
                    focus: false,
                    show: false,
                    ..Default::default()
                },
                cx,
                |_, cx| cx.new(|_| pane),
            )
        })
        .expect("window");
    frame(cx, h);
    (h, e)
}

fn frame(cx: &mut HeadlessAppContext, h: AnyWindowHandle) {
    cx.update_window(h, |_, window, cx| window.render_frame(cx))
        .expect("frame");
}

fn shown(cx: &mut HeadlessAppContext, pane: &Entity<ViewPane>, tag: &str) -> String {
    cx.update(|cx| {
        let p = pane.read(cx);
        p.find(tag)
            .and_then(|id| p.shown(id))
            .unwrap_or_default()
            .to_string()
    })
}

fn pane_with(store: SharedHandle) -> ViewPane {
    ViewPane::with_host(CALC, Rc::new(DenyAll), store, Limits::default())
}

#[test]
fn moving_the_slider_updates_the_stat_in_the_same_frame() {
    let mut cx = context();
    let (h, pane) = open(&mut cx, ViewPane::new(CALC));
    assert_eq!(shown(&mut cx, &pane, "Stat"), "$14,700.00");
    let slider = cx.update(|cx| {
        let p = pane.read(cx);
        p.slider(p.find("Slider").expect("slider"))
            .expect("slider state")
    });
    cx.update_window(h, |_, window, cx| {
        slider.update(cx, |s, cx| {
            s.set_value(200.0, window, cx);
            cx.emit(SliderEvent::Change(SliderValue::Single(200.0)));
        });
    })
    .expect("drag");
    frame(&mut cx, h);
    // `shown` is rebuilt every frame, so both values come from the same frame.
    assert_eq!(shown(&mut cx, &pane, "Slider"), "200");
    assert_eq!(shown(&mut cx, &pane, "Stat"), "$117,600.00");
    let seats = cx.update(|cx| {
        pane.read(cx)
            .runtime()
            .get(&["shared".into(), "seats".into()])
    });
    assert_eq!(seats, Value::Num(200.0));
}

#[test]
fn local_is_per_viewer_and_shared_is_common() {
    let mut cx = context();
    let store = MemoryStore::handle();
    let (ha, a) = open(&mut cx, pane_with(store.clone()));
    let (hb, b) = open(&mut cx, pane_with(store.clone()));
    cx.update(|cx| {
        a.update(cx, |p, cx| {
            let id = p.find("Button").expect("button");
            p.runtime()
                .dispatch(id, "onClick", &Frame::root(), Value::Null);
            cx.notify();
        })
    });
    frame(&mut cx, ha);
    cx.update(|cx| b.update(cx, |_, cx| cx.notify()));
    frame(&mut cx, hb);
    assert_eq!(shown(&mut cx, &a, "Stat"), "$15,000.00");
    assert_eq!(
        shown(&mut cx, &b, "Stat"),
        "$15,000.00",
        "shared reaches the other viewer"
    );
    assert_eq!(shown(&mut cx, &a, "Text"), "Clicks: 1");
    assert_eq!(
        shown(&mut cx, &b, "Text"),
        "Clicks: 0",
        "local stays with its viewer"
    );
}

#[test]
fn component_state_survives_an_edit_elsewhere() {
    let mut cx = context();
    let (h, pane) = open(&mut cx, ViewPane::new(CALC));
    let (input, slider) = cx.update(|cx| {
        let p = pane.read(cx);
        (
            p.input(p.find("TextField").expect("field")).expect("input"),
            p.slider(p.find("Slider").expect("s")).expect("slider"),
        )
    });
    // Typing: the input changes and emits its change event.
    cx.update_window(h, |_, window, cx| {
        input.update(cx, |s, cx| {
            s.set_value("draft text", window, cx);
            cx.emit(InputEvent::Change);
        })
    })
    .expect("type");
    frame(&mut cx, h);
    let count = cx.update(|cx| pane.read(cx).state_count());
    let patches = cx
        .update(|cx| {
            pane.update(cx, |p, cx| {
                p.apply_edits(
                    &[Edit {
                        find: "Seat cost calculator".into(),
                        replace: "Seat pricing".into(),
                    }],
                    cx,
                )
            })
        })
        .expect("edit");
    assert_eq!(patches.iter().filter(|p| p.is_visible()).count(), 1);
    frame(&mut cx, h);
    let (input2, slider2, count2) = cx.update(|cx| {
        let p = pane.read(cx);
        (
            p.input(p.find("TextField").expect("f")).expect("i"),
            p.slider(p.find("Slider").expect("s")).expect("s"),
            p.state_count(),
        )
    });
    assert_eq!(
        input.entity_id(),
        input2.entity_id(),
        "the text field keeps its state entity"
    );
    assert_eq!(
        slider.entity_id(),
        slider2.entity_id(),
        "the slider keeps its state entity"
    );
    assert_eq!(count, count2);
    assert_eq!(
        cx.update(|cx| input2.read(cx).value().to_string()),
        "draft text"
    );
    assert_eq!(shown(&mut cx, &pane, "Heading"), "Seat pricing");
}

#[test]
fn every_streamed_prefix_renders() {
    let mut cx = context();
    let (h, pane) = open(&mut cx, ViewPane::new(""));
    let mut ids = Vec::new();
    for chunk in CALC.as_bytes().chunks(5) {
        let s = std::str::from_utf8(chunk).expect("ascii");
        cx.update(|cx| pane.update(cx, |p, cx| p.append(s, cx)));
        frame(&mut cx, h);
        if let Some(id) = cx.update(|cx| pane.read(cx).find("Slider")) {
            ids.push(id);
        }
    }
    assert!(
        ids.windows(2).all(|w| w[0] == w[1]),
        "the slider keeps one ID while streaming"
    );
    assert_eq!(shown(&mut cx, &pane, "Stat"), "$14,700.00");
    assert!(cx.update(|cx| pane.read(cx).diagnostics().is_empty()));
}

struct FakeClock(Rc<Cell<u64>>);

impl FrameClock for FakeClock {
    fn start(&mut self) {
        self.0.set(0);
    }

    fn elapsed_us(&self) -> u64 {
        // Every check costs a millisecond of pretend time.
        self.0.set(self.0.get() + 1_000);
        self.0.get()
    }
}

#[test]
fn the_render_time_limit_cuts_a_slow_view() {
    let mut cx = context();
    let src = format!(
        "<View version=\"0.1\">{}</View>",
        "<Text>row</Text>".repeat(40)
    );
    let mut pane = ViewPane::with_host(
        &src,
        Rc::new(DenyAll),
        MemoryStore::handle(),
        Limits {
            max_render_us: 8_000,
            ..Limits::default()
        },
    );
    pane.set_clock(Box::new(FakeClock(Rc::new(Cell::new(0)))));
    let (_, pane) = open(&mut cx, pane);
    let (truncated, drawn) = cx.update(|cx| {
        let p = pane.read(cx);
        let doc_texts = p
            .runtime()
            .doc()
            .nodes
            .values()
            .filter(|n| n.is("Text"))
            .count();
        let drawn = p
            .runtime()
            .doc()
            .nodes
            .values()
            .filter(|n| n.is("Text") && p.shown(n.id).is_some())
            .count();
        assert_eq!(doc_texts, 40);
        (p.truncated(), drawn)
    });
    assert!(truncated);
    assert!(drawn < 10, "drew {drawn} of 40 before the budget ran out");
}

#[test]
fn an_unknown_tag_is_a_visible_placeholder() {
    let mut cx = context();
    let (_, pane) = open(
        &mut cx,
        ViewPane::new("<View version=\"0.1\"><Hologram>hi</Hologram><Text>after</Text></View>"),
    );
    assert_eq!(shown(&mut cx, &pane, "Text"), "after");
    let codes: Vec<String> = cx.update(|cx| {
        pane.read(cx)
            .diagnostics()
            .into_iter()
            .map(|d| d.code)
            .collect()
    });
    assert!(codes.contains(&"unknown-component".to_string()));
}
