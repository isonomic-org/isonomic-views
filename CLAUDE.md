# isonomic-views

Engine for Isonomic Views. A Rust workspace. The spec is `docs/spec.md` and `docs/spec/`.

## Rules

1. **The parser never fails.** Every prefix of every document yields a tree. No panics anywhere
   on user input.
2. **IDs are stable.** A node's ID must not change as text streams in or when an unrelated part
   of the source is edited. Component state hangs off it.
3. **Headless crates stay headless.** views-expr, views-markup, views-ir, views-runtime and
   views-catalog never depend on the UI framework and contain no network, file or process
   access. `cargo xtask deps` enforces the allow-list.
4. **views-expr depends on no other crate here.**
5. **Only `ui` names the component library.** Nothing else in the repo names the framework or
   the library directly, and inside views-gpui only `style.rs` reads the theme or names icons.
6. **Tokens only.** No raw colors, sizes or durations in views or in the renderer's mapping.
7. **The host denies by default.** Everything a view needs from outside goes through `Host`.
8. **Hermetic tests.** No network, no real timers.
9. **Small files.** Every file under 20 KB.
10. **Names.** No other product, repo or account name anywhere.

## Workflow

- One task per branch and pull request; a merge needs green checks and Omar's approval.
- Before pushing: `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`,
  `cargo test`.
- Docs change with behaviour: update `docs/status.md` (newest first) and `docs/PROGRESS.md`.
- Look at every screenshot you ship (`cargo xtask screenshots`).

## Licence

- Repos stay private. This repository is licensed GPL-3.0-or-later: keep LICENSE at the root unmodified, keep `license = "GPL-3.0-or-later"` in every Cargo.toml, and start every new source file with `Copyright 2026 Anud Inc.` and `SPDX-License-Identifier: GPL-3.0-or-later`. Never remove an existing copyright or licence notice. Never rename or delete a repo.
