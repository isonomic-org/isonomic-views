use std::rc::Rc;

use views_ir::{Doc, Limits, NodeId};
use views_runtime::{DenyAll, Effect, FixtureHost, Frame, Host, MemoryStore, Runtime, Value};

const CALC: &str = r#"<View local={{ open: false, note: "" }} shared={{ seats: 25, price: 49, items: [{name: "a", done: false}, {name: "b", done: true}] }}>
  <Slider bind={shared.seats} min={1} max={500} />
  <Stat label="Annual" value={currency(shared.seats * shared.price * 12)} />
  <Text>Price {shared.price}</Text>
  <Text>Open {local.open}</Text>
  <Button onClick={toggle(local.open); set(shared.price, shared.price + 1)}>Go</Button>
  <For each={shared.items} as="it">
    <Checkbox bind={it.done} label={it.name} />
  </For>
  <Button onClick={append(shared.items, {name: "c", done: false}); remove(shared.items, x => x.done)}>Edit list</Button>
  <Button onClick={query("orders", {open: true}, local.rows); open("doc:42")}>Ask</Button>
  <Button onClick={reset(shared.price); reset()}>Reset</Button>
  <Text>{data.sales.total}</Text>
</View>"#;

fn rt_with(host: Rc<dyn Host>, store: views_runtime::SharedHandle) -> Runtime {
    Runtime::new(
        Doc::parse(CALC, &Limits::default()),
        host,
        store,
        Limits::default(),
    )
}

fn find(rt: &Runtime, tag: &str, nth: usize) -> NodeId {
    rt.doc()
        .preorder()
        .into_iter()
        .filter(|id| rt.doc().nodes[id].tag == tag)
        .nth(nth)
        .unwrap()
}

fn p(s: &str) -> Vec<String> {
    s.split('.').map(String::from).collect()
}

#[test]
fn scopes_initialise_and_evaluate() {
    let rt = rt_with(Rc::new(DenyAll), MemoryStore::handle());
    let f = Frame::root();
    assert_eq!(
        rt.eval(find(&rt, "Stat", 0), "value", &f).to_text(),
        "$14,700.00"
    );
    assert_eq!(
        rt.eval(find(&rt, "Stat", 0), "label", &f).to_text(),
        "Annual"
    );
    assert_eq!(rt.text(find(&rt, "Text", 0), &f), "Price 49");
    assert_eq!(rt.get(&p("local.open")), Value::Bool(false));
}

#[test]
fn bindings_re_evaluate_only_what_changed() {
    let rt = rt_with(Rc::new(DenyAll), MemoryStore::handle());
    let f = Frame::root();
    let stat = find(&rt, "Stat", 0);
    let open_text = find(&rt, "Text", 1);
    let all = |rt: &Runtime| {
        rt.eval(stat, "value", &f);
        rt.text(find(rt, "Text", 0), &f);
        rt.text(open_text, &f);
    };
    all(&rt);
    let before = rt.evals();
    all(&rt);
    assert_eq!(rt.evals(), before, "a second pass must hit the cache");
    rt.write_bind(find(&rt, "Slider", 0), &f, Value::Num(200.0))
        .unwrap();
    all(&rt);
    assert_eq!(
        rt.evals(),
        before + 1,
        "only the Stat depends on shared.seats"
    );
    assert_eq!(rt.eval(stat, "value", &f).to_text(), "$117,600.00");
}

#[test]
fn local_is_per_viewer_and_shared_is_common() {
    let store = MemoryStore::handle();
    let a = rt_with(Rc::new(DenyAll), store.clone());
    let b = rt_with(Rc::new(DenyAll), store.clone());
    let f = Frame::root();
    assert_eq!(b.text(find(&b, "Text", 0), &f), "Price 49");
    let effects = a.dispatch(find(&a, "Button", 0), "onClick", &f, Value::Null);
    assert_eq!(effects.len(), 2);
    assert_eq!(a.get(&p("local.open")), Value::Bool(true));
    assert_eq!(b.get(&p("local.open")), Value::Bool(false));
    assert_eq!(
        b.text(find(&b, "Text", 0), &f),
        "Price 49",
        "b has not synced yet"
    );
    assert!(b.sync());
    assert_eq!(b.text(find(&b, "Text", 0), &f), "Price 50");
    assert_eq!(a.shared(), b.shared());
}

#[test]
fn loop_items_bind_to_stored_items() {
    let rt = rt_with(Rc::new(DenyAll), MemoryStore::handle());
    let root = Frame::root();
    let for_id = find(&rt, "For", 0);
    let frames = rt.for_frames(for_id, &root);
    assert_eq!(frames.len(), 2);
    let cb = find(&rt, "Checkbox", 0);
    assert_eq!(rt.eval(cb, "label", &frames[1]).to_text(), "b");
    assert_eq!(rt.eval(cb, "bind", &frames[0]), Value::Bool(false));
    rt.write_bind(cb, &frames[0], Value::Bool(true)).unwrap();
    assert_eq!(rt.get(&p("shared.items.0.done")), Value::Bool(true));
    let frames = rt.for_frames(for_id, &root);
    assert_eq!(rt.eval(cb, "bind", &frames[0]), Value::Bool(true));
    assert_ne!(frames[0].inst, frames[1].inst);
}

