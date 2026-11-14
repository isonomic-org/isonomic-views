# Status

What is built, newest first. Every deviation from the kickoff brief is stated here.

## 2026-11-14 · Isonomic UI replaces GPUI Kit (Milestone 1, issue #12)

Built: `crates/ui` now re-exports `isonomic-ui`, pinned to commit `5cdf134` of its main
branch; no file names gpui-kit any more. Every theme read in the renderer goes through
`style::tokens` (a `Tokens` struct), and catalog icon names map to the library's `IconName` in
one table, `style::icon`, with a test that every catalog icon has an entry, so a later theme or
icon change touches only `views-gpui/src/style.rs`. `Cargo.lock` is committed, seeded from
Isonomic UI's own lockfile so shared dependencies resolve to the versions it tests, and CI
builds with `--locked`. CI fetches the private library over SSH with a read-only deploy key
(`UI_DEPLOY_KEY`), set up by `.github/ui-access.sh`. Every test passes, and all 49 corpus views
were rendered light and dark on the new library and looked at.

Notes:
- The switch needed no source changes beyond `ui`: the renamed fork keeps the kit's paths
  (`ui::component`, `ui::base`), token names and icon file names. The refactor into `Tokens`
  and the icon table is what keeps the next change to one file.
- Screenshots now use the fonts Isonomic UI bundles, so the Linux fallback-font deviation
  below no longer applies.
- The catalog still allows the same 64 icons. Isonomic UI embeds more; adding names is a
  catalog change plus one line each in the icon table.
- `Cargo.lock` is over 20 KB and is pushed with git from Omar's Mac, as agreed with the master
  session; source and doc files stay under 20 KB, and CI now checks that.

## 2026-11-14 · Playground (views-playground) and the Milestone 0 gate

Built: `cargo run -p views-playground` with the source on the left (edits re-render live), the
view on the right, diagnostics with line and column below, four corpus examples, and "Replay as
a stream", which feeds the current source in six characters every 16 ms. An offscreen test
renders the playground; its screenshot was looked at.

Gate, as measured on the build machine (Linux, release build, Mesa software renderer):
`cargo xtask report` draws a 641-node view in 11.3 ms (parse 1.2, evaluate 1.2, build 0.9,
layout and paint 8.0) against a 16.7 ms frame, and streams it at 0.88 ms per node against
1 ms. The per-node margin is thin because each chunk re-parses the whole source.

Deviations:
- No macOS CI job. CI runs on Linux; the Mac build is the same code but is not exercised in CI.
- The pull requests carry the issue briefs (Context, Acceptance criteria, Files in scope, Out
  of scope) in their descriptions instead of separate GitHub issues.

## 2026-11-14 · Corpus and xtask

Built: 49 corpus views (calculators, forms, dashboards, explainers, trackers, quizzes) that
together use every component, every chart type, all three scopes and all eight actions;
corpus tests (every view validates with no warnings; every prefix of every view parses and
complete nodes keep their IDs; patches reproduce the map; a one-line edit gives exactly one
visible patch); and `cargo xtask` with `screenshots` (every view, light and dark, offscreen),
`report` (timings), `guide` (authoring guide and JSON Schema into `target/generated/`) and
`deps` (the headless crates' dependency allow-list and a scan for network, file, process and
environment access). Every screenshot was looked at before this was opened.

Deviations:
- The corpus comes before the playground in the PR order, because the playground's examples
  are corpus files.
- Screenshots render text with the font the Linux machine has, not the font on a Mac, so
  glyphs differ slightly from what a Mac shows.
- `Cargo.lock` was not committed in Milestone 0 (it is over the 20 KB limit). It is committed
  from Milestone 1 on; see the entry above.

## 2026-11-14 · Renderer (ui, views-gpui)

Built: the `ui` crate, the only place GPUI Kit is named (pinned `=0.9.1`); `ViewPane`, a
component that takes a source whole, streamed or as edits and renders it with real kit
components; a keyed reconciler that keeps component state (input text, slider, select, date
picker, selected tab, open sections) per node ID and loop instance; mapping of tones, palette,
spacing, radii and class utilities onto the theme; a frame clock that cuts a slow frame at the
render budget; a visible placeholder for unknown tags; headless rendering for screenshots and
timings. Tests (offscreen, real components): a slider move changes the dependent Stat in the
same frame; two panes over one store share `shared` and keep their own `local`; typed text and
the slider keep their state entities across an edit elsewhere; every streamed prefix renders
and the slider keeps one ID; a slow view is cut by the time budget; an unknown tag shows a
placeholder.

Deviations:
- The slider test sets the slider's value and emits its change event, rather than simulating a
  pointer drag; the drag itself is the kit's code.
