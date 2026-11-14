# Progress

A successor session can resume from this file alone.

## Plan (dependency order, one stacked pull request each)

1. Workspace, docs, formula language (views-expr)
2. Streaming parser (views-markup)
3. Node map, stable IDs, patches, limits, artifact operations (views-ir)
4. Runtime: scopes, reactive cache, actions, host (views-runtime)
5. Catalog, JSON Schema, authoring guide, validator (views-catalog)
6. Renderer (ui, views-gpui)
7. Playground (views-playground)
8. Corpus, xtask (screenshots, report, guide, deps), gate tests

Each pull request is based on the previous one, so they merge in order.

## Where things stand

See docs/status.md. Pull request 1 is open.

## Environment notes

- Pushing: the GitHub connector (`push_files`) works for normal files. It cannot write
  `.github/workflows/*` (403); workflow files are added through the github.com web editor.
- Rendering on Linux: install `mesa-vulkan-drivers libxkbcommon-dev libwayland-dev
  libxcb1-dev libx11-xcb-dev libasound2-dev libfontconfig-dev libfreetype-dev libvulkan-dev`.
  A cold build of the component library takes about 10 minutes on 2 cores.
- Shared CI: when `isonomic-org/.github/.github/workflows/rust-ci.yml` exists, replace the jobs
  in `.github/workflows/ci.yml` with `uses: isonomic-org/.github/.github/workflows/rust-ci.yml@main`.
