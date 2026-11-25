// Copyright 2026 Anud Inc.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Conversions between view dates (days since 1970-01-01) and the date picker's values.

use chrono::NaiveDate;
use ui::{
    Context, Window,
    component::{
        calendar::Date,
        date_picker::{DatePickerState, DateTime},
    },
};

fn epoch() -> NaiveDate {
    NaiveDate::from_ymd_opt(1970, 1, 1).unwrap_or_default()
}

pub fn to_naive(days: i64) -> NaiveDate {
    epoch() + chrono::Duration::days(days)
}

pub fn from_naive(d: NaiveDate) -> i64 {
    (d - epoch()).num_days()
}

pub fn from_picker(v: &DateTime) -> Option<i64> {
    match v {
        DateTime::Single(Some(dt)) => Some(from_naive(dt.date())),
        _ => None,
    }
}

pub fn of_state(s: &DatePickerState) -> Option<i64> {
    match s.date() {
        Date::Single(Some(d)) => Some(from_naive(d)),
        _ => None,
    }
}

pub fn set_state(
    s: &mut DatePickerState,
    days: i64,
    window: &mut Window,
    cx: &mut Context<DatePickerState>,
) {
    s.set_date(to_naive(days), window, cx);
}
