//! Display components.

use ui::{
    AnyElement, Context, FontWeight, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window,
    component::{
        Icon, IconName, Sizable,
        alert::{Alert, AlertVariant},
        avatar::Avatar,
        chart::{AreaChart, BarChart, LineChart, PieChart},
        h_flex,
        progress::Progress,
        skeleton::Skeleton,
        spinner::Spinner,
        tag::Tag,
        tooltip::Tooltip,
        v_flex,
    },
    div, px,
};
use views_expr::Value;
use views_ir::{Node, NodeId};
use views_runtime::Frame;

use crate::{ViewPane, style};

impl ViewPane {
    pub(crate) fn display(
        &mut self,
        n: &Node,
        frame: &Frame,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = n.id;
        let theme = style::tokens(cx);
        let tone = self.s(id, "tone", frame);
        match n.tag.as_str() {
            "Badge" => {
                let t = self.rt.text(id, frame);
                self.note(id, frame, &t);
                let tag = match tone.as_deref() {
                    Some("info") => Tag::info(),
                    Some("success") => Tag::success(),
                    Some("warning") => Tag::warning(),
                    Some("danger") => Tag::danger(),
                    Some("accent") => Tag::primary(),
                    _ => Tag::secondary(),
                };
                tag.small().child(t).into_any_element()
            }
            "Kbd" => {
                let t = self.rt.text(id, frame);
                self.note(id, frame, &t);
                // Key caps drawn from theme tokens; each key of a chord gets its own cap.
                h_flex()
                    .gap_1()
                    .children(t.split(['-', '+']).filter(|k| !k.is_empty()).map(|k| {
                        div()
                            .px_1p5()
                            .min_w(px(20.))
                            .text_center()
                            .rounded(theme.radius * 0.5)
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.secondary)
                            .text_xs()
                            .font_family(theme.mono_font_family.clone())
                            .child(key_label(k))
                    }))
                    .into_any_element()
            }
            "Stat" => {
                let label = self.s(id, "label", frame).unwrap_or_default();
                let value = self.s(id, "value", frame).unwrap_or_default();
                self.note(id, frame, &value);
                let mut e = v_flex()
                    .gap_1()
                    .min_w_0()
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(label),
                    )
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(
                                tone.as_deref()
                                    .map_or(theme.foreground, |t| style::tone(&theme, t)),
                            )
                            .child(value),
                    );
                if let Some(d) = self.s(id, "delta", frame) {
                    let c = if d.starts_with('-') {
                        theme.danger
                    } else {
                        theme.success
                    };
                    e = e.child(div().text_sm().text_color(c).child(d));
                }
                if let Some(c) = self.s(id, "caption", frame) {
                    e = e.child(div().text_xs().text_color(theme.muted_foreground).child(c));
                }
                e.into_any_element()
            }
            "Table" => self.table(id, frame, cx),
            "List" => {
                let items = self.rt.eval(id, "items", frame);
                let items = items.as_list().unwrap_or(&[]);
                let ordered = self.flag(id, "ordered", frame);
                if items.is_empty() {
                    let empty = self
                        .s(id, "empty", frame)
                        .unwrap_or_else(|| "Nothing here yet.".into());
                    return div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(empty)
                        .into_any_element();
                }
                let max = self.rt.limits().max_list;
                v_flex()
                    .w_full()
                    .children(items.iter().take(max).enumerate().map(|(i, it)| {
                        let (title, desc, badge) = match it {
                            Value::Map(_) => (
                                it.field("title").to_text(),
                                it.field("description").to_text(),
                                it.field("badge").to_text(),
                            ),
                            other => (other.to_text(), String::new(), String::new()),
                        };
                        let mut row = h_flex()
                            .gap_3()
                            .py_2()
                            .border_b_1()
                            .border_color(theme.border)
                            .items_start();
                        row =
                            row.child(div().text_color(theme.muted_foreground).child(if ordered {
                                format!("{}.", i + 1)
                            } else {
                                "•".into()
                            }));
                        let mut body = v_flex().flex_1().gap_0p5().child(title);
                        if !desc.is_empty() {
                            body = body.child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child(desc),
                            );
                        }
                        row = row.child(body);
                        if !badge.is_empty() {
                            row = row.child(Tag::secondary().small().child(badge));
                        }
                        row
                    }))
                    .into_any_element()
            }
            "Progress" => {
                let v = self
                    .num(id, "value", frame)
                    .unwrap_or(0.0)
                    .clamp(0.0, 100.0);
                self.note(id, frame, &views_expr::fmt_num(v));
                let mut p = Progress::new(self.eid(id, frame)).value(v as f32);
                if let Some(t) = tone.as_deref() {
                    p = p.color(style::tone(&theme, t));
                }
                let mut e = v_flex().w_full().gap_1p5();
                if let Some(l) = self.s(id, "label", frame) {
                    e = e.child(
                        h_flex().justify_between().text_sm().child(l).child(
                            div()
                                .text_color(theme.muted_foreground)
                                .child(format!("{}%", v.round())),
                        ),
                    );
                }
                e.child(p).into_any_element()
            }
            "Chart" => self.chart(id, frame, cx),
            "Icon" => {
                let name = self
                    .s(id, "name", frame)
                    .and_then(|n| style::icon(&n))
                    .unwrap_or(IconName::Square);
                let color = self
                    .s(id, "color", frame)
                    .map(|c| style::palette(&theme, &c))
                    .or(tone.as_deref().map(|t| style::tone(&theme, t)))
                    .unwrap_or(theme.foreground);
                let icon = Icon::new(name).text_color(color);
                match self.s(id, "size", frame).as_deref() {
                    Some("sm") => icon.size_3p5(),
                    Some("lg") => icon.size_7(),
                    _ => icon.size_4(),
                }
                .into_any_element()
            }
            "Avatar" => {
                let name = self.s(id, "name", frame).unwrap_or_default();
                let a = Avatar::new().name(name);
                match self.s(id, "size", frame).as_deref() {
                    Some("sm") => a.small(),
                    Some("lg") => a.large(),
                    _ => a,
                }
                .into_any_element()
            }
            "Callout" => {
                let t = self.rt.text(id, frame);
                self.note(id, frame, &t);
                let variant = match tone.as_deref() {
                    Some("success") => AlertVariant::Success,
                    Some("warning") => AlertVariant::Warning,
                    Some("danger") => AlertVariant::Error,
                    Some("info") => AlertVariant::Info,
                    _ => AlertVariant::Default,
                };
                let mut a = Alert::new(self.eid(id, frame), t).with_variant(variant);
                if let Some(title) = self.s(id, "title", frame) {
                    a = a.title(title);
                }
                if let Some(icon) = self.s(id, "icon", frame).and_then(|n| style::icon(&n)) {
                    a = a.icon(Icon::new(icon));
                }
                a.into_any_element()
            }
            "Tooltip" => {
                let text: SharedString = self.s(id, "text", frame).unwrap_or_default().into();
                let kids = self.kids(id, frame, window, cx);
                h_flex()
                    .id(self.eid(id, frame))
                    .children(kids)
                    .tooltip(move |window, cx| Tooltip::new(text.clone()).build(window, cx))
                    .into_any_element()
            }
            "Skeleton" => {
                if let Some(h) = self.s(id, "height", frame) {
                    return Skeleton::new()
                        .w_full()
                        .h(px(views_catalog::size_px(&h)))
                        .rounded(theme.radius)
                        .into_any_element();
                }
                let lines = self.num(id, "lines", frame).unwrap_or(3.0).clamp(1.0, 12.0) as usize;
                v_flex()
                    .w_full()
                    .gap_2()
                    .children((0..lines).map(|i| {
                        Skeleton::new()
                            .h(px(14.))
                            .w(if i + 1 == lines && lines > 1 {
                                ui::relative(0.6)
                            } else {
                                ui::relative(1.)
                            })
                            .rounded(theme.radius)
                    }))
                    .into_any_element()
            }
            "Spinner" => {
                let s = Spinner::new();
                let s = match self.s(id, "size", frame).as_deref() {
                    Some("sm") => s.small(),
                    Some("lg") => s.large(),
                    _ => s,
                };
                let mut e = h_flex().gap_2().child(s);
                if let Some(l) = self.s(id, "label", frame) {
                    e = e.child(div().text_sm().text_color(theme.muted_foreground).child(l));
                }
                e.into_any_element()
            }
            other => {
                // Unknown tag: a visible placeholder, never a blank view.
                let kids = self.kids(id, frame, window, cx);
                v_flex()
                    .gap_1()
                    .p_2()
                    .border_1()
                    .border_dashed()
                    .border_color(theme.border)
                    .rounded(theme.radius)
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(format!("<{other}> is not available")),
                    )
                    .children(kids)
                    .into_any_element()
            }
        }
    }

    fn table(&mut self, id: NodeId, frame: &Frame, cx: &mut Context<Self>) -> AnyElement {
        let theme = style::tokens(cx);
        let rows = self.rt.eval(id, "rows", frame);
        let rows = rows.as_list().unwrap_or(&[]).to_vec();
        let mut cols: Vec<(String, String, String, String)> = Vec::new();
        match self.rt.eval(id, "columns", frame) {
            Value::List(l) => {
                for c in l.iter() {
                    match c {
                        Value::Map(_) => {
                            let key = c.field("key").to_text();
                            let label = c.field("label");
                            cols.push((
                                key.clone(),
                                if label.is_null() {
                                    key
                                } else {
                                    label.to_text()
                                },
                                c.field("format").to_text(),
                                c.field("align").to_text(),
                            ));
                        }
                        other => cols.push((
                            other.to_text(),
                            other.to_text(),
                            String::new(),
                            String::new(),
                        )),
                    }
                }
            }
            _ => {
                if let Some(Value::Map(m)) = rows.first() {
                    cols = m
                        .keys()
                        .map(|k| (k.clone(), k.clone(), String::new(), String::new()))
                        .collect();
                }
            }
        }
        if rows.is_empty() {
            let empty = self
                .s(id, "empty", frame)
                .unwrap_or_else(|| "No rows.".into());
            return div()
                .p_3()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(empty)
                .into_any_element();
        }
        let cell = |text: String, align: &str| {
            let d = div()
                .flex_1()
                .min_w_0()
                .px_3()
                .py_2()
                .truncate()
                .child(text);
            if align == "right" { d.text_right() } else { d }
        };
        let header = h_flex()
            .bg(theme.table_head)
            .text_color(theme.table_head_foreground)
            .text_sm()
            .font_weight(FontWeight::MEDIUM)
            .children(cols.iter().map(|(_, l, _, a)| cell(l.clone(), a)));
        let max = self.rt.limits().max_list;
        let body = rows.iter().take(max).enumerate().map(|(i, r)| {
            let row = h_flex()
                .text_sm()
                .border_t_1()
                .border_color(theme.table_row_border)
                .children(
                    cols.iter()
                        .map(|(k, _, f, a)| cell(format_cell(&r.field(k), f), a)),
                );
            if i % 2 == 1 {
                row.bg(theme.table_even)
            } else {
                row
            }
        });
        self.note(id, frame, &format!("{} rows", rows.len()));
        v_flex()
            .w_full()
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius)
            .overflow_hidden()
            .child(header)
            .children(body)
            .into_any_element()
    }

    fn chart(&mut self, id: NodeId, frame: &Frame, cx: &mut Context<Self>) -> AnyElement {
        let theme = style::tokens(cx);
        let data = self.rt.eval(id, "data", frame);
        let x = self.s(id, "x", frame).unwrap_or_default();
        let y = self.s(id, "y", frame).unwrap_or_default();
        let points: Vec<(SharedString, f64)> = data
            .as_list()
            .unwrap_or(&[])
            .iter()
            .take(self.rt.limits().max_list)
            .map(|r| {
                (
                    SharedString::from(r.field(&x).to_text()),
                    r.field(&y).as_num().unwrap_or(0.0),
                )
            })
            .collect();
        let color = self
            .s(id, "color", frame)
            .map_or(theme.chart[0], |c| style::palette(&theme, &c));
        let h = views_catalog::size_px(self.s(id, "height", frame).as_deref().unwrap_or("md"));
        self.note(id, frame, &format!("{} points", points.len()));
        let chart: AnyElement = match self.s(id, "type", frame).as_deref() {
            Some("line") => LineChart::new(points)
                .appear(false)
                .x(|p| p.0.clone())
                .y(|p| p.1)
                .stroke(color)
                .into_any_element(),
            Some("area") => AreaChart::new(points)
                .appear(false)
                .x(|p| p.0.clone())
                .y(|p| p.1)
                .stroke(color)
                .fill(color.opacity(0.3))
                .into_any_element(),
            Some("pie") => {
                let t = theme.clone();
                let idx: Vec<(usize, SharedString, f64)> = points
                    .into_iter()
                    .enumerate()
                    .map(|(i, (l, v))| (i, l, v))
                    .collect();
                PieChart::new(idx)
                    .appear(false)
                    .value(|p| p.2 as f32)
                    .color(move |p| style::series(&t, p.0))
                    .outer_radius(h * 0.42)
                    .into_any_element()
            }
            _ => BarChart::new(points)
                .appear(false)
                .band(|p| p.0.clone())
                .value(|p| p.1)
                .fill(move |_, _, _, _| color)
                .into_any_element(),
        };
        let mut e = v_flex().w_full().gap_2();
        if let Some(l) = self.s(id, "label", frame) {
            e = e.child(div().text_sm().font_weight(FontWeight::MEDIUM).child(l));
        }
        e.child(div().w_full().h(px(h)).child(chart))
            .into_any_element()
    }
}

fn format_cell(v: &Value, format: &str) -> String {
    match (format, v) {
        ("currency", Value::Num(n)) => views_expr::currency(*n, "USD", None),
        ("number", Value::Num(n)) => views_expr::grouped(*n, if n.fract() == 0.0 { 0 } else { 2 }),
        ("percent", Value::Num(n)) => views_expr::percent(*n, 0),
        ("date", Value::Date(d)) => views_expr::format_date(*d, "MMM d, yyyy"),
        ("date", Value::Str(s)) => views_expr::parse_date(s).map_or_else(
            || s.to_string(),
            |d| views_expr::format_date(d, "MMM d, yyyy"),
        ),
        _ => v.to_text(),
    }
}

fn key_label(k: &str) -> String {
    match k.to_ascii_lowercase().as_str() {
        "cmd" | "command" => "⌘".into(),
        "shift" => "⇧".into(),
        "alt" | "option" => "⌥".into(),
        "ctrl" | "control" => "⌃".into(),
        "enter" | "return" => "↵".into(),
        "escape" | "esc" => "esc".into(),
        other if other.len() == 1 => other.to_uppercase(),
        other => other.to_string(),
    }
}
