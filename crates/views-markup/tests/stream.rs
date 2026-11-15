//! A stream fed in chunks shows exactly the tree a fresh parse of the same text gives.

use views_markup::{Cut, Node, Stream, parse};

/// Small deterministic generator, so the tests need no dependency and repeat exactly.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Cut points at char boundaries, `max` bytes apart at most.
fn chunk_ends(src: &str, rng: &mut Rng, max: usize) -> Vec<usize> {
    let mut ends = Vec::new();
    let mut at = 0;
    while at < src.len() {
        at = (at + 1 + rng.below(max)).min(src.len());
        while !src.is_char_boundary(at) {
            at += 1;
        }
        ends.push(at);
    }
    ends
}

fn check(name: &str, src: &str, rng: &mut Rng, max: usize) {
    let mut s = Stream::new();
    for end in chunk_ends(src, rng, max) {
        s.advance(&src[..end]);
        assert_eq!(
            s.document(),
            parse(&src[..end]),
            "{name}: stream differs from a parse at byte {end} (chunks up to {max})"
        );
    }
}

fn corpus() -> Vec<(String, String)> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../corpus");
    let mut out: Vec<(String, String)> = std::fs::read_dir(dir)
        .expect("corpus directory")
        .filter_map(|e| {
            let p = e.ok()?.path();
            (p.extension()? == "view").then(|| {
                let name = p.file_name()?.to_string_lossy().into_owned();
                Some((name, std::fs::read_to_string(&p).ok()?))
            })?
        })
        .collect();
    out.sort();
    out
}

/// Inputs that exercise every construct the end of the input can cut off.
const TRICKY: &[&str] = &[
    "<View><Code>let a = 1 < 2;\n</Cod</Code><Text>after</Text></View>",
    "<View>\n  <Markdown>\n# Title\n\n  two  spaces &amp; more\n</Markdown>\n</View>",
    "<View><!-- a comment with <tags> --><Text a=\"1\" a='2' b={x} c d=bad>hi</Text></View>",
    "<View>< not a tag {'<'} &lt;ok&gt; {  } <Text>x</Wrong></Text></View><Extra/>tail",
    "<View><Col><Text>one<Text>two</Col><Badge tone=\"info\" />\u{e9}t\u{e9} \u{1F600}</View>",
    "<View><Slider bind={shared.items.filter(i => i.name == \"}\")} min={[1, {a: 2}][0]} />",
    "lead text <View>\n<Text>unclosed\n",
];

#[test]
fn corpus_streams_match_fresh_parses() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for (name, src) in corpus() {
        for max in [1, 7, 24, 80] {
            check(&name, &src, &mut rng, max);
        }
    }
}

#[test]
fn tricky_streams_match_fresh_parses() {
    let mut rng = Rng(42);
    for (i, src) in TRICKY.iter().enumerate() {
        for max in [1, 2, 3, 5, 11] {
            for _ in 0..8 {
                check(&format!("tricky {i}"), src, &mut rng, max);
            }
        }
    }
}

#[test]
fn deep_nesting_streams_match() {
    let mut src = String::new();
    for i in 0..140 {
        src.push_str(&format!("<Col k=\"{i}\">"));
    }
    src.push_str("deep");
    for _ in 0..140 {
        src.push_str("</Col>");
    }
    let mut rng = Rng(7);
    for max in [3, 16, 64] {
        check("deep", &src, &mut rng, max);
    }
}

#[test]
fn settled_text_is_not_scanned_again() {
    let mut s = Stream::new();
    s.advance("<View><Text>a</Text><Text>b");
    assert_eq!(s.open().len(), 2);
    assert_eq!(s.open()[0].children.len(), 1, "the first Text is final");
    match s.cut() {
        Cut::Text(t) => assert!(!t.complete && t.text == "b"),
        other => panic!("{other:?}"),
    }
    s.advance("<View><Text>a</Text><Text>b</Text><Text c=\"1");
    assert!(matches!(s.cut(), Cut::Element(e) if e.tag == "Text" && e.attrs.is_empty()));
    let Node::Element(first) = &s.open()[0].children[0] else {
        panic!("element")
    };
    assert!(first.closed);
}
