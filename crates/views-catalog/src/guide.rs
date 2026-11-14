//! The authoring guide: plain text a model is given so it knows what it may write.

use std::fmt::Write as _;

use crate::{COMPONENTS, Category, Children, UNIVERSAL, VERSION, classes::PATTERNS, tokens};

/// Build the guide. `examples` are complete views (name, source) appended at the end.
pub fn authoring_guide(examples: &[(&str, &str)]) -> String {
    let mut g = String::new();
    let _ = writeln!(g, "ISONOMIC VIEWS: AUTHORING GUIDE (catalog {VERSION})");
    let _ = writeln!(g, "Generated from the catalog. Do not edit by hand.\n");
    g.push_str(RULES);
    g.push_str("\nCOMPONENTS\n");
    for cat in [
        Category::Layout,
        Category::Text,
        Category::Input,
        Category::Display,
        Category::Control,
    ] {
        let _ = writeln!(g, "\n## {}", cat.name());
        for c in COMPONENTS.iter().filter(|c| c.category == cat) {
            let kids = match c.children {
                Children::None => "no children".to_string(),
                Children::Text => "text children".to_string(),
                Children::Any => "any children".to_string(),
                Children::Only(t) => format!("children: {}", t.join(", ")),
            };
            let inside = c
                .parent
                .map(|p| format!(", only inside <{p}>"))
                .unwrap_or_default();
            let _ = writeln!(
                g,
                "\n<{}>  {}  [role {}; {kids}{inside}]",
                c.tag, c.doc, c.role
            );
            for p in c.props {
                let req = if p.required { " (required)" } else { "" };
                let lit = if p.literal_only { " literal only." } else { "" };
                let _ = writeln!(
                    g,
                    "  {:<12} {}{req}. {}{lit}",
                    p.name,
                    p.ty.describe(),
                    p.doc
                );
            }
            let _ = writeln!(g, "  e.g. {}", c.example);
        }
    }
    let _ = writeln!(g, "\nEvery element also accepts:");
    for p in UNIVERSAL {
        let _ = writeln!(g, "  {:<12} {}", p.name, p.doc);
    }
    g.push_str("\nFUNCTIONS (use inside { })\n");
    for (_, sig, doc) in views_expr::FUNCTIONS {
        let _ = writeln!(g, "  {sig:<40} {doc}");
    }
    g.push_str("\nACTIONS (event props such as onClick and onChange; separate several with ;)\n");
    for (_, sig, doc) in views_runtime_actions() {
        let _ = writeln!(g, "  {sig:<40} {doc}");
    }
    g.push_str("\nCLASS UTILITIES (the class prop; token values only, nothing in brackets)\n");
    for (pat, doc) in PATTERNS {
        let _ = writeln!(g, "  {pat:<44} {doc}");
    }
    let _ = writeln!(g, "\nTONES   {}", tokens::TONES.join(" "));
    let _ = writeln!(g, "PALETTE {}", tokens::PALETTE.join(" "));
    let _ = writeln!(g, "HEIGHTS xs sm md lg xl");
    let _ = writeln!(g, "ICONS   {}", tokens::ICONS.join(" "));
    g.push_str(LIMITS);
    if !examples.is_empty() {
        g.push_str("\nEXAMPLES\n");
        for (name, src) in examples {
            let _ = writeln!(g, "\n### {name}\n{}", src.trim_end());
        }
    }
    g
}

fn views_runtime_actions() -> &'static [(&'static str, &'static str, &'static str)] {
    views_runtime::ACTIONS
}

const RULES: &str = r#"WRITING A VIEW
A view is markup. Tags are components, attributes are props.
- The root is <View version="0.1">. Everything else goes inside it.
- name="text" is a literal. name={expression} is computed. A bare name means true.
- Text children may mix literal text and {expressions}: <Text>Total: {currency(local.sum)}</Text>.
  Write {'{'} and {'}'} for literal braces; &lt; &gt; &amp; also work.
- <Code> content is literal: braces and tags inside it are shown as written.
- Expressions are JavaScript-like: + - * / % == != < <= > >= && || ! ?: ?? . [] calls,
  [lists], {records}, and functions such as x => x.price for map, filter and friends.
  They are pure: they compute values and change nothing.
- State has three scopes:
    local   per viewer, not saved. Start it with <View local={{ tab: "a" }}>.
    shared  saved with the view and identical for everyone who sees it. <View shared={{ count: 0 }}>.
    data    read-only data the host provides, e.g. data.sales. It may be empty if not granted.
- bind={local.x} or bind={shared.x} connects an input to state both ways. Inside <For>,
  bind={item.done} edits that item in its stored list.
- Event props (onClick, onChange) take actions: onClick={set(local.n, local.n + 1); toggle(local.open)}.
  In onChange the new value is `event`.
- Use only catalog components, props, tones, palette colors, icons and class utilities.
  Colors, sizes, radii and durations come from tokens; raw values like #ff0000 or p-[13px] are rejected.
- There is no network, no file access and no script. Images and data come only from the host;
  a view cannot name a URL.
- Keep views small and readable. Prefer Card, Stat, Table and Chart over hand-built layouts.
"#;

const LIMITS: &str = r#"
LIMITS (a view that exceeds them is cut, not rejected)
  2000 nodes, 32 levels of nesting, 256 KB of source, 16 KB per text, 500 list items,
  20000 steps per expression, 8 ms of rendering per frame.
"#;