#[test]
fn append_remove_reset() {
    let rt = rt_with(Rc::new(DenyAll), MemoryStore::handle());
    let f = Frame::root();
    rt.dispatch(find(&rt, "Button", 1), "onClick", &f, Value::Null);
    let names: Vec<String> = rt
        .get(&p("shared.items"))
        .as_list()
        .unwrap()
        .iter()
        .map(|i| i.field("name").to_text())
        .collect();
    assert_eq!(names, ["a", "c"]);
    rt.dispatch(find(&rt, "Button", 0), "onClick", &f, Value::Null);
    assert_eq!(rt.get(&p("shared.price")), Value::Num(50.0));
    rt.dispatch(find(&rt, "Button", 3), "onClick", &f, Value::Null);
    assert_eq!(rt.get(&p("shared.price")), Value::Num(49.0));
    assert_eq!(rt.get(&p("local.open")), Value::Bool(false));
}

#[test]
fn the_default_host_denies_everything() {
    let rt = rt_with(Rc::new(DenyAll), MemoryStore::handle());
    let f = Frame::root();
    let eff = rt.dispatch(find(&rt, "Button", 2), "onClick", &f, Value::Null);
    assert!(
        eff.iter()
            .all(|e| matches!(e, Effect::Host { ok: false, .. })),
        "{eff:?}"
    );
    assert!(rt.get(&p("local.rows")).is_null());
    assert_eq!(rt.text(find(&rt, "Text", 2), &f), "");
    let codes: Vec<String> = rt.diagnostics().into_iter().map(|d| d.code).collect();
    assert!(
        codes.contains(&"host-denied".to_string()) && codes.contains(&"action".to_string()),
        "{codes:?}"
    );
    let h = DenyAll;
    assert!(h.call("x", &Value::Null).is_err() && h.image("x").is_err() && h.open("x").is_err());
}

#[test]
fn a_granting_host_supplies_data_and_results() {
    let host = Rc::new(
        FixtureHost::default()
            .with_data(
                "sales",
                Value::from_json(&serde_json::json!({"total": 1200})),
            )
            .with_result("orders", Value::from_json(&serde_json::json!([{"id": 1}]))),
    );
    let rt = rt_with(host.clone(), MemoryStore::handle());
    let f = Frame::root();
    assert_eq!(rt.text(find(&rt, "Text", 2), &f), "1200");
    rt.dispatch(find(&rt, "Button", 2), "onClick", &f, Value::Null);
    assert_eq!(
        rt.get(&p("local.rows")).as_list().map(<[Value]>::len),
        Some(1)
    );
    assert_eq!(host.opened.borrow().as_slice(), ["doc:42"]);
}

#[test]
fn errors_become_diagnostics() {
    let src = r#"<View><Text>{1 / 0}</Text><Text>{1 +* 2}</Text><Button onClick={set(data.x, 1)}>x</Button><Button onClick={set(local.big, range(600))}>y</Button></View>"#;
    let rt = Runtime::new(
        Doc::parse(src, &Limits::default()),
        Rc::new(DenyAll),
        MemoryStore::handle(),
        Limits::default(),
    );
    let f = Frame::root();
    assert_eq!(rt.text(find(&rt, "Text", 0), &f), "");
    for i in 0..2 {
        rt.dispatch(find(&rt, "Button", i), "onClick", &f, Value::Null);
    }
    rt.text(find(&rt, "Text", 1), &f);
    let msgs: Vec<String> = rt.diagnostics().into_iter().map(|d| d.message).collect();
    assert!(msgs.iter().any(|m| m.contains("division")), "{msgs:?}");
    assert!(rt.diagnostics().iter().any(|d| d.code == "expr-syntax"));
    assert!(msgs.iter().any(|m| m.contains("read-only")), "{msgs:?}");
    assert!(msgs.iter().any(|m| m.contains("list")), "{msgs:?}");
}

#[test]
fn expression_step_limit_applies() {
    let src = "<View><Text>{sum(map(range(400), i => sum(map(range(400), j => j))))}</Text></View>";
    let rt = Runtime::new(
        Doc::parse(src, &Limits::default()),
        Rc::new(DenyAll),
        MemoryStore::handle(),
        Limits::default(),
    );
    assert_eq!(rt.text(find(&rt, "Text", 0), &Frame::root()), "");
    assert!(rt.diagnostics()[0].message.contains("steps"));
}

#[test]
fn state_survives_doc_updates() {
    let mut rt = rt_with(Rc::new(DenyAll), MemoryStore::handle());
    let f = Frame::root();
    rt.write_bind(find(&rt, "Slider", 0), &f, Value::Num(300.0))
        .unwrap();
    rt.dispatch(find(&rt, "Button", 0), "onClick", &f, Value::Null);
    let (next, _) = rt.doc().reparse(
        &CALC
            .replace("label=\"Annual\"", "label=\"Per year\"")
            .replace("note: \"\"", "note: \"\", extra: 1"),
        &Limits::default(),
    );
    rt.set_doc(next);
    assert_eq!(rt.get(&p("shared.seats")), Value::Num(300.0));
    assert_eq!(rt.get(&p("local.open")), Value::Bool(true));
    assert_eq!(rt.get(&p("local.extra")), Value::Num(1.0));
    assert_eq!(
        rt.eval(find(&rt, "Stat", 0), "label", &f).to_text(),
        "Per year"
    );
}
