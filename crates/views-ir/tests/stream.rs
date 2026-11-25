// Copyright 2026 Anud Inc.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Streaming re-parses only what is new, yet gives exactly the map and patches of a full
//! re-parse aligned against the previous map.

use views_ir::{Artifact, Doc, Limits, Patch};

/// Small deterministic generator, so the tests need no dependency and repeat exactly.
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
}

/// Chunks at char boundaries, between 1 and `max` bytes long.
fn chunks<'a>(src: &'a str, rng: &mut Rng, max: usize) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < src.len() {
        let mut end = (at + 1 + rng.below(max)).min(src.len());
        while !src.is_char_boundary(end) {
            end += 1;
        }
        out.push(&src[at..end]);
        at = end;
    }
    out
}

/// Stream `src` in random chunks and compare every step with a full re-parse. `strict`: the
/// source never changes a node's key while it is written, so streaming never needs a full
/// re-parse and never removes or moves a node.
fn check(name: &str, src: &str, lim: Limits, rng: &mut Rng, max: usize) {
    check_with(name, src, lim, rng, max, true);
}

fn check_with(name: &str, src: &str, lim: Limits, rng: &mut Rng, max: usize, strict: bool) {
    let mut art = Artifact::create("", lim);
    let mut reference = Doc::parse("", &lim);
    for chunk in chunks(src, rng, max) {
        let patches = art.append(chunk);
        let (next, expected) = reference.reparse(art.source(), &lim);
        let at = art.source().len();
        assert_eq!(art.doc(), &next, "{name}: map differs at byte {at}");
        assert_eq!(patches, expected, "{name}: patches differ at byte {at}");
        assert!(
            !strict
                || !patches
                    .iter()
                    .any(|p| matches!(p, Patch::Remove { .. } | Patch::Move { .. })),
            "{name}: a growing source removed or moved a node at byte {at}"
        );
        reference = next;
    }
    if strict {
        assert_eq!(
            art.full_reparses(),
            0,
            "{name}: the stream fell back to a full re-parse"
        );
    }
}

fn corpus() -> Vec<(String, String)> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../corpus");
    let mut out: Vec<(String, String)> = std::fs::read_dir(dir)
        .expect("corpus directory")
        .filter_map(|e| {
            let p = e.ok()?.path();
            if p.extension()? != "view" {
                return None;
            }
            let name = p.file_name()?.to_string_lossy().into_owned();
            Some((name, std::fs::read_to_string(&p).ok()?))
        })
        .collect();
    out.sort();
    out
}

#[test]
fn corpus_streams_match_full_reparses() {
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    let views = corpus();
    assert!(views.len() > 40);
    for (name, src) in &views {
        for max in [6, 40, 200] {
            check(name, src, Limits::default(), &mut rng, max);
        }
    }
}

/// Small limits, so streaming crosses every one of them.
fn tight() -> Limits {
    Limits {
        max_nodes: 40,
        max_depth: 3,
        max_source: 900,
        max_text: 12,
        ..Limits::default()
    }
}

#[test]
fn corpus_streams_match_under_tight_limits() {
    let mut rng = Rng(99);
    for (name, src) in corpus().iter().step_by(3) {
        for max in [5, 60] {
            check(name, src, tight(), &mut rng, max);
        }
    }
}

const TRICKY: &[&str] = &[
    "<View><Code>let a = 1 < 2;\n</Cod</Code><Text>after</Text></View>",
    "<View>\n  <Markdown>\n# Title\n\n  two  spaces &amp; more\n</Markdown>\n</View>",
    "<View><!-- note --><Text a=\"1\" a='2' b={x} c d=bad>hi</Text></View>",
    "<View>< not a tag {'<'} {  } <Text>x</Wrong></Text></View><Extra><B>in</B></Extra>tail",
    "lead <View><Col><Text>one<Text>two</Col><Badge tone=\"info\" />\u{e9}t\u{e9} \u{1F600}",
    "<View><For each={xs} as=\"x\"><Item key={x.id} label={x.name} /><Item />x</For>",
    "<View><Col><Text>a</Text><Text>a</Text><Text>a</Text><Divider /><Divider /></Col></View>",
    "<View/><View><Text>second</Text></View>",
];

