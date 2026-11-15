//! Timings for `cargo xtask report`: parse, evaluate and build a view of about 500 nodes and
//! one at the 2,000-node limit, and the cost of streaming each one in.

use std::{rc::Rc, time::Instant};

use views_ir::{Artifact, Doc, Limits};
use views_runtime::{DenyAll, Frame, MemoryStore, Runtime};

/// A grid of `cards` cards, each with a heading, a computed line, a badge and a progress bar,
/// all bound to shared state: 9 nodes per card plus 2.
fn grid_view(cards: usize) -> String {
    let mut s = String::from(
        "<View version=\"0.1\" shared={{ n: 7 }}>\n  <Grid cols={4} class=\"gap-3\">\n",
    );
    for i in 0..cards {
        s.push_str(&format!(
            "    <Card title=\"Card {i}\">\n      <Text>Value {{shared.n * {i}}} of {{currency(shared.n * {i} * 12)}}</Text>\n      <Badge tone=\"info\">#{i}</Badge>\n      <Progress value={{(shared.n * {i}) % 100}} />\n    </Card>\n"
        ));
    }
    s.push_str("  </Grid>\n</View>\n");
    s
}

fn us(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e6
}

/// Bytes per streamed chunk, about what a model emits per token batch.
const CHUNK: usize = 24;
/// Per streamed node and worst single chunk, in microseconds.
const PER_NODE_TARGET: f64 = 300.0;
const WORST_CHUNK_TARGET: f64 = 1000.0;

struct Stream {
    chunks: usize,
    patches: usize,
    total_us: f64,
    worst_us: f64,
    full_reparses: u64,
}

/// Feed the source `CHUNK` bytes at a time, as a model would.
fn stream(src: &str, lim: Limits) -> Stream {
    let mut art = Artifact::create("", lim);
    let t = Instant::now();
    let mut patches = 0;
    let mut worst = 0f64;
    for chunk in src.as_bytes().chunks(CHUNK) {
        let c = Instant::now();
        patches += art.append(std::str::from_utf8(chunk).expect("ascii")).len();
        worst = worst.max(us(c));
    }
    Stream {
        chunks: src.len().div_ceil(CHUNK),
        patches,
        total_us: us(t),
        worst_us: worst,
        full_reparses: art.full_reparses(),
    }
}

fn verdict(ok: bool) -> &'static str {
    if ok { "PASS" } else { "MISS" }
}

fn section(cx: &mut ui::HeadlessAppContext, cards: usize, lim: Limits) {
    let src = grid_view(cards);
    let t = Instant::now();
    let doc = Doc::parse(&src, &lim);
    let parse_us = us(t);
    let nodes = doc.len();

    let t = Instant::now();
    let rt = Runtime::new(doc.clone(), Rc::new(DenyAll), MemoryStore::handle(), lim);
    let f = Frame::root();
    for id in doc.preorder() {
        let n = &doc.nodes[&id];
        for prop in n.bindings.keys().filter(|p| !p.starts_with("on")) {
            rt.eval(id, prop, &f);
        }
    }
    let eval_us = us(t);

    let first =
        views_gpui::headless::render(cx, &src, false, 1200.0, Rc::new(DenyAll)).expect("render");
    // A second pane in the same app, so renderer start-up is not counted.
    let shot =
        views_gpui::headless::render(cx, &src, false, 1200.0, Rc::new(DenyAll)).expect("render");
    let s = stream(&src, lim);

    let budget = 1e6 / 60.0;
    // Parse, evaluate and build the elements, then lay out and paint them. The warm frame
    // re-runs build, layout and paint for the same tree.
    let total = parse_us + eval_us + shot.build_us.0 as f64 + shot.warm_frame_us as f64;
    let per_node = s.total_us / nodes as f64;
    println!("  view                {nodes} nodes, {} bytes", src.len());
    println!("  parse               {parse_us:>9.0} us");
    println!(
        "  evaluate            {eval_us:>9.0} us  ({} evaluations)",
        rt.evals()
    );
    println!(
        "  build elements      {:>9} us  (again, warm cache: {} us)",
        shot.build_us.0, shot.build_us.1
    );
    println!(
        "  layout and paint    {:>9} us  (warm frame, software renderer)",
        shot.warm_frame_us
    );
    println!(
        "  total               {total:>9.0} us  target {budget:.0} us (one frame at 60 Hz): {}",
        verdict(total <= budget)
    );
    println!(
        "  first frame         {:>9} us  includes window creation and first glyph rasterization",
        shot.first_frame_us
    );
    println!(
        "  first app frame     {:>9} us  includes renderer start-up",
        first.first_frame_us
    );
    println!(
        "  streamed            {} chunks of {CHUNK} bytes, {} patches, {:.0} us total, {} full re-parses",
        s.chunks, s.patches, s.total_us, s.full_reparses
    );
    println!(
        "  per streamed node   {per_node:>9.1} us  target {PER_NODE_TARGET:.0} us: {}",
        verdict(per_node < PER_NODE_TARGET)
    );
    println!(
        "  worst chunk         {:>9.1} us  target {WORST_CHUNK_TARGET:.0} us: {}",
        s.worst_us,
        verdict(s.worst_us < WORST_CHUNK_TARGET)
    );
}

fn main() {
    let lim = Limits::default();
    let mut cx = views_gpui::headless::context();
    println!("views report");
    println!("about 500 nodes");
    section(&mut cx, 71, lim);
    println!("at the node limit");
    section(&mut cx, (lim.max_nodes - 2) / 9, lim);
}
