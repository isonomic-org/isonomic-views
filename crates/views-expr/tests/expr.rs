use std::collections::BTreeMap;

use views_expr::{ErrorKind, Limits, Value, deps, eval_str, evaluate, parse, parse_seq};

fn scope() -> BTreeMap<String, Value> {
    let shared = Value::from_json(&serde_json::json!({
        "seats": 25, "price": 49,
        "items": [
            {"name": "Milk", "done": true, "qty": 2},
            {"name": "Eggs", "done": false, "qty": 12},
            {"name": "Bread", "done": false, "qty": 1}
        ]
    }));
    let mut m = BTreeMap::new();
    m.insert("shared".to_string(), shared);
    m.insert(
        "local".to_string(),
        Value::from_json(&serde_json::json!({"q": "e"})),
    );
    m
}

fn ev(src: &str) -> Value {
    eval_str(src, &scope(), Limits::default()).unwrap_or_else(|e| panic!("{src}: {e}"))
}

fn text(src: &str) -> String {
    ev(src).to_text()
}

#[test]
fn arithmetic_and_logic() {
    assert_eq!(text("shared.seats * shared.price * 12"), "14700");
    assert_eq!(text("1 + 2 * 3 - 4 / 2"), "5");
    assert_eq!(text("(1 + 2) * 3 % 4"), "1");
    assert_eq!(text("-shared.seats + 5"), "-20");
    assert_eq!(ev("shared.seats > 100"), Value::Bool(false));
    assert_eq!(ev("!(1 == 2) && 'a' < 'b' || false"), Value::Bool(true));
    assert_eq!(text("shared.seats > 10 ? 'many' : 'few'"), "many");
    assert_eq!(text("local.missing ?? 'default'"), "default");
    assert_eq!(text("'Total: ' + 3"), "Total: 3");
    assert_eq!(text("0.1 + 0.2"), "0.3");
    assert_eq!(ev("local.missing + 1"), Value::Null);
}

#[test]
fn lists_and_lambdas() {
    assert_eq!(text("count(shared.items, i => i.done)"), "1");
    assert_eq!(text("sum(shared.items, i => i.qty)"), "15");
    assert_eq!(
        text("join(map(filter(shared.items, i => !i.done), i => i.name), ' / ')"),
        "Eggs / Bread"
    );
    assert_eq!(
        text("find(shared.items, i => contains(lower(i.name), local.q)).name"),
        "Eggs"
    );
    assert_eq!(
        text("map(sortBy(shared.items, i => i.qty, true), i => i.qty)"),
        "12, 2, 1"
    );
    assert_eq!(text("map(shared.items, (i, n) => n)"), "0, 1, 2");
    assert_eq!(text("shared.items[1].name"), "Eggs");
    assert_eq!(text("len(range(5)) + max(3, 9, 4) + min([7, 2])"), "16");
    assert_eq!(text("avg([2, 4])"), "3");
    assert_eq!(ev("avg([])"), Value::Null);
    assert_eq!(text("unique([1, 1, 2])"), "1, 2");
    assert_eq!(text("slice('abcdef', 1, 3) + slice([1,2,3], 2)"), "bc3");
    assert_eq!(text("{a: 1, 'b c': [1, 2]}['b c'][1]"), "2");
    assert_eq!(ev("any(shared.items, i => i.qty > 10)"), Value::Bool(true));
    assert_eq!(ev("all(shared.items, i => i.done)"), Value::Bool(false));
}

#[test]
fn text_dates_and_formatting() {
    assert_eq!(
        text("currency(shared.seats * shared.price * 12)"),
        "$14,700.00"
    );
    assert_eq!(text("number(1234567.5, 1)"), "1,234,567.5");
    assert_eq!(text("percent(0.125, 1)"), "12.5%");
    assert_eq!(text("compact(1500)"), "1.5K");
    assert_eq!(
        text("plural(1, 'seat', 'seats') + ' / ' + plural(3, 'seat', 'seats')"),
        "1 seat / 3 seats"
    );
    assert_eq!(
        text("formatDate(addDays(date('2026-11-14'), 30))"),
        "Dec 14, 2026"
    );
    assert_eq!(
        text("daysBetween(date(2026, 1, 1), date('2026-12-25'))"),
        "358"
    );
    assert_eq!(text("weekday(date('2026-11-16'))"), "0");
    assert_eq!(text("upper(trim('  hi '))"), "HI");
    assert_eq!(text("replace('a-b-c', '-', '+')"), "a+b+c");
    assert_eq!(text("split('a,b', ',')[1]"), "b");
    assert_eq!(text("if(true, 'y', 1 / 0)"), "y");
    assert_eq!(text("num('1,200') + 1"), "1201");
}

