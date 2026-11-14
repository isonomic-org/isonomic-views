//! Timings for `cargo xtask report`: parse, evaluate and build a 500-node view, and the cost
//! of a streamed patch per node.

use std::{rc::Rc, time::Instant};

use views_ir::{Artifact, Doc, Limits};
use views_runtime::{DenyAll, Frame, MemoryStore, Runtime};

/// A view of about 500 nodes: a grid of cards, each with a heading, a computed line, a badge
/// and a progress bar, all bound to shared state.
fn big_view() -> String {
    let mut s = String::from(
        "<View version=\"0.1\" shared={{ n: 7 }}>\n  <Grid cols={4} class=\"gap-3\">\n",
    );
    for i in 0..71 {
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

fn main() {
    let src = big_view();
    let lim = Limits::default();

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

    let mut cx = views_gpui::headless::context();
    let first = views_gpui::headless::render(&mut cx, &src, false, 1200.0, Rc::new(DenyAll))
        .expect("render");
    // A second pane in the same app, so renderer start-up is not counted.
    let shot = views_gpui::headless::render(&mut cx, &src, false, 1200.0, Rc::new(DenyAll))
        .expect("render");

    // Streaming: feed the source 24 bytes at a time, as a model would.
    let mut art = Artifact::create("", lim);
    let t = Instant::now();
    let mut patches = 0usize;
    let mut worst = 0f64;
    for chunk in src.as_bytes().chunks(24) {
        let c = Instant::now();
        patches += art.append(std::str::from_utf8(chunk).expect("ascii")).len();
        worst = worst.max(us(c));
    }
    let stream_us = us(t);

    let budget = 1e6 / 60.0;
    // Parse, evaluate and build the elements, then lay out and paint them. The warm frame
    // re-runs build, layout and paint for the same tree.
    let total = parse_us + eval_us + shot.build_us.0 as f64 + shot.warm_frame_us as f64;
    let per_node = stream_us / nodes as f64;
    let verdict = |ok: bool| if ok { "PASS" } else { "MISS" };
    println!("views report");
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
        "  streamed            {} chunks, {patches} patches, {stream_us:.0} us total, worst chunk {worst:.0} us",
        src.len().div_ceil(24)
    );
    println!(
        "  per streamed node   {per_node:>9.1} us  target 1000 us: {}",
        verdict(per_node < 1000.0)
    );
}
