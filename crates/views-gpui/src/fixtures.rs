//! Sample data the screenshot and playground hosts grant, so data-bound views show content.

use views_expr::Value;
use views_runtime::FixtureHost;

pub fn host() -> FixtureHost {
    let j = |s: &str| Value::from_json(&serde_json::from_str(s).unwrap_or_default());
    FixtureHost::default()
        .with_data("sales", j(r#"{"total": 184200, "growth": 0.124, "months": [
            {"month": "Jan", "revenue": 12400, "orders": 310}, {"month": "Feb", "revenue": 13900, "orders": 342},
            {"month": "Mar", "revenue": 15100, "orders": 371}, {"month": "Apr", "revenue": 14200, "orders": 355},
            {"month": "May", "revenue": 16800, "orders": 402}, {"month": "Jun", "revenue": 18300, "orders": 437}],
            "regions": [{"region": "North", "share": 38}, {"region": "South", "share": 24}, {"region": "East", "share": 21}, {"region": "West", "share": 17}]}"#))
        .with_data("team", j(r#"[
            {"name": "Ada Lovelace", "role": "Engineering", "status": "active", "tickets": 12},
            {"name": "Grace Hopper", "role": "Platform", "status": "away", "tickets": 7},
            {"name": "Alan Turing", "role": "Research", "status": "active", "tickets": 9},
            {"name": "Katherine Johnson", "role": "Analytics", "status": "active", "tickets": 4}]"#))
        .with_data("weather", j(r#"{"city": "Buffalo", "temp": 41, "condition": "Cloudy", "week": [
            {"day": "Mon", "high": 44}, {"day": "Tue", "high": 47}, {"day": "Wed", "high": 39},
            {"day": "Thu", "high": 36}, {"day": "Fri", "high": 42}]}"#))
        .with_result("search_orders", j(r#"[{"id": "A-1001", "customer": "Northwind", "total": 1240, "status": "paid"},
            {"id": "A-1002", "customer": "Contoso", "total": 860, "status": "open"}]"#))
}
