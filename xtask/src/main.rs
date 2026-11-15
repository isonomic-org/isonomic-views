//! Repository tasks: `cargo xtask <screenshots|report|guide|deps>`.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

/// The crates that must build and test anywhere, with no UI framework and no I/O.
const HEADLESS: &[&str] = &[
    "views-expr",
    "views-markup",
    "views-ir",
    "views-runtime",
    "views-catalog",
];

/// Everything the headless crates may depend on, transitively (normal dependencies).
const ALLOWED: &[&str] = &[
    "views-expr",
    "views-markup",
    "views-ir",
    "views-runtime",
    "views-catalog",
    "serde",
    "serde_core",
    "serde_derive",
    "serde_json",
    "itoa",
    "ryu",
    "zmij",
    "memchr",
    "proc-macro2",
    "quote",
    "syn",
    "unicode-ident",
    // serde_json's ordered maps, enabled through feature unification with the UI crates.
    "indexmap",
    "hashbrown",
    "equivalent",
    "foldhash",
];

/// Source patterns the headless crates may not contain.
const FORBIDDEN: &[&str] = &[
    "std::net",
    "std::fs",
    "std::process",
    "std::env",
    "TcpStream",
    "UdpSocket",
    "Command::new",
    "File::open",
    "File::create",
    "env::var",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

fn main() -> ExitCode {
    let task = std::env::args().nth(1).unwrap_or_default();
    let ok = match task.as_str() {
        "screenshots" => screenshots(),
        "report" => report(),
        "guide" => guide(),
        "deps" => deps(),
        _ => {
            eprintln!("usage: cargo xtask <screenshots|report|guide|deps>");
            false
        }
    };
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn cargo() -> Command {
    let mut c = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    c.current_dir(root());
    c
}

fn corpus() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(root().join("corpus"))
        .map(|d| {
            d.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "view"))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// Render every corpus view offscreen in light and dark into target/screenshots.
fn screenshots() -> bool {
    let out = root().join("target/screenshots");
    let mut c = cargo();
    c.args([
        "run",
        "--locked",
        "--quiet",
        "-p",
        "views-gpui",
        "--features",
        "headless",
        "--bin",
        "views-shots",
        "--",
    ])
    .arg(&out)
    .args(corpus());
    c.status().is_ok_and(|s| s.success())
}

/// Timings for a 500-node view and streamed patches.
fn report() -> bool {
    let mut c = cargo();
    c.args([
        "run",
        "--locked",
        "--quiet",
        "--release",
        "-p",
        "views-gpui",
        "--features",
        "headless",
        "--bin",
        "views-report",
    ]);
    c.status().is_ok_and(|s| s.success())
}

/// Write the authoring guide and the JSON Schema to target/generated. Both are generated from
/// the catalog at any time (`views_catalog::authoring_guide`, `views_catalog::json_schema`), so
/// they are not committed.
fn guide() -> bool {
    let examples: Vec<(String, String)> = views_catalog::GUIDE_EXAMPLES
        .iter()
        .map(|n| {
            (
                n.to_string(),
                fs::read_to_string(root().join("corpus").join(format!("{n}.view")))
                    .unwrap_or_default(),
            )
        })
        .collect();
    let refs: Vec<(&str, &str)> = examples
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let guide = views_catalog::authoring_guide(&refs);
    let schema =
        serde_json::to_string_pretty(&views_catalog::json_schema()).unwrap_or_default() + "\n";
    let dir = root().join("target/generated");
    let mut ok = fs::create_dir_all(&dir).is_ok();
    for (name, text) in [
        ("authoring-guide.txt", guide),
        ("catalog.schema.json", schema),
    ] {
        ok &= fs::write(dir.join(name), &text).is_ok();
        println!("wrote target/generated/{name} ({} bytes)", text.len());
    }
    ok
}

/// The headless crates' dependency closure must stay inside the allow-list, and their source
/// must not touch the network, files or processes.
fn deps() -> bool {
    let out = match cargo()
        .args(["metadata", "--locked", "--format-version", "1"])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => {
            eprintln!("cargo metadata failed");
            return false;
        }
    };
    let meta: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_default();
    let mut names: BTreeMap<String, String> = BTreeMap::new();
    for p in meta["packages"].as_array().into_iter().flatten() {
        names.insert(
            p["id"].as_str().unwrap_or("").into(),
            p["name"].as_str().unwrap_or("").into(),
        );
    }
    let mut graph: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for n in meta["resolve"]["nodes"].as_array().into_iter().flatten() {
        let deps = n["deps"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|d| {
                d["dep_kinds"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|k| k["kind"].is_null())
            })
            .filter_map(|d| d["pkg"].as_str().map(String::from))
            .collect();
        graph.insert(n["id"].as_str().unwrap_or("").into(), deps);
    }
    let mut ok = true;
    for krate in HEADLESS {
        let Some(start) = names
            .iter()
            .find(|(_, n)| n == krate)
            .map(|(id, _)| id.clone())
        else {
            eprintln!("{krate}: not found in the workspace");
            ok = false;
            continue;
        };
        let mut seen = BTreeSet::new();
        let mut stack = vec![start];
        while let Some(id) = stack.pop() {
            if seen.insert(id.clone()) {
                stack.extend(graph.get(&id).cloned().unwrap_or_default());
            }
        }
        let bad: Vec<&String> = seen
            .iter()
            .filter_map(|id| names.get(id))
            .filter(|n| !ALLOWED.contains(&n.as_str()))
            .collect();
        if bad.is_empty() {
            println!("{krate}: {} crates in the closure, all allowed", seen.len());
        } else {
            eprintln!("{krate}: dependencies outside the allow-list: {bad:?}");
            ok = false;
        }
        let src = root().join("crates").join(krate).join("src");
        for file in walk(&src) {
            let text = fs::read_to_string(&file).unwrap_or_default();
            for pat in FORBIDDEN {
                if text.contains(pat) {
                    eprintln!("{}: uses {pat}", file.display());
                    ok = false;
                }
            }
        }
    }
    ok
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
    out
}