#[test]
fn tricky_streams_match_full_reparses() {
    let mut rng = Rng(5);
    for (i, src) in TRICKY.iter().enumerate() {
        for lim in [Limits::default(), tight()] {
            for max in [1, 2, 3, 7] {
                for _ in 0..6 {
                    check(&format!("tricky {i}"), src, lim, &mut rng, max);
                }
            }
        }
    }
}

#[test]
fn a_key_rewritten_mid_tag_falls_back_to_a_full_reparse() {
    // The second `key` wins, so the node's key changes while its tag is written; the full
    // re-parse gives it a new identity, and streaming hands that case to it.
    let src = "<View><Col><Item key={x.id} key=\"dup\" /><Item key=\"dup\" /></Col></View>";
    let mut rng = Rng(3);
    for max in [1, 2, 5] {
        for _ in 0..10 {
            check_with(
                "rewritten key",
                src,
                Limits::default(),
                &mut rng,
                max,
                false,
            );
        }
    }
    let mut art = Artifact::create("", Limits::default());
    for i in 0..src.len() {
        art.append(&src[i..i + 1]);
    }
    assert!(art.full_reparses() > 0, "the fallback was exercised");
}

#[test]
fn a_stream_that_crosses_every_limit_matches() {
    let mut src = String::from("<View>");
    for i in 0..30 {
        src.push_str(&format!(
            "<Col><Row><Card title=\"{i} with a long title\"><Text>t{i}</Text></Card></Row></Col>"
        ));
    }
    src.push_str("</View>");
    while src.len() < 1200 {
        src.push_str("<Extra>x</Extra>");
    }
    let mut rng = Rng(11);
    for max in [3, 17, 90] {
        check("limits", &src, tight(), &mut rng, max);
    }
}

#[test]
fn edits_after_streaming_resync_and_streaming_continues() {
    let lim = Limits::default();
    let mut art = Artifact::create("", lim);
    art.append("<View><Col><Text>one</Text>");
    art.apply_edits(&[views_ir::Edit {
        find: "one".into(),
        replace: "uno".into(),
    }])
    .unwrap();
    assert_eq!(art.full_reparses(), 1);
    let mut reference = art.doc().clone();
    for chunk in ["<Text>two", "</Text></Col>", "</View>"] {
        let patches = art.append(chunk);
        let (next, expected) = reference.reparse(art.source(), &lim);
        assert_eq!(art.doc(), &next);
        assert_eq!(patches, expected);
        reference = next;
    }
    assert_eq!(art.full_reparses(), 1, "streaming resumed after the edit");
}

#[test]
fn creating_with_text_equals_a_fresh_parse() {
    for (name, src) in corpus() {
        let art = Artifact::create(&src, Limits::default());
        assert_eq!(art.doc(), &Doc::parse(&src, &Limits::default()), "{name}");
    }
}

#[test]
fn a_chunk_that_repeats_an_open_element_keeps_its_id() {
    // The open card is completed and an identical card begins in one chunk. Its ID used to
    // move to the new card, whose content equals what the open card had so far.
    let lim = Limits::default();
    let a = "<View><Col><Card title=\"a\"><Text>x</Text>";
    let b = "<Text>y</Text></Card><Card title=\"a\"><Text>x</Text>";
    let card = |d: &Doc| d.nodes.values().find(|n| n.tag == "Card").map(|n| n.id);
    let mut art = Artifact::create(a, lim);
    let id = card(art.doc()).unwrap();
    art.append(b);
    assert_eq!(art.doc().nodes[&id].span.start, 11);
    let (full, _) = Doc::parse(a, &lim).reparse(&format!("{a}{b}"), &lim);
    assert_eq!(full.nodes[&id].span.start, 11);
}