- Charts have their appear animation turned off; animation is out of scope for this milestone.
- `Kbd` key caps are drawn from theme tokens, because the kit's `Kbd` only takes keys known at
  compile time.
- The headless crates are the workspace's default members, so `cargo test` alone needs no GPU;
  CI runs the whole workspace with rendering on Mesa's software Vulkan driver.

## 2026-11-14 · Catalog, schema, guide and validator (views-catalog)

Built: catalog 0.1 with 42 components (each with typed props, events, accessibility role,
allowed children and an example); design tokens (tones, palette, spacing and size steps,
heights, icons); the class-utility subset (layout and spacing, token values only); a JSON
Schema for the tree form; a plain-text authoring guide generated from the catalog; and a
validator that checks tags, placement, children, props, literal values, classes, bindings,
actions and every name and function used, with "did you mean" hints. Diagnostics serialize
as JSON (`code`, `severity`, `message`, `span`, `node`, `hint`).

Deviations:
- Two sub-tags beyond the brief's list: `Tab` (inside `Tabs`) and `Section` (inside
  `Accordion`), so each page or section is ordinary markup with its own children.
- The icon list is the part of the component library's icon set that its default asset bundle
  embeds (64 names). Other names render as a neutral square rather than nothing.
- The generated guide and schema are produced by `cargo xtask guide` into `target/generated/`
  and are not committed; the tests check they come from the catalog.

## 2026-11-14 · State and actions (views-runtime)

Built: the three scopes (`local` per viewer; `shared` behind the `SharedStore` trait, with an
in-memory store; `data` read from the host), a reactive cache keyed by node, prop and loop
instance that records the state paths each value read, so a write drops only overlapping
entries; `For` frames that map loop variables back to stored items, so `bind={it.done}`
writes the right list item; the eight actions; and the `Host` trait, whose every method denies
unless the host grants it. Expression errors, denied requests and failed actions become
diagnostics. Tests: a slider write re-evaluates exactly one binding; two viewers share
`shared` and keep their own `local`; the default host denies data, query, call, open and
images; state survives a document update.

Deviation: `shared` changes reach other viewers when they call `sync()` (the renderer does at
the start of each frame), not by push. A collaborative store can notify later without changing
the trait.

## 2026-11-14 · Node map, IDs and patches (views-ir)

Built: lowering from markup to a flat node map; stable IDs by aligning each re-parse against
the previous map (explicit keys, then a weighted longest common subsequence over content
fingerprints, then identical subtrees out of order); patches (insert, remove, move, set_prop,
set_text, set_meta) with minimal moves; apply; hard limits on nodes, depth, source, text,
lists, expression steps and render time; `Artifact` for create (append) and update (edits that
must each match exactly once, all or nothing). Tests: every prefix of a sample keeps the IDs of
complete nodes and patches reproduce the map; a one-line edit gives one visible patch; 400
random edits round-trip through patches; each limit.

Deviation: patches carry a sixth kind, `set_meta`, for source spans and completeness. It
changes nothing on screen and `Patch::is_visible` filters it out; without it the map and the
patched map would drift apart.

## 2026-11-14 · Streaming parser (views-markup)

Built: an iterative, linear scanner that returns a tree for every input. Incomplete tag names,
attributes, expressions and closing tags at the end are held back; open tags are closed;
malformed input becomes diagnostics with byte spans; nesting is bounded at 128. Tests: every
prefix of a sample, malformed input, incomplete constructs, a 30,000-case fuzz.

Deviation: the parser knows two tags. `Markdown` and `Code` keep their line breaks, and `Code`
content is literal (no tags, no expressions), the way HTML treats `pre` and `script`. Without
it, code samples would need every brace escaped.

## 2026-11-14 · Formula language (views-expr)

Built: lexer, parser (depth-bounded, incomplete-aware errors), values, bounded evaluator with a
step budget, list, text, date, aggregate and formatting functions, lambdas as arguments,
dependency extraction, action sequences. Tests: unit, behaviour, 20,000-case deterministic
fuzz of parser and evaluator.

Deviations:
- Formatting is English-only (`1,234.50`, `$`, `Nov 14, 2026`). Locales come with the host.
- Dates have no time of day or zone in this milestone.

## 2026-11-14 · Step 0

Repo created (private, README). Connector push works. GPUI Kit 0.9.1 renders offscreen on Linux
with Mesa's software Vulkan driver; screenshots were looked at. The shared workflow repo
`isonomic-org/.github` does not exist yet and the connector cannot write workflow files.

Deviation: CI runs a self-contained `ci.yml` in this repo until the shared workflow exists.
Switching is a one-line change (see docs/PROGRESS.md).
