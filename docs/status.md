# Status

What is built, newest first. Every deviation from the kickoff brief is stated here.

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
