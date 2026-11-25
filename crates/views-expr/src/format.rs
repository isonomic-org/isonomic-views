// Copyright 2026 Anud Inc.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Number formatting. Fixed to English conventions in this milestone.

/// `1234567.891` with 2 decimals → `1,234,567.89`.
pub fn grouped(n: f64, decimals: usize) -> String {
    if !n.is_finite() {
        return crate::value::fmt_num(n);
    }
    let decimals = decimals.min(10);
    let s = format!("{:.*}", decimals, n.abs());
    let (int, frac) = match s.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (s.as_str(), None),
    };
    let mut out = String::new();
    for (i, ch) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    if let Some(f) = frac {
        out.push('.');
        out.push_str(f);
    }
    let zero = out.chars().all(|c| c == '0' || c == '.' || c == ',');
    if n < 0.0 && !zero {
        format!("-{out}")
    } else {
        out
    }
}

pub fn currency(n: f64, code: &str, decimals: Option<usize>) -> String {
    let code = code.to_ascii_uppercase();
    let (sym, default_dec) = match code.as_str() {
        "USD" => ("$", 2),
        "CAD" => ("CA$", 2),
        "AUD" => ("A$", 2),
        "EUR" => ("€", 2),
        "GBP" => ("£", 2),
        "JPY" => ("¥", 0),
        "INR" => ("₹", 2),
        "CHF" => ("CHF ", 2),
        _ => ("", 2),
    };
    let body = grouped(n.abs(), decimals.unwrap_or(default_dec));
    let sign = if n < 0.0 && body.chars().any(|c| c.is_ascii_digit() && c != '0') {
        "-"
    } else {
        ""
    };
    if sym.is_empty() {
        format!("{sign}{code} {body}")
    } else {
        format!("{sign}{sym}{body}")
    }
}

/// `0.256` → `25.6%` with 1 decimal.
pub fn percent(fraction: f64, decimals: usize) -> String {
    format!("{}%", grouped(fraction * 100.0, decimals))
}

/// `1234` → `1.2K`, `2500000` → `2.5M`.
pub fn compact(n: f64) -> String {
    let a = n.abs();
    let (v, suffix) = if a >= 1e12 {
        (n / 1e12, "T")
    } else if a >= 1e9 {
        (n / 1e9, "B")
    } else if a >= 1e6 {
        (n / 1e6, "M")
    } else if a >= 1e3 {
        (n / 1e3, "K")
    } else {
        return crate::value::fmt_num((n * 10.0).round() / 10.0);
    };
    let s = format!("{v:.1}");
    format!("{}{suffix}", s.trim_end_matches(".0"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(grouped(1_234_567.891, 2), "1,234,567.89");
        assert_eq!(grouped(-1000.0, 0), "-1,000");
        assert_eq!(grouped(-0.001, 2), "0.00");
        assert_eq!(currency(14_700.0, "usd", None), "$14,700.00");
        assert_eq!(currency(-5.5, "EUR", Some(1)), "-€5.5");
        assert_eq!(currency(3.0, "XYZ", Some(0)), "XYZ 3");
        assert_eq!(percent(0.256, 1), "25.6%");
        assert_eq!(compact(1234.0), "1.2K");
        assert_eq!(compact(2_000_000.0), "2M");
        assert_eq!(compact(12.34), "12.3");
    }
}
