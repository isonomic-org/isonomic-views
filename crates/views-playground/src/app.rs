use std::{rc::Rc, time::Duration};

use ui::{
    AppContext, Context, Entity, FontWeight, IntoElement, ParentElement, Render, SharedString,
    Styled, Subscription, Task, Window,
    component::{
        Disableable, Sizable,
        button::{Button, ButtonVariants},
        h_flex,
        input::{InputEvent, Textarea, TextareaState},
        v_flex,
    },
    div,
};
use views_gpui::ViewPane;
use views_ir::{Limits, Severity, markup::line_col};
use views_runtime::MemoryStore;

/// Examples offered in the playground, from the corpus.
pub const EXAMPLES: &[(&str, &str)] = &[
    (
        "Seat cost",
        include_str!("../../../corpus/seat-cost-calculator.view"),
    ),
    ("To-do", include_str!("../../../corpus/todo-list.view")),
    (
        "Sales",
        include_str!("../../../corpus/sales-dashboard.view"),
    ),
    ("Quiz", include_str!("../../../corpus/capitals-quiz.view")),
];

/// Characters fed per tick in replay mode, and the tick length.
const REPLAY_CHUNK: usize = 6;
const REPLAY_TICK: Duration = Duration::from_millis(16);

pub struct Playground {
    editor: Entity<TextareaState>,
    pane: Entity<ViewPane>,
    replay: Option<Task<()>>,
    _subs: Vec<Subscription>,
}

impl Playground {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let src = EXAMPLES[0].1;
        let editor = cx.new(|cx| TextareaState::new(window, cx).rows(36).default_value(src));
        let pane = cx.new(|_| {
            ViewPane::with_host(
                src,
                Rc::new(views_gpui::fixtures::host()),
                MemoryStore::handle(),
                Limits::default(),
            )
        });
        let sub = cx.subscribe_in(&editor, window, |this, editor, ev: &InputEvent, _, cx| {
            if let InputEvent::Change = ev {
                if this.replay.is_some() {
                    return;
                }
                let text = editor.read(cx).value().to_string();
                this.pane.update(cx, |p, cx| {
                    p.set_source(&text, cx);
                });
                cx.notify();
            }
        });
        Playground {
            editor,
            pane,
            replay: None,
            _subs: vec![sub],
        }
    }

    fn load(&mut self, src: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        self.replay = None;
        self.editor.update(cx, |e, cx| e.set_value(src, window, cx));
        self.pane.update(cx, |p, cx| {
            p.set_source(src, cx);
        });
    }

    /// Feed the current source in from empty, a few characters per frame.
    fn start_replay(&mut self, cx: &mut Context<Self>) {
        let src = self.editor.read(cx).value().to_string();
        self.pane.update(cx, |p, cx| {
            p.set_source("", cx);
        });
        let pane = self.pane.clone();
        self.replay = Some(cx.spawn(async move |this, cx| {
            let chars: Vec<char> = src.chars().collect();
            for chunk in chars.chunks(REPLAY_CHUNK) {
                cx.background_executor().timer(REPLAY_TICK).await;
                let s: String = chunk.iter().collect();
                pane.update(cx, |p, cx| {
                    p.append(&s, cx);
                });
            }
            let _ = this.update(cx, |this, cx| {
                this.replay = None;
                cx.notify();
            });
        }));
        cx.notify();
    }
}

impl Render for Playground {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = views_gpui::style::tokens(cx);
        let pane = self.pane.read(cx);
        let src = pane.source().to_string();
        let diags = pane.diagnostics();
        let replaying = self.replay.is_some();
        let toolbar = h_flex()
            .gap_2()
            .p_2()
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .px_2()
                    .child("Views playground"),
            )
            .children(EXAMPLES.iter().enumerate().map(|(i, (name, src))| {
                Button::new(("example", i))
                    .small()
                    .ghost()
                    .label(*name)
                    .on_click(cx.listener(move |this, _, window, cx| this.load(src, window, cx)))
            }))
            .child(div().flex_1())
            .child(
                Button::new("replay")
                    .small()
                    .primary()
                    .label(if replaying {
                        "Streaming…"
                    } else {
                        "Replay as a stream"
                    })
                    .disabled(replaying)
                    .on_click(cx.listener(|this, _, _, cx| this.start_replay(cx))),
            );
        let diag_list = v_flex()
            .h(ui::px(140.))
            .p_2()
            .gap_1()
            .border_t_1()
            .border_color(theme.border)
            .text_xs()
            .font_family(theme.mono_font_family.clone())
            .child(
                div()
                    .text_color(theme.muted_foreground)
                    .child(format!("{} diagnostics", diags.len())),
            )
            .children(diags.into_iter().take(40).map(|d| {
                let (line, col) = d.span.map_or((0, 0), |s| line_col(&src, s.start));
                let color = match d.severity {
                    Severity::Error => theme.danger,
                    Severity::Warning => theme.warning,
                    Severity::Info => theme.muted_foreground,
                };
                let hint = d.hint.map(|h| format!("  ({h})")).unwrap_or_default();
                div().text_color(color).child(SharedString::from(format!(
                    "{line}:{col} {} {}{hint}",
                    d.code, d.message
                )))
            }));
        v_flex()
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(toolbar)
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .items_start()
                    .child(
                        div()
                            .w(ui::relative(0.42))
                            .h_full()
                            .p_2()
                            .border_r_1()
                            .border_color(theme.border)
                            .font_family(theme.mono_font_family.clone())
                            .text_sm()
                            .child(Textarea::new(&self.editor).h_full()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .overflow_hidden()
                            .child(self.pane.clone()),
                    ),
            )
            .child(diag_list)
    }
}
