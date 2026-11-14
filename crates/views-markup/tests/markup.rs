use views_markup::{AttrValue, Node, Severity, line_col, parse};

const SAMPLE: &str = r#"<View shared={{ seats: 25, price: 49 }}>
  <Col class="gap-3 p-4">
    <Heading>Seat cost calculator</Heading>
    <Slider bind={shared.seats} min={1} max={500} />
    <NumberInput bind={shared.price} prefix="$" />
    <Stat label="Annual cost" value={currency(shared.seats * shared.price * 12)} />
    <If when={shared.seats > 100}>
      <Callout tone="info">Volume pricing applies.</Callout>
    </If>
    <Text>Total: {shared.seats} seats &amp; more</Text>
  </Col>
</View>
"#;

fn el(n: &Node) -> &views_markup::Element {
    match n {
        Node::Element(e) => e,
        other => panic!("expected element, got {other:?}"),
    }
}

#[test]
fn parses_the_brief_example() {
    let d = parse(SAMPLE);
    assert!(d.diagnostics.is_empty(), "{:?}", d.diagnostics);
    let view = d.root().unwrap();
    assert_eq!(view.tag, "View");
    assert_eq!(
        view.attr("shared").unwrap().value,
        AttrValue::Expr("{ seats: 25, price: 49 }".into())
    );
    let col = el(&view.children[0]);
    assert_eq!(col.children.len(), 6);
    let slider = el(&col.children[1]);
    assert!(slider.closed);
    assert_eq!(
        slider.attr("bind").unwrap().value,
        AttrValue::Expr("shared.seats".into())
    );
    let text = el(&col.children[5]);
    let kinds: Vec<_> = text
        .children
        .iter()
        .map(|c| match c {
            Node::Text(t) => format!("t:{}", t.text),
            Node::Expr(x) => format!("x:{}", x.src),
            Node::Element(_) => "e".into(),
        })
        .collect();
    assert_eq!(kinds, ["t:Total: ", "x:shared.seats", "t: seats & more"]);
}

#[test]
fn every_prefix_yields_a_tree() {
    for end in 0..=SAMPLE.len() {
        if !SAMPLE.is_char_boundary(end) {
            continue;
        }
        let d = parse(&SAMPLE[..end]);
        assert!(d.consumed <= end);
        if end > 7 {
            assert_eq!(
                d.root().map(|r| r.tag.as_str()),
                Some("View"),
                "prefix {end}"
            );
        }
    }
}

#[test]
fn incomplete_constructs_are_held_back() {
    let d = parse("<View><Slider bind={shared.se");
    let s = el(&el(&d.nodes[0]).children[0]);
    assert_eq!(s.tag, "Slider");
    assert!(s.attrs.is_empty());
    let d = parse("<View><Slid");
    assert!(el(&d.nodes[0]).children.is_empty());
    let d = parse("<View><Text>Hi {shared.na");
    assert_eq!(el(&el(&d.nodes[0]).children[0]).children.len(), 1);
    let d = parse("<View><Badge tone=\"inf");
    assert!(el(&el(&d.nodes[0]).children[0]).attrs.is_empty());
    let d = parse("<View><Button disabled");
    assert!(el(&el(&d.nodes[0]).children[0]).attrs.is_empty());
    let d = parse("<View><Button disabled />");
    assert_eq!(
        el(&el(&d.nodes[0]).children[0]).attrs[0].value,
        AttrValue::Flag
    );
    let d = parse("<View><Text>a</Te");
    assert_eq!(el(&d.nodes[0]).children.len(), 1);
    assert!(!el(&el(&d.nodes[0]).children[0]).closed);
}

#[test]
fn malformed_input_becomes_diagnostics() {
    let d = parse("<View><Row><Text>x</Row></Bogus><Col class=wide>a < b</Col></View>");
    let codes: Vec<_> = d.diagnostics.iter().map(|d| d.code).collect();
    assert!(codes.contains(&"unclosed-tag"));
    assert!(codes.contains(&"stray-close"));
    assert!(codes.contains(&"unquoted-value"));
    assert!(codes.contains(&"stray-lt"));
    assert!(d.diagnostics.iter().any(|d| d.severity == Severity::Error));
    let v = el(&d.nodes[0]);
    assert_eq!(v.children.len(), 2);
}

#[test]
fn braces_inside_strings_and_nesting() {
    let d = parse(r#"<View><Text>{"}" + '{' + join([1, {a: 2}.a], "}")}</Text></View>"#);
    let t = el(&el(&d.nodes[0]).children[0]);
    match &t.children[0] {
        Node::Expr(x) => assert_eq!(x.src, r#""}" + '{' + join([1, {a: 2}.a], "}")"#),
        other => panic!("{other:?}"),
    }
}

#[test]
fn depth_is_bounded_and_comments_skipped() {
    let src = "<View>".to_string() + &"<Col>".repeat(5000) + "x";
    let d = parse(&src);
    assert!(d.diagnostics.iter().any(|d| d.code == "too-deep"));
    let d = parse("<View><!-- <Text>no</Text> --><Text>yes</Text></View>");
    assert_eq!(el(&d.nodes[0]).children.len(), 1);
    assert_eq!(line_col("ab\ncd", 4), (2, 2));
}

#[test]
fn fuzz_never_panics() {
    let atoms = [
        "<", ">", "/", "View", "Text", " ", "=", "\"", "'", "{", "}", "a", "<!--", "-->", "\n",
        "&amp;", "x={1}", "</", "é",
    ];
    let mut seed: u64 = 42;
    for _ in 0..30_000 {
        let mut s = String::new();
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let n = (seed % 40) as usize;
        for k in 0..n {
            let r = seed.rotate_left(k as u32 * 5);
            s.push_str(atoms[(r % atoms.len() as u64) as usize]);
        }
        let d = parse(&s);
        assert!(d.consumed <= s.len());
    }
}

#[test]
fn code_is_literal_and_markdown_keeps_lines() {
    let src = "<View><Code lang=\"rust\">\n  fn main() {\n      <b>{x}</b>\n  }\n</Code><Markdown>\n  # Title\n\n  - {a}\n</Markdown></View>";
    let d = parse(src);
    let v = el(&d.nodes[0]);
    let code = el(&v.children[0]);
    assert_eq!(code.children.len(), 1);
    match &code.children[0] {
        Node::Text(t) => assert!(t.text.contains("<b>{x}</b>") && t.text.contains("\n      ")),
        other => panic!("{other:?}"),
    }
    let md = el(&v.children[1]);
    assert!(matches!(&md.children[0], Node::Text(t) if t.text.contains("# Title\n\n")));
    assert!(matches!(&md.children[1], Node::Expr(x) if x.src == "a"));
    for end in 0..src.len() {
        let d = parse(&src[..end]);
        assert!(d.consumed <= end);
    }
    let d = parse("<View><Code>let x = 1;</Co");
    let code = el(&el(&d.nodes[0]).children[0]);
    assert!(matches!(&code.children[0], Node::Text(t) if t.text == "let x = 1;"));
}
