# Isonomic Views: specification (Milestone 0)

Isonomic Views renders a small declarative document into native Isonomic UI components, live,
while the document is still being written. It is the engine behind artifacts in Isonomic Chat,
and later live blocks in documents, dashboards over tables and third-party apps. It is a
general engine, not a chat feature.

A view does the job of an HTML block with no browser: no DOM, no CSS cascade, no JavaScript.
It paints while it streams, binds to live data, keeps state (including state shared between
viewers), has no ambient authority, and looks native in light and dark because it can only use
design tokens.

Area specs:

| Area | File | Crate |
| --- | --- | --- |
| Formula language | [spec/expressions.md](spec/expressions.md) | views-expr |
| Markup and the streaming parser | [spec/markup.md](spec/markup.md) | views-markup |
| Node map, IDs, patches, limits | [spec/ir.md](spec/ir.md) | views-ir |
| State, bindings, actions, host | [spec/runtime.md](spec/runtime.md) | views-runtime |
| Catalog, schema, guide, validator | [spec/catalog.md](spec/catalog.md) | views-catalog |
| Renderer and playground | [spec/renderer.md](spec/renderer.md) | views-gpui, views-playground |
| Create and update operations | [artifact-contract.md](artifact-contract.md) | (contract for the chat runtime) |

Status, newest first, with every deviation: [status.md](status.md). Resume point for a new
session: [PROGRESS.md](PROGRESS.md).

## Crates

```
views-expr        formula language. Depends on no other crate here.
views-markup      streaming, never-failing parser.
views-ir          node map, stable IDs, patches, diffing, serialization, limits.
views-runtime     state scopes, reactive cache, bindings, actions, the Host trait.
views-catalog     registry, prop schemas, JSON Schema, authoring guide, validator.
ui                the single re-export of the component library.
views-gpui        renderer: keyed reconciler from the node map to components.
views-playground  source on the left, live render on the right, diagnostics, replay.
xtask             screenshots, report, guide generation, dependency allow-list.
```

The first five crates do not depend on the UI framework and build and test anywhere. They
contain no network, file or process access; CI checks their dependency closure against an
allow-list (`cargo xtask deps`).

Only `ui` names the component library. It re-exports Isonomic UI (`isonomic-ui`), pinned to one
git revision of its main branch; moving the pin is a deliberate change made together with
`Cargo.lock`. Inside the renderer, only `views-gpui/src/style.rs` reads the library's theme or
names its icons.

## Decisions (from the kickoff brief)

1. **Source is markup a model already writes well.** Tags are components, attributes are props,
   `class` takes a defined subset of utility classes, `{...}` holds expressions.
2. **The parser never fails.** Every prefix of every document yields a usable tree. Errors are
   diagnostics with positions.
3. **The internal form is a flat node map keyed by stable ID.** IDs do not change as text
   arrives or when an unrelated part of the source is edited. Changes are patches.
4. **Two ways to write: create (stream) and update (find/replace edit).** See the contract.
5. **State in three scopes**: `local` (per viewer, not saved), `shared` (saved, identical for
   everyone, behind a storage trait), `data` (read from the host).
6. **Expressions are pure and total**, with a step limit and a reactive dependency graph.
7. **Actions are declarative.** Host actions go through the host trait, which denies by default.
8. **A versioned catalog** generates a JSON Schema and an authoring guide; the validator emits
   machine-readable diagnostics.
9. **Tokens only.** Arbitrary colors and sizes are rejected.
10. **Hard limits** on nodes, depth, text, list length, expression steps and render time.
