//! Calendar dates as days since 1970-01-01 (proleptic Gregorian, no time zone).

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const DAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

pub fn from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub fn to_civil(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

pub fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        2 if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Parse `YYYY-MM-DD`.
pub fn parse_iso(s: &str) -> Option<i64> {
    let mut it = s.trim().splitn(3, '-');
    let y: i64 = it.next()?.parse().ok()?;
    let m: u32 = it.next()?.parse().ok()?;
    let d: u32 = it.next()?.get(..2)?.parse().ok()?;
    if !(1..=12).contains(&m) || d == 0 || d > days_in_month(y, m) || !(1..=9999).contains(&y) {
        return None;
    }
    Some(from_civil(y, m, d))
}

pub fn iso(z: i64) -> String {
    let (y, m, d) = to_civil(z);
    format!("{y:04}-{m:02}-{d:02}")
}

/// 0 = Monday.
pub fn weekday(z: i64) -> u32 {
    (z + 3).rem_euclid(7) as u32
}

pub fn add_months(z: i64, n: i64) -> i64 {
    let (y, m, d) = to_civil(z);
    let total = y * 12 + (m as i64 - 1) + n;
    let (ny, nm) = (total.div_euclid(12), (total.rem_euclid(12) + 1) as u32);
    from_civil(ny, nm, d.min(days_in_month(ny, nm)))
}

/// Format with tokens: yyyy, yy, MMMM, MMM, MM, M, dd, d, EEEE, EEE. Other text is literal.
pub fn format(z: i64, pattern: &str) -> String {
    let (y, m, d) = to_civil(z);
    let wd = weekday(z) as usize;
    let mut out = String::new();
    let mut rest = pattern;
    while !rest.is_empty() {
        let (tok, text): (usize, String) = if rest.starts_with("yyyy") {
            (4, format!("{y:04}"))
        } else if rest.starts_with("yy") {
            (2, format!("{:02}", y.rem_euclid(100)))
        } else if rest.starts_with("MMMM") {
            (4, MONTHS[m as usize - 1].into())
        } else if rest.starts_with("MMM") {
            (3, MONTHS[m as usize - 1][..3].into())
        } else if rest.starts_with("MM") {
            (2, format!("{m:02}"))
        } else if rest.starts_with('M') {
            (1, m.to_string())
        } else if rest.starts_with("dd") {
            (2, format!("{d:02}"))
        } else if rest.starts_with('d') {
            (1, d.to_string())
        } else if rest.starts_with("EEEE") {
            (4, DAYS[wd].into())
        } else if rest.starts_with("EEE") {
            (3, DAYS[wd][..3].into())
        } else {
            let ch = rest.chars().next().unwrap_or(' ');
            (ch.len_utf8(), ch.to_string())
        };
        out.push_str(&text);
        rest = &rest[tok..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        for z in [-800_000, -1, 0, 1, 19_000, 20_000, 2_900_000] {
            let (y, m, d) = to_civil(z);
            assert_eq!(from_civil(y, m, d), z);
        }
        assert_eq!(iso(0), "1970-01-01");
        assert_eq!(
            parse_iso("2024-02-29").map(iso).as_deref(),
            Some("2024-02-29")
        );
        assert_eq!(parse_iso("2023-02-29"), None);
        assert_eq!(weekday(0), 3);
        assert_eq!(
            format(parse_iso("2026-11-14").unwrap(), "EEE, MMM d yyyy"),
            "Sat, Nov 14 2026"
        );
        assert_eq!(
            iso(add_months(parse_iso("2026-01-31").unwrap(), 1)),
            "2026-02-28"
        );
    }
}
