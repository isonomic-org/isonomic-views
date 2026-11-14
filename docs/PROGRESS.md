# Progress

A successor session can resume from this file alone.

## Pull requests (stacked; each is based on the one before, so they merge in order)

| # | Branch | Content | State |
| --- | --- | --- | --- |
| 1 | m0/01-expr | Workspace, docs, CI, formula language (views-expr) | open, CI green |
| 2 | m0/02-markup | Streaming parser (views-markup) | open, CI green |
| 3 | m0/03-ir | Node map, stable IDs, patches, limits, artifact operations (views-ir) | open, CI green |
| 4 | m0/04-runtime | Scopes, reactive cache, actions, host (views-runtime) | open, CI green |
| 5 | m0/05-catalog | Catalog, JSON Schema, authoring guide, validator (views-catalog) | open, CI green |
| 6 | m0/06-renderer | ui, renderer, live UI tests, screenshot and report tools (views-gpui) | open |
| 7 | m0/07-corpus | 49-view corpus, corpus tests, cargo xtask | open |
| 8 | m0/08-playground | Playground (views-playground), final docs | open |

Nothing is merged; Omar approves every merge. After a merge, the next pull request's base
becomes `main` on GitHub automatically when its base branch is deleted, or change it by hand.

## Milestone 0 gate

All seven items are met; the evidence is in the final report on pull request 8 and in
docs/status.md. `cargo xtask report` on the build machine (release, software renderer):
500-node view (641 nodes) parse, evaluate, build, layout and paint in 11.3 ms against a 16.7 ms
frame; 0.88 ms per streamed node against 1 ms.

## Next issues

1. Shared-store sync by notification instead of per-frame polling, and a collaborative store.
2. Incremental re-parse for streaming (re-parse only the open tail), to widen the per-node
   margin on large views.
3. A macOS CI job with screenshots on Metal, and committed reference screenshots.

## Environment notes

- Pushing: the GitHub connector (`push_files`) works for normal files. It cannot write
  `.github/workflows/*` (403); workflow files are added through the github.com web editor.
  Verify a push by comparing `git hash-object` (or `git write-tree` of a staged copy) with
  the tree and blob SHAs the connector reports.
- Check runs cannot be read through the connector (403); read them on the Actions page.
- Rendering on Linux: install `mesa-vulkan-drivers libxkbcommon-dev libwayland-dev
  libxcb1-dev libx11-xcb-dev libasound2-dev libfontconfig-dev libfreetype-dev libvulkan-dev`.
  A cold build of the component library takes about 10 minutes on 2 cores.
- Shared CI: when `isonomic-org/.github/.github/workflows/rust-ci.yml` exists, replace the jobs
  in `.github/workflows/ci.yml` with `uses: isonomic-org/.github/.github/workflows/rust-ci.yml@main`.
