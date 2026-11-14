# Status

What is built, newest first. Every deviation from the kickoff brief is stated here.

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
