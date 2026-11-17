// Copyright 2026 Anud Inc.
// SPDX-License-Identifier: GPL-3.0-or-later

//! The table of standard functions.

/// Every standard function: name, signature, one-line description. The authoring guide is
/// generated from this table, so keep it in step with `call`.
pub const FUNCTIONS: &[(&str, &str, &str)] = &[
    // math
    ("abs", "abs(x)", "Absolute value."),
    (
        "round",
        "round(x, digits?)",
        "Round to `digits` decimal places (default 0).",
    ),
    ("floor", "floor(x)", "Round down."),
    ("ceil", "ceil(x)", "Round up."),
    ("sqrt", "sqrt(x)", "Square root."),
    ("pow", "pow(x, y)", "x to the power y."),
    ("clamp", "clamp(x, lo, hi)", "Limit x to the range lo..hi."),
    ("min", "min(a, b, ...) or min(list, f?)", "Smallest value."),
    ("max", "max(a, b, ...) or max(list, f?)", "Largest value."),
    (
        "sum",
        "sum(list, f?)",
        "Total of a list, optionally of f(item).",
    ),
    (
        "avg",
        "avg(list, f?)",
        "Mean of a list, optionally of f(item); null when empty.",
    ),
    (
        "count",
        "count(list, f?)",
        "Number of items, optionally only those where f(item) is true.",
    ),
    // text
    ("len", "len(x)", "Length of a text or list."),
    ("upper", "upper(text)", "Upper case."),
    ("lower", "lower(text)", "Lower case."),
    ("trim", "trim(text)", "Remove surrounding spaces."),
    (
        "contains",
        "contains(text_or_list, x)",
        "Whether x occurs in the text or list.",
    ),
    (
        "startsWith",
        "startsWith(text, prefix)",
        "Whether text starts with prefix.",
    ),
    (
        "replace",
        "replace(text, find, with)",
        "Replace every occurrence.",
    ),
    ("split", "split(text, sep)", "Split text into a list."),
    (
        "join",
        "join(list, sep?)",
        "Join a list into text (default separator \", \").",
    ),
    ("text", "text(x)", "Any value as text."),
    (
        "plural",
        "plural(n, one, many)",
        "\"1 seat\" / \"3 seats\" style wording.",
    ),
    // lists
    ("map", "map(list, x => ...)", "Transform each item."),
    (
        "filter",
        "filter(list, x => ...)",
        "Keep items where the function is true.",
    ),
    (
        "find",
        "find(list, x => ...)",
        "First item where the function is true, or null.",
    ),
    ("any", "any(list, x => ...)", "Whether any item matches."),
    ("all", "all(list, x => ...)", "Whether every item matches."),
    (
        "sortBy",
        "sortBy(list, x => key, desc?)",
        "Sorted copy by key.",
    ),
    ("reverse", "reverse(list)", "Reversed copy."),
    ("first", "first(list)", "First item or null."),
    ("last", "last(list)", "Last item or null."),
    (
        "slice",
        "slice(list_or_text, start, end?)",
        "Part of a list or text.",
    ),
    (
        "range",
        "range(n) or range(a, b)",
        "Numbers a..b-1 (bounded by the list limit).",
    ),
    (
        "unique",
        "unique(list)",
        "Distinct items, first occurrence kept.",
    ),
    ("keys", "keys(record)", "Field names of a record."),
    ("values", "values(record)", "Field values of a record."),
    ("merge", "merge(a, b)", "Record with b's fields over a's."),
    // dates
    (
        "date",
        "date(\"2026-03-01\") or date(y, m, d)",
        "A calendar date.",
    ),
    ("year", "year(d)", "Year of a date."),
    ("month", "month(d)", "Month of a date, 1-12."),
    ("day", "day(d)", "Day of the month."),
    ("weekday", "weekday(d)", "Day of the week, 0 = Monday."),
    ("addDays", "addDays(d, n)", "Date n days later."),
    (
        "addMonths",
        "addMonths(d, n)",
        "Date n months later, clamped to month end.",
    ),
    (
        "daysBetween",
        "daysBetween(a, b)",
        "Whole days from a to b.",
    ),
    // formatting and logic
    ("number", "number(x, decimals?)", "1,234.5 style number."),
    (
        "currency",
        "currency(x, code?, decimals?)",
        "$1,234.00 style money (default USD).",
    ),
    ("percent", "percent(fraction, decimals?)", "0.25 → 25%."),
    ("compact", "compact(x)", "1200 → 1.2K."),
    (
        "formatDate",
        "formatDate(d, pattern?)",
        "Pattern tokens yyyy MMMM MMM MM dd d EEEE EEE (default \"MMM d, yyyy\").",
    ),
    ("if", "if(cond, a, b)", "a when cond is true, else b."),
    (
        "coalesce",
        "coalesce(a, b, ...)",
        "First value that is not null.",
    ),
    (
        "num",
        "num(x)",
        "Text or boolean as a number; null if not a number.",
    ),
];
