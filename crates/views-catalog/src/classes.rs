// Copyright 2026 Anud Inc.
// SPDX-License-Identifier: GPL-3.0-or-later

//! The utility-class subset: layout and spacing only, token values only.

use crate::tokens::{SIZES, SPACING};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Utility {
    Gap(u8),
    GapX(u8),
    GapY(u8),
    P(u8),
    Px(u8),
    Py(u8),
    Pt(u8),
    Pb(u8),
    /// `w-full`
    WFull,
    /// `w-<n>` in base units
    W(u16),
    /// `max-w-<n>` in base units
    MaxW(u16),
    HFull,
    H(u16),
    /// `flex-1`
    Flex1,
    Grow,
    Shrink0,
    Wrap,
    ItemsStart,
    ItemsCenter,
    ItemsEnd,
    ItemsStretch,
    JustifyStart,
    JustifyCenter,
    JustifyEnd,
    JustifyBetween,
    GridCols(u8),
    ColSpan(u8),
    /// `text-xs` .. `text-2xl`
    TextSize(&'static str),
    /// `font-medium`, `font-semibold`, `font-bold`
    Font(&'static str),
    TextLeft,
    TextCenter,
    TextRight,
    /// `text-muted`
    TextMuted,
    Truncate,
    /// `bg-muted`, `bg-surface`, `bg-accent`
    Bg(&'static str),
    Border,
    /// `rounded`, `rounded-sm|md|lg|xl|full|none`
    Rounded(&'static str),
    MinW0,
}

const TEXT_SIZES: &[&str] = &["xs", "sm", "base", "lg", "xl", "2xl"];
const WEIGHTS: &[&str] = &["normal", "medium", "semibold", "bold"];
const BGS: &[&str] = &["muted", "surface", "accent"];
const RADII: &[&str] = &["none", "sm", "md", "lg", "xl", "full"];

/// Every utility pattern, for the authoring guide.
pub const PATTERNS: &[(&str, &str)] = &[
    (
        "gap-N gap-x-N gap-y-N",
        "space between children; N in 0 1 2 3 4 5 6 8 10 12 (× 4 px)",
    ),
    ("p-N px-N py-N pt-N pb-N", "padding, same steps"),
    (
        "w-full w-N max-w-N h-full h-N",
        "size; N in 8 12 16 20 24 32 40 48 56 64 72 80 96 128 (× 4 px)",
    ),
    ("flex-1 grow shrink-0 min-w-0 wrap", "flex behaviour"),
    ("items-start|center|end|stretch", "cross-axis alignment"),
    ("justify-start|center|end|between", "main-axis alignment"),
    ("grid-cols-N col-span-N", "grid columns, N from 1 to 6"),
    ("text-xs|sm|base|lg|xl|2xl", "text size"),
    ("font-normal|medium|semibold|bold", "text weight"),
    (
        "text-left|center|right text-muted truncate",
        "text alignment, muted color, one-line truncation",
    ),
    (
        "bg-muted|surface|accent border rounded rounded-sm|md|lg|xl|full|none",
        "surface, border and radius tokens",
    ),
];

/// Parse a class attribute. Returns the utilities and the words that are not utilities.
pub fn parse_class(s: &str) -> (Vec<Utility>, Vec<String>) {
    let mut ok = Vec::new();
    let mut bad = Vec::new();
    for w in s.split_whitespace() {
        match one(w) {
            Some(u) => ok.push(u),
            None => bad.push(w.to_string()),
        }
    }
    (ok, bad)
}

fn pick(list: &'static [&'static str], v: &str) -> Option<&'static str> {
    list.iter().copied().find(|x| *x == v)
}

fn step(v: &str) -> Option<u8> {
    v.parse::<u8>().ok().filter(|n| SPACING.contains(n))
}

fn size(v: &str) -> Option<u16> {
    v.parse::<u16>().ok().filter(|n| SIZES.contains(n))
}

fn cols(v: &str) -> Option<u8> {
    v.parse::<u8>().ok().filter(|n| (1..=6).contains(n))
}

fn one(w: &str) -> Option<Utility> {
    use Utility::*;
    Some(match w {
        "w-full" => WFull,
        "h-full" => HFull,
        "flex-1" => Flex1,
        "grow" => Grow,
        "shrink-0" => Shrink0,
        "wrap" | "flex-wrap" => Wrap,
        "items-start" => ItemsStart,
        "items-center" => ItemsCenter,
        "items-end" => ItemsEnd,
        "items-stretch" => ItemsStretch,
        "justify-start" => JustifyStart,
        "justify-center" => JustifyCenter,
        "justify-end" => JustifyEnd,
        "justify-between" => JustifyBetween,
        "text-left" => TextLeft,
        "text-center" => TextCenter,
        "text-right" => TextRight,
        "text-muted" => TextMuted,
        "truncate" => Truncate,
        "border" => Border,
        "rounded" => Rounded("md"),
        "min-w-0" => MinW0,
        _ => {
            let (k, v) = w.rsplit_once('-')?;
            match k {
                "gap" => Gap(step(v)?),
                "gap-x" => GapX(step(v)?),
                "gap-y" => GapY(step(v)?),
                "p" => P(step(v)?),
                "px" => Px(step(v)?),
                "py" => Py(step(v)?),
                "pt" => Pt(step(v)?),
                "pb" => Pb(step(v)?),
                "w" => W(size(v)?),
                "max-w" => MaxW(size(v)?),
                "h" => H(size(v)?),
                "grid-cols" => GridCols(cols(v)?),
                "col-span" => ColSpan(cols(v)?),
                "text" => TextSize(pick(TEXT_SIZES, v)?),
                "font" => Font(pick(WEIGHTS, v)?),
                "bg" => Bg(pick(BGS, v)?),
                "rounded" => Rounded(pick(RADII, v)?),
                _ => return None,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_only() {
        let (ok, bad) = parse_class(
            "gap-3 p-4 w-full grid-cols-3 text-lg font-bold bg-muted rounded-lg p-[13px] bg-red-500 w-17 gap-7",
        );
        assert_eq!(ok.len(), 8);
        assert_eq!(bad, ["p-[13px]", "bg-red-500", "w-17", "gap-7"]);
    }
}
