//! The corpus is the test suite: every view validates, every prefix of every view parses with
//! stable IDs, a one-line edit gives a minimal patch, and together the views use every
//! component, every state scope and every action.

use std::collections::BTreeSet;

use views_catalog::{COMPONENTS, validate};
use views_ir::{Doc, Limits, Patch, Severity, apply};

fn corpus() -> Vec<(String, String)> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../corpus");
    let mut out: Vec<(String, String)> = std::fs::read_dir(dir)
        .expect("corpus directory")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "view"))
        .map(|p| {
            (
                p.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                std::fs::read_to_string(&p).unwrap_or_default(),
            )
        })
        .collect();
    out.sort();
    out
}

#[test]
fn there_are_at_least_forty_views() {
    assert!(corpus().len() >= 40);
}

#[test]
fn every_view_validates_cleanly() {
    let mut bad = Vec::new();
    for (name, src) in corpus() {
        let doc = Doc::parse(&src, &Limits::default());
        for d in validate(&doc) {
            if d.severity >= Severity::Warning {
                bad.push(format!("{name}: {} {} {:?}", d.code, d.message, d.hint));
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn every_prefix_parses_and_complete_ids_never_change() {
    for (name, src) in corpus() {
        let mut prev = Doc::parse("", &Limits::default());
        for end in 1..=src.len() {
            if !src.is_char_boundary(end) {
                continue;
            }
            let (next, patches) = prev.reparse(&src[..end], &Limits::default());
            for n in prev.nodes.values().filter(|n| n.complete) {
                let m = next
                    .nodes
                    .get(&n.id)
                    .unwrap_or_else(|| panic!("{name}@{end}: <{}> lost its ID", n.tag));
                assert_eq!(m.span.start, n.span.start, "{name}@{end}");
            }
            if end % 97 == 0 || end == src.len() {
                let mut check = prev.clone();
                apply(&mut check, &patches);
                assert_eq!(
                    check.nodes, next.nodes,
                    "{name}@{end}: patches do not reproduce the map"
                );
            }
            prev = next;
        }
    }
}

#[test]
fn a_one_line_edit_gives_a_minimal_patch_set() {
    for (name, src) in corpus() {
        let a = Doc::parse(&src, &Limits::default());
        // Change the first literal title or label.
        let Some(pos) = src.find("label=\"").or_else(|| src.find("title=\"")) else {
            continue;
        };
        let start = src[pos..].find('"').map(|o| pos + o + 1).unwrap_or(pos);
        let edited = format!("{}Edited {}", &src[..start], &src[start..]);
        let (b, patches) = a.reparse(&edited, &Limits::default());
        let visible: Vec<&Patch> = patches.iter().filter(|p| p.is_visible()).collect();
        assert_eq!(visible.len(), 1, "{name}: {visible:?}");
        assert!(matches!(visible[0], Patch::SetProp { .. }), "{name}");
        let same: usize = a.nodes.keys().filter(|id| b.nodes.contains_key(id)).count();
        assert_eq!(same, a.len(), "{name}: every node keeps its ID");
    }
}

#[test]
fn the_corpus_covers_the_catalog_scopes_and_actions() {
    let mut tags = BTreeSet::new();
    let mut text = String::new();
    for (_, src) in corpus() {
        let doc = Doc::parse(&src, &Limits::default());
        tags.extend(doc.nodes.values().map(|n| n.tag.clone()));
        for n in doc.nodes.values() {
            if n.is("Chart") {
                tags.insert(format!("Chart:{}", n.prop("type").unwrap_or("")));
            }
            for b in n.bindings.values() {
                text.push_str(b);
                text.push('\n');
            }
        }
    }
    let missing: Vec<&str> = COMPONENTS
        .iter()
        .map(|c| c.tag)
        .filter(|t| !tags.contains(*t))
        .collect();
    assert!(
        missing.is_empty(),
        "components not in the corpus: {missing:?}"
    );
    for t in ["bar", "line", "area", "pie"] {
        assert!(tags.contains(&format!("Chart:{t}")), "chart type {t}");
    }
    for scope in ["local.", "shared.", "data."] {
        assert!(text.contains(scope), "scope {scope}");
    }
    for action in [
        "set(", "toggle(", "append(", "remove(", "reset(", "query(", "call(", "open(",
    ] {
        assert!(text.contains(action), "action {action}");
    }
}
