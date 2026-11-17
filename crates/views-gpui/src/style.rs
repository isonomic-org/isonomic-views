// Copyright 2026 Anud Inc.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tokens and utility classes mapped onto the design system's theme. No raw values: every
//! color comes from the theme, every size from the spacing scale or the theme's radii.
//!
//! This file is the only place that reads the component library's theme. Everything else in
//! the renderer draws with [`Tokens`], so a theme rename in the library changes [`tokens`]
//! and nothing else.

use ui::{
    App, Hsla, Pixels, SharedString, Styled,
    component::{ActiveTheme, IconName},
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

/// The theme values the renderer draws with.
#[derive(Clone, Debug)]
pub struct Tokens {
    pub background: Hsla,
    pub foreground: Hsla,
    pub muted: Hsla,
    pub muted_foreground: Hsla,
    pub secondary: Hsla,
    pub accent: Hsla,
    pub border: Hsla,
    pub primary: Hsla,
    pub info: Hsla,
    pub success: Hsla,
    pub warning: Hsla,
    pub danger: Hsla,
    pub table_head: Hsla,
    pub table_head_foreground: Hsla,
    pub table_row_border: Hsla,
    pub table_even: Hsla,
    pub chart: [Hsla; 5],
    pub blue: Hsla,
    pub green: Hsla,
    pub red: Hsla,
    pub yellow: Hsla,
    pub magenta: Hsla,
    pub cyan: Hsla,
    pub radius: Pixels,
    pub radius_lg: Pixels,
    pub font_family: SharedString,
    pub mono_font_family: SharedString,
}

/// The current theme as tokens.
pub fn tokens(cx: &App) -> Tokens {
    let t = cx.theme();
    Tokens {
        background: t.background,
        foreground: t.foreground,
        muted: t.muted,
        muted_foreground: t.muted_foreground,
        secondary: t.secondary,
        accent: t.accent,
        border: t.border,
        primary: t.primary,
        info: t.info,
        success: t.success,
        warning: t.warning,
        danger: t.danger,
        table_head: t.table_head,
        table_head_foreground: t.table_head_foreground,
        table_row_border: t.table_row_border,
        table_even: t.table_even,
        chart: [t.chart_1, t.chart_2, t.chart_3, t.chart_4, t.chart_5],
        blue: t.blue,
        green: t.green,
        red: t.red,
        yellow: t.yellow,
        magenta: t.magenta,
        cyan: t.cyan,
        radius: t.radius,
        radius_lg: t.radius_lg,
        font_family: t.font_family.clone(),
        mono_font_family: t.mono_font_family.clone(),
    }
}

/// Color of a semantic tone.
pub fn tone(t: &Tokens, name: &str) -> Hsla {
    match name {
        "info" => t.info,
        "success" => t.success,
        "warning" => t.warning,
        "danger" => t.danger,
        "accent" => t.primary,
        "muted" => t.muted_foreground,
        _ => t.foreground,
    }
}

/// Color of a palette name.
pub fn palette(t: &Tokens, name: &str) -> Hsla {
    match name {
        "blue" => t.blue,
        "green" => t.green,
        "red" => t.red,
        "yellow" => t.yellow,
        "magenta" => t.magenta,
        "cyan" => t.cyan,
        "gray" => t.muted_foreground,
        "chart-2" => t.chart[1],
        "chart-3" => t.chart[2],
        "chart-4" => t.chart[3],
        "chart-5" => t.chart[4],
        _ => t.chart[0],
    }
}

/// The nth series color.
pub fn series(t: &Tokens, i: usize) -> Hsla {
    t.chart[i % 5]
}

/// The library icon that draws a catalog icon name, or None for a name outside the catalog.
/// This table is the only place catalog icon names meet the library's icon set.
pub fn icon(name: &str) -> Option<IconName> {
    use IconName as I;
    Some(match name {
        "arrow-down" => I::ArrowDown,
        "arrow-left" => I::ArrowLeft,
        "arrow-right" => I::ArrowRight,
        "arrow-up" => I::ArrowUp,
        "asterisk" => I::Asterisk,
        "ban" => I::Ban,
        "battery" => I::Battery,
        "bell" => I::Bell,
        "book-open" => I::BookOpen,
        "bot" => I::Bot,
        "building-2" => I::Building2,
        "calendar" => I::Calendar,
        "chart-pie" => I::ChartPie,
        "check" => I::Check,
        "chevron-down" => I::ChevronDown,
        "chevron-left" => I::ChevronLeft,
        "chevron-right" => I::ChevronRight,
        "chevron-up" => I::ChevronUp,
        "circle-alert" => I::CircleAlert,
        "circle-check" => I::CircleCheck,
        "circle-user" => I::CircleUser,
        "circle-x" => I::CircleX,
        "copy" => I::Copy,
        "cpu" => I::Cpu,
        "ellipsis" => I::Ellipsis,
        "external-link" => I::ExternalLink,
        "eye" => I::Eye,
        "eye-off" => I::EyeOff,
        "file" => I::File,
        "file-text" => I::FileText,
        "folder" => I::Folder,
        "folder-open" => I::FolderOpen,
        "globe" => I::Globe,
        "hard-drive" => I::HardDrive,
        "heart" => I::Heart,
        "inbox" => I::Inbox,
        "info" => I::Info,
        "layout-dashboard" => I::LayoutDashboard,
        "loader" => I::Loader,
        "map" => I::Map,
        "maximize" => I::Maximize,
        "mic" => I::Mic,
        "minimize" => I::Minimize,
        "minus" => I::Minus,
        "moon" => I::Moon,
        "network" => I::Network,
        "palette" => I::Palette,
        "pause" => I::Pause,
        "play" => I::Play,
        "plus" => I::Plus,
        "refresh-cw" => I::RefreshCw,
        "rotate-cw" => I::RotateCw,
        "search" => I::Search,
        "settings" => I::Settings,
        "square" => I::Square,
        "square-terminal" => I::SquareTerminal,
        "star" => I::Star,
        "star-fill" => I::StarFill,
        "sun" => I::Sun,
        "thumbs-down" => I::ThumbsDown,
        "thumbs-up" => I::ThumbsUp,
        "triangle-alert" => I::TriangleAlert,
        "undo" => I::Undo,
        "user" => I::User,
        _ => return None,
    })
}

/// Apply a class attribute to an element. Unknown words were reported by the validator and
/// are ignored here.
pub fn apply<E: Styled>(mut e: E, class: Option<&str>, cx: &App) -> E {
    let Some(class) = class else { return e };
    let theme = tokens(cx);
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

#[cfg(test)]
mod tests {
    #[test]
    fn every_catalog_icon_has_a_library_icon() {
        let missing: Vec<&str> = views_catalog::ICONS
            .iter()
            .copied()
            .filter(|n| super::icon(n).is_none())
            .collect();
        assert!(missing.is_empty(), "no library icon for {missing:?}");
    }
}
