use views_catalog::{
    COMPONENTS, Children, ICONS, Type, authoring_guide, check, component, json_schema,
};
use views_ir::Severity;

fn codes(src: &str) -> Vec<String> {
    check(src).diagnostics.into_iter().map(|d| d.code).collect()
}

#[test]
fn the_brief_example_is_valid() {
    let src = r#"<View version="0.1" shared={{ seats: 25, price: 49 }}>
  <Col class="gap-3 p-4">
    <Heading>Seat cost calculator</Heading>
    <Slider bind={shared.seats} min={1} max={500} />
    <NumberInput bind={shared.price} prefix="$" />
    <Stat label="Annual cost" value={currency(shared.seats * shared.price * 12)} />
    <If when={shared.seats > 100}>
      <Callout tone="info">Volume pricing applies.</Callout>
    </If>
  </Col>
</View>"#;
    let r = check(src);
    assert!(r.ok && r.diagnostics.is_empty(), "{:?}", r.diagnostics);
}

#[test]
fn catalog_covers_the_brief() {
    for tag in [
        "View",
        "Row",
        "Col",
        "Grid",
        "Card",
        "Tabs",
        "Accordion",
        "Divider",
        "Spacer",
        "ScrollArea",
        "Heading",
        "Text",
        "Markdown",
        "Code",
        "Badge",
        "Kbd",
        "Link",
        "Button",
        "TextField",
        "TextArea",
        "NumberInput",
        "Checkbox",
        "Switch",
        "Slider",
        "Select",
        "SegmentedControl",
        "DatePicker",
        "Stat",
        "Table",
        "List",
        "Progress",
        "Chart",
        "Icon",
        "Avatar",
        "Callout",
        "Tooltip",
        "Skeleton",
        "Spinner",
        "If",
        "For",
    ] {
        assert!(component(tag).is_some(), "{tag}");
    }
    for c in COMPONENTS {
        assert!(!c.role.is_empty() && !c.doc.is_empty(), "{}", c.tag);
        if let Children::Only(t) = c.children {
            assert!(t.iter().all(|t| component(t).is_some()));
        }
        for p in c.props {
            if p.name.starts_with("on") {
                assert_eq!(p.ty, Type::Action, "{}.{}", c.tag, p.name);
            }
        }
        let r = check(&format!(
            "<View version=\"0.1\">{}</View>",
            if c.tag == "View" { "" } else { c.example }
        ));
        let errors: Vec<_> = r
            .diagnostics
            .iter()
            .filter(|d| {
                d.severity == Severity::Error
                    && !["unknown-name", "misplaced", "bad-bind"].contains(&d.code.as_str())
            })
            .collect();
        assert!(errors.is_empty(), "example of {}: {errors:?}", c.tag);
    }
    assert!(ICONS.len() > 50);
}

#[test]
fn catches_what_a_model_gets_wrong() {
    let src = r##"<View>
  <Colum>
    <Row class="gap-3 p-[13px] bg-red-500" />
    <Slider bind={currency(shared.x)} max="lots" />
    <Badge tone="purple" color="#ff0000">x</Badge>
    <Button onClick={increment(local.n)} variant="huge">Go</Button>
    <Button onClick={set(data.x, 1)}>Bad</Button>
    <Text>{totl * 2}</Text>
    <Text>{sumof(shared.list)}</Text>
    <Stat value={1} />
    <Tab label="x" />
    <Avatar name="A" asset="https://example.com/a.png" />
    <Select bind={local.s} options="a,b" />
    <Icon name="sta" />
    <Divider>no kids</Divider>
    <For each={shared.items} as={x}><Text>{item.name}</Text></For>
    <Text>{1 +* 2}</Text>
  </Colum>
  <Col>loose text</Col>
</View>"##;
    let c = codes(src);
    for want in [
        "missing-version",
        "unknown-component",
        "bad-class",
        "bad-bind",
        "bad-value",
        "unknown-prop",
        "bad-action",
        "unknown-name",
        "unknown-function",
        "missing-prop",
        "misplaced",
        "url-not-allowed",
        "invalid-child",
        "literal-required",
        "expr-syntax",
        "bare-text",
    ] {
        assert!(c.iter().any(|x| x == want), "missing {want} in {c:?}");
    }
    let r = check(src);
    assert!(!r.ok);
    let json = serde_json::to_value(&r).unwrap();
    let first = &json["diagnostics"][0];
    assert!(
        first["code"].is_string() && first["severity"].is_string() && first["message"].is_string()
    );
    let hints: Vec<_> = r
        .diagnostics
        .iter()
        .filter_map(|d| d.hint.clone())
        .collect();
    assert!(hints.iter().any(|h| h.contains("<Col>")), "{hints:?}");
    assert!(hints.iter().any(|h| h.contains("'star'")), "{hints:?}");
}

#[test]
fn loop_variables_and_events_are_in_scope() {
    let src = r#"<View version="0.1" shared={{ todos: [] }}>
  <For each={shared.todos} as="t" key={t.id}>
    <Checkbox bind={t.done} label={t.title + " " + index} onChange={set(shared.last, event)} />
    <Button onClick={remove(shared.todos, x => x.id == t.id)}>x</Button>
  </For>
</View>"#;
    let r = check(src);
    assert!(r.ok && r.diagnostics.is_empty(), "{:?}", r.diagnostics);
}

#[test]
fn schema_and_guide_come_from_the_catalog() {
    let s = json_schema();
    for c in COMPONENTS {
        assert!(s["$defs"][c.tag].is_object(), "{}", c.tag);
    }
    assert_eq!(
        s["$defs"]["Slider"]["properties"]["props"]["properties"]["bind"]["$ref"],
        "#/$defs/expr"
    );
    assert_eq!(
        s["$defs"]["Badge"]["properties"]["props"]["properties"]["tone"]["anyOf"][0]["enum"][0],
        "neutral"
    );
    let g = authoring_guide(&[("calc", "<View version=\"0.1\"></View>")]);
    for c in COMPONENTS {
        assert!(g.contains(&format!("<{}>", c.tag)), "{}", c.tag);
    }
    for f in views_expr::FUNCTIONS {
        assert!(g.contains(f.1));
    }
    assert!(g.contains("### calc"));
}
