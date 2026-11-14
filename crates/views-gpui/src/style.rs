//! Tokens and utility classes mapped onto the design system's theme. No raw values: every
//! color comes from the theme, every size from the spacing scale or the theme's radii.

use ui::{
    App, Hsla, Pixels, Styled,
    component::{ActiveTheme, Theme},
    px,
};
use views_catalog::{Utility, parse_class};

/// One spacing step in pixels.
pub const UNIT: f32 = 4.0;

pub fn step(n: u8) -> Pixels {
    px(n as f32 * UNIT)
}

pub fn size(n: u16) -> Pixels {
    px(n as f32 * UNIT)
}

/// Color of a semantic tone.
pub fn tone(theme: &Theme, name: &str) -> Hsla {
    match name {
        "info" => theme.info,
        "success" => theme.success,
        "warning" => theme.warning,
        "danger" => theme.danger,
        "accent" => theme.primary,
        "muted" => theme.muted_foreground,
        _ => theme.foreground,
    }
}

/// Color of a palette name.
pub fn palette(theme: &Theme, name: &str) -> Hsla {
    match name {
        "blue" => theme.blue,
        "green" => theme.green,
        "red" => theme.red,
        "yellow" => theme.yellow,
        "magenta" => theme.magenta,
        "cyan" => theme.cyan,
        "gray" => theme.muted_foreground,
        "chart-2" => theme.chart_2,
        "chart-3" => theme.chart_3,
        "chart-4" => theme.chart_4,
        "chart-5" => theme.chart_5,
        _ => theme.chart_1,
    }
}

/// The nth series color.
pub fn series(theme: &Theme, i: usize) -> Hsla {
    [
        theme.chart_1,
        theme.chart_2,
        theme.chart_3,
        theme.chart_4,
        theme.chart_5,
    ][i % 5]
}

/// Apply a class attribute to an element. Unknown words were reported by the validator and
/// are ignored here.
pub fn apply<E: Styled>(mut e: E, class: Option<&str>, cx: &App) -> E {
    let Some(class) = class else { return e };
    let theme = cx.theme();
    let (utils, _) = parse_class(class);
    for u in utils {
        e = match u {
            Utility::Gap(n) => e.gap(step(n)),
            Utility::GapX(n) => e.gap_x(step(n)),
            Utility::GapY(n) => e.gap_y(step(n)),
            Utility::P(n) => e.p(step(n)),
            Utility::Px(n) => e.px(step(n)),
            Utility::Py(n) => e.py(step(n)),
            Utility::Pt(n) => e.pt(step(n)),
            Utility::Pb(n) => e.pb(step(n)),
            Utility::WFull => e.w_full(),
            Utility::W(n) => e.w(size(n)),
            Utility::MaxW(n) => e.max_w(size(n)),
            Utility::HFull => e.h_full(),
            Utility::H(n) => e.h(size(n)),
            Utility::Flex1 => e.flex_1(),
            Utility::Grow => e.flex_grow_1(),
            Utility::Shrink0 => e.flex_shrink_0(),
            Utility::Wrap => e.flex_wrap(),
            Utility::ItemsStart => e.items_start(),
            Utility::ItemsCenter => e.items_center(),
            Utility::ItemsEnd => e.items_end(),
            Utility::ItemsStretch => e.items_stretch(),
            Utility::JustifyStart => e.justify_start(),
            Utility::JustifyCenter => e.justify_center(),
            Utility::JustifyEnd => e.justify_end(),
            Utility::JustifyBetween => e.justify_between(),
            Utility::GridCols(n) => e.grid().grid_cols(n as u16),
            Utility::ColSpan(n) => e.col_span(n as u16),
            Utility::TextSize(s) => text_size(e, s),
            Utility::Font(w) => weight(e, w),
            Utility::TextLeft => e.text_left(),
            Utility::TextCenter => e.text_center(),
            Utility::TextRight => e.text_right(),
            Utility::TextMuted => e.text_color(theme.muted_foreground),
            Utility::Truncate => e.truncate(),
            Utility::Bg(b) => match b {
                "muted" => e.bg(theme.muted),
                "accent" => e.bg(theme.accent),
                _ => e.bg(theme.secondary),
            },
            Utility::Border => e.border_1().border_color(theme.border),
            Utility::Rounded(r) => match r {
                "none" => e.rounded_none(),
                "sm" => e.rounded(theme.radius * 0.5),
                "lg" => e.rounded(theme.radius_lg),
                "xl" => e.rounded(theme.radius_lg * 1.5),
                "full" => e.rounded_full(),
                _ => e.rounded(theme.radius),
            },
            Utility::MinW0 => e.min_w_0(),
        };
    }
    e
}

pub fn text_size<E: Styled>(e: E, s: &str) -> E {
    match s {
        "xs" => e.text_xs(),
        "sm" => e.text_sm(),
        "lg" => e.text_lg(),
        "xl" => e.text_xl(),
        "2xl" => e.text_2xl(),
        _ => e.text_base(),
    }
}

pub fn weight<E: Styled>(e: E, w: &str) -> E {
    match w {
        "medium" => e.font_weight(ui::FontWeight::MEDIUM),
        "semibold" => e.font_weight(ui::FontWeight::SEMIBOLD),
        "bold" => e.font_weight(ui::FontWeight::BOLD),
        _ => e.font_weight(ui::FontWeight::NORMAL),
    }
}