#[test]
fn errors_are_values_not_panics() {
    let s = scope();
    let err = |src: &str| eval_str(src, &s, Limits::default()).unwrap_err();
    assert!(err("1 / 0").contains("division"));
    assert!(err("nope(1)").contains("unknown function"));
    assert!(err("undefined_name").contains("unknown name"));
    assert!(err("abs(1, 2)").contains("takes"));
    assert!(err("x => x").contains("function"));
    assert!(err("'a' < 1").contains("compare"));
}

#[test]
fn step_and_size_limits() {
    let e = parse("sum(map(range(1000), i => sum(map(range(1000), j => j))))").unwrap();
    let r = evaluate(&e, &scope(), Limits::default());
    assert_eq!(r.unwrap_err().kind, ErrorKind::Steps);
    let e = parse("range(100000)").unwrap();
    assert_eq!(
        evaluate(&e, &scope(), Limits::default()).unwrap_err().kind,
        ErrorKind::Size
    );
    let tight = Limits {
        max_steps: 5,
        ..Limits::default()
    };
    let e = parse("1 + 2 + 3 + 4 + 5").unwrap();
    assert_eq!(
        evaluate(&e, &scope(), tight).unwrap_err().kind,
        ErrorKind::Steps
    );
}

#[test]
fn parse_errors_know_when_input_is_incomplete() {
    assert!(parse("shared.seats *").unwrap_err().incomplete);
    assert!(parse("currency(shared.seats").unwrap_err().incomplete);
    assert!(parse("'unterminated").unwrap_err().incomplete);
    assert!(!parse("1 + * 2").unwrap_err().incomplete);
    assert!(!parse("a.b(1)").unwrap_err().incomplete);
    let deep = "(".repeat(200) + "1" + &")".repeat(200);
    assert!(parse(&deep).is_err());
}

#[test]
fn dependencies() {
    let d = |s: &str| deps(&parse(s).unwrap()).into_iter().collect::<Vec<_>>();
    assert_eq!(
        d("shared.seats * shared.price"),
        ["shared.price", "shared.seats"]
    );
    assert_eq!(
        d("map(shared.items, i => i.qty * local.k)"),
        ["local.k", "shared.items"]
    );
    assert_eq!(d("shared.items[local.i].name"), ["local.i", "shared.items"]);
    assert_eq!(d("shared.rows[0].x"), ["shared.rows.0.x"]);
    assert_eq!(d("1 + 2"), Vec::<String>::new());
}

#[test]
fn action_sequences() {
    let seq = parse_seq("set(local.a, 1); toggle(shared.b);").unwrap();
    assert_eq!(seq.len(), 2);
    assert_eq!(seq[0], parse("set(local.a, 1)").unwrap());
    assert_eq!(
        parse("shared.items[2].done").unwrap().as_path().unwrap(),
        ["shared", "items", "2", "done"]
    );
}

#[test]
fn values_and_paths() {
    let v = Value::from_json(&serde_json::json!({"a": {"b": [1, 2]}}));
    let v2 = v.with_path(&["a", "b", "1"], Value::Num(9.0));
    assert_eq!(v2.at_path(&["a", "b", "1"]), Value::Num(9.0));
    assert_eq!(v.at_path(&["a", "b", "1"]), Value::Num(2.0));
    let v3 = Value::Null.with_path(&["x", "y"], Value::Bool(true));
    assert_eq!(v3.to_json(), serde_json::json!({"x": {"y": true}}));
}

/// Deterministic fuzzing: random token soup and random byte strings must never panic,
/// and evaluation must finish within its step limit.
#[test]
fn fuzz_parser_and_evaluator() {
    let atoms = [
        "shared", "local", ".", "seats", "items", "(", ")", "[", "]", "{", "}", ",", ":", "?", "+",
        "-", "*", "/", "%", "!", "==", "<", "&&", "||", "??", "=>", "x", "1", "0", "2.5", "'t'",
        "\"", "map", "filter", "sum", "range", "currency", "date", "if", "len", "999", "sortBy",
        "join", "split", " ", "true", "null", ";", "i", "unique", "slice",
    ];
    let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let s = scope();
    for round in 0..20_000 {
        let n = (next() % 24) as usize + 1;
        let src: String = if round % 4 == 0 {
            (0..n)
                .map(|_| char::from_u32((next() % 0x250) as u32).unwrap_or('?'))
                .collect()
        } else {
            (0..n)
                .map(|_| atoms[(next() % atoms.len() as u64) as usize])
                .collect::<Vec<_>>()
                .join("")
        };
        let _ = parse_seq(&src);
        if let Ok(e) = parse(&src) {
            let _ = deps(&e);
            let mut ev = views_expr::Evaluator::new(&s, Limits::default());
            let _ = ev.eval(&e);
            assert!(ev.steps() <= Limits::default().max_steps + 1, "{src}");
        }
    }
}
