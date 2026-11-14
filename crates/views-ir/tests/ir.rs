use views_ir::{Doc, Kind, Limits, Patch, apply, diff};

const SAMPLE: &str = r#"<View shared={{ seats: 25, price: 49 }}>
  <Col class="gap-3 p-4">
    <Heading>Seat cost calculator</Heading>
    <Slider bind={shared.seats} min={1} max={500} />
    <NumberInput bind={shared.price} prefix="$" />
    <Stat label="Annual cost" value={currency(shared.seats * shared.price * 12)} />
    <If when={shared.seats > 100}>
      <Callout tone="info">Volume pricing applies.</Callout>
    </If>
    <Text>Total: {shared.seats} seats</Text>
  </Col>
</View>"#;

fn lim() -> Limits {
    Limits::default()
}

/// Structural equality ignoring diagnostics and the ID counter.
fn same(a: &Doc, b: &Doc) -> bool {
    a.root == b.root && a.nodes == b.nodes
}

fn find<'a>(d: &'a Doc, tag: &str) -> &'a views_ir::Node {
    d.nodes
        .values()
        .find(|n| n.tag == tag)
        .unwrap_or_else(|| panic!("no {tag}"))
}

#[test]
fn lowers_props_bindings_and_text() {
    let d = Doc::parse(SAMPLE, &lim());
    assert!(d.diagnostics.is_empty(), "{:?}", d.diagnostics);
    let root = d.root_node().unwrap();
    assert_eq!(root.tag, "View");
    assert_eq!(root.binding("shared"), Some("{ seats: 25, price: 49 }"));
    let s = find(&d, "Slider");
    assert_eq!(s.binding("bind"), Some("shared.seats"));
    assert_eq!(s.binding("min"), Some("1"));
    assert_eq!(find(&d, "NumberInput").prop("prefix"), Some("$"));
    let t = find(&d, "Text");
    let kinds: Vec<Kind> = t.children.iter().map(|c| d.nodes[c].kind).collect();
    assert_eq!(kinds, [Kind::Text, Kind::Expr, Kind::Text]);
    assert_eq!(d.depth(find(&d, "Callout").id), 3);
    assert_eq!(d.preorder().len(), d.len());
}

#[test]
fn ids_of_complete_nodes_never_change_while_streaming() {
    let mut prev = Doc::parse("", &lim());
    let mut prefixes = 0;
    for end in 1..=SAMPLE.len() {
        if !SAMPLE.is_char_boundary(end) {
            continue;
        }
        let (next, patches) = prev.reparse(&SAMPLE[..end], &lim());
        let mut check = prev.clone();
        apply(&mut check, &patches);
        assert!(same(&check, &next), "patches do not reproduce prefix {end}");
        for n in prev.nodes.values().filter(|n| n.complete) {
            let m = next
                .nodes
                .get(&n.id)
                .unwrap_or_else(|| panic!("complete node {} lost at {end}", n.tag));
            assert_eq!(
                (m.tag.as_str(), m.span.start),
                (n.tag.as_str(), n.span.start)
            );
        }
        prev = next;
        prefixes += 1;
    }
    assert!(prefixes > 300);
    assert!(
        same(&prev, &{
            let mut d = Doc::parse(SAMPLE, &lim());
            // Fresh IDs differ only in numbering; compare shapes instead.
            d.root = prev.root;
            d
        }) || prev.len() == Doc::parse(SAMPLE, &lim()).len()
    );
}

#[test]
fn one_line_edit_gives_a_minimal_patch() {
    let a = Doc::parse(SAMPLE, &lim());
    let edited = SAMPLE.replace("max={500}", "max={1000}");
    let (b, patches) = a.reparse(&edited, &lim());
    let visible: Vec<&Patch> = patches.iter().filter(|p| p.is_visible()).collect();
    assert_eq!(visible.len(), 1, "{visible:?}");
    assert!(matches!(visible[0], Patch::SetProp { name, .. } if name == "max"));
    for (id, n) in &a.nodes {
        assert_eq!(b.nodes[id].tag, n.tag);
    }
}

#[test]
fn inserting_a_sibling_keeps_the_others() {
    let src = "<View><Col><Text>one</Text><Text>two</Text><Text>three</Text></Col></View>";
    let a = Doc::parse(src, &lim());
    let ids_of = |d: &Doc, s: &str| {
        d.nodes
            .values()
            .find(|n| n.text == s)
            .map(|n| d.nodes[&n.parent.unwrap()].id)
    };
    let (b, patches) = a.reparse(
        &src.replace("<Text>two", "<Text>new</Text><Text>two"),
        &lim(),
    );
    for word in ["one", "two", "three"] {
        assert_eq!(ids_of(&a, word), ids_of(&b, word), "{word}");
    }
    let inserts = patches
        .iter()
        .filter(|p| matches!(p, Patch::Insert { .. }))
        .count();
    let moves = patches
        .iter()
        .filter(|p| matches!(p, Patch::Move { .. }))
        .count();
    assert_eq!((inserts, moves), (2, 0), "{patches:?}");
    let (c, patches) = b.reparse("<View><Col><Text>three</Text><Text>one</Text><Text>new</Text><Text>two</Text></Col></View>", &lim());
    assert_eq!(
        patches
            .iter()
            .filter(|p| matches!(p, Patch::Move { .. }))
            .count(),
        1
    );
    let mut check = b.clone();
    apply(&mut check, &patches);
    assert!(same(&check, &c));
}

#[test]
fn keyed_children_match_by_key() {
    let a = Doc::parse(
        r#"<View><Card key="a">A</Card><Card key="b">B</Card></View>"#,
        &lim(),
    );
    let (b, _) = a.reparse(
        r#"<View><Card key="b">B2</Card><Card key="a">A</Card></View>"#,
        &lim(),
    );
    let key_id = |d: &Doc, k: &str| {
        d.nodes
            .values()
            .find(|n| n.prop("key") == Some(k))
            .unwrap()
            .id
    };
    assert_eq!(key_id(&a, "a"), key_id(&b, "a"));
    assert_eq!(key_id(&a, "b"), key_id(&b, "b"));
}

#[test]
fn random_edits_round_trip_through_patches() {
    let pieces = [
        "<Text>x</Text>",
        "<Badge tone=\"info\">b</Badge>",
        "<Row><Text>r</Text></Row>",
        "{shared.a}",
        "plain",
        "<Divider />",
    ];
    let mut seed: u64 = 7;
    let mut rnd = |n: usize| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % n as u64) as usize
    };
    let mut items: Vec<&str> = vec![];
    let mut doc = Doc::parse("<View></View>", &lim());
    for _ in 0..400 {
        match rnd(3) {
            0 if !items.is_empty() => {
                let i = rnd(items.len());
                items.remove(i);
            }
            1 if items.len() > 1 => {
                let (i, j) = (rnd(items.len()), rnd(items.len()));
                items.swap(i, j);
            }
            _ => {
                let i = rnd(items.len() + 1);
                items.insert(i, pieces[rnd(pieces.len())]);
            }
        }
        let src = format!("<View><Col>{}</Col></View>", items.join(""));
        let (next, patches) = doc.reparse(&src, &lim());
        let mut check = doc.clone();
        apply(&mut check, &patches);
        assert!(same(&check, &next), "{src}");
        assert_eq!(diff(&next, &next), vec![]);
        doc = next;
    }
}

#[test]
fn limits_are_enforced() {
    let many = format!("<View>{}</View>", "<Text>x</Text>".repeat(3000));
    let d = Doc::parse(&many, &lim());
    assert!(d.len() <= lim().max_nodes);
    assert!(d.diagnostics.iter().any(|x| x.code == "limit-nodes"));

    let deep = format!("<View>{}x", "<Col>".repeat(60));
    let d = Doc::parse(&deep, &lim());
    assert!(
        d.preorder()
            .iter()
            .all(|id| d.depth(*id) <= lim().max_depth + 1)
    );
    assert!(d.diagnostics.iter().any(|x| x.code == "limit-depth"));

    let big = format!("<View><Text>{}</Text></View>", "y".repeat(40_000));
    let d = Doc::parse(&big, &lim());
    assert!(d.nodes.values().all(|n| n.text.len() <= lim().max_text));
    assert!(d.diagnostics.iter().any(|x| x.code == "limit-text"));

    let huge = format!("<View>{}</View>", " ".repeat(300 * 1024));
    let d = Doc::parse(&huge, &lim());
    assert!(d.diagnostics.iter().any(|x| x.code == "limit-source"));
}

#[test]
fn serializes() {
    let a = Doc::parse(SAMPLE, &lim());
    let b = Doc::from_json(&a.to_json()).unwrap();
    assert_eq!(a, b);
    let (_, patches) = a.reparse(&SAMPLE.replace("info", "warning"), &lim());
    let j = serde_json::to_string(&patches).unwrap();
    assert!(j.contains(r#""op":"set_prop""#), "{j}");
    let back: Vec<Patch> = serde_json::from_str(&j).unwrap();
    assert_eq!(back, patches);
}

#[test]
fn artifact_create_then_update() {
    use views_ir::{Artifact, Edit, EditError};
    let mut a = Artifact::create("", lim());
    for chunk in SAMPLE.as_bytes().chunks(7) {
        a.append(std::str::from_utf8(chunk).unwrap());
    }
    assert_eq!(a.source(), SAMPLE);
    let slider = find(a.doc(), "Slider").id;
    let edit = |f: &str, r: &str| Edit {
        find: f.into(),
        replace: r.into(),
    };
    let patches = a
        .apply_edits(&[
            edit("max={500}", "max={900}"),
            edit("Seat cost", "Seat price"),
        ])
        .unwrap();
    assert_eq!(patches.iter().filter(|p| p.is_visible()).count(), 2);
    assert_eq!(find(a.doc(), "Slider").id, slider);
    let v = a.version();
    assert_eq!(
        a.apply_edits(&[edit("nope", "x")]),
        Err(EditError::NotFound { index: 0 })
    );
    assert_eq!(
        a.apply_edits(&[edit("max={900}", "max={1}"), edit("shared.", "x")]),
        Err(EditError::Ambiguous { index: 1, count: 6 })
    );
    assert_eq!(a.version(), v, "failed edits change nothing");
    assert!(a.source().contains("max={900}"));
}
