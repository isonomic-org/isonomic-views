# Artifact contract

How the chat runtime creates and updates a view. The chat runtime itself is out of scope for
Milestone 0; this is the contract it will call. The Rust side is `views_ir::Artifact`,
validated with `views_catalog::validate`, rendered with `views_gpui::ViewPane`.

## Model-facing operations

The assistant writes views through two tool calls. Both carry markup, never a node map.

### `view_create`

```json
{ "title": "Seat cost calculator", "source": "<View version=\"0.1\" ...>...</View>" }
```

- `source` streams. The host feeds each arriving chunk to `Artifact::append` and applies the
  returned patches to the pane in the same frame. Any prefix renders: the valid part paints,
  incomplete constructs are held back, open tags are closed.
- When the call finishes, the host runs `views_catalog::validate` on the final document and
  returns the result to the model (see Results).
- The artifact ID is assigned by the host and returned in the result.

### `view_update`

```json
{ "id": "art_123", "edits": [ { "find": "max={500}", "replace": "max={1000}" } ] }
```

- Edits apply in order. Each `find` must occur exactly once in the source as it stands after
  the previous edits; the match is exact, including whitespace.
- All or nothing: if any edit fails, nothing changes and the error names the edit's index.
- The new source is re-parsed and aligned to the old node map; untouched nodes keep their IDs,
  so their component state (scroll, focus, input text, local state) survives. Only the
  resulting minimal patches reach the renderer.
- `local` and `shared` values are kept. New fields in `<View local={{...}}>` or
  `shared={{...}}` are added with their starting value; existing values are not reset.
- Large rewrites are allowed but discouraged: a model should prefer several small edits.
  Replacing the whole source is `Artifact::replace_source` and still keeps matching IDs.

## Results returned to the model

```json
{
  "ok": false,
  "id": "art_123",
  "version": 7,
  "catalog": "0.1",
  "diagnostics": [
    { "code": "unknown-prop", "severity": "warning", "message": "<Slider> has no prop 'maxx'; it is ignored",
      "span": { "start": 120, "end": 168 }, "node": 9, "hint": "did you mean 'max'?" }
  ]
}
```

- `ok` is false when any diagnostic is an `error`. Warnings do not block rendering.
- Spans are byte offsets into the current source. `hint` is present when the fix is clear.
- An update that fails returns `{ "ok": false, "error": "not_found" | "ambiguous" | "empty" |
  "too_large", "index": n, "count": n? }` and the unchanged version.
- The model repairs by issuing `view_update` with edits that address the diagnostics.

## What the model is told

The system prompt for a view-capable assistant includes the generated authoring guide
(`docs/authoring-guide.md`, produced from the catalog). It names every component, prop,
function, action, token and limit, and includes corpus examples. The JSON Schema
(`docs/catalog.schema.json`) describes the same catalog for tools that want structure.

## Host obligations

- Provide a `Host`. The default denies everything; grant `data`, `query`, `call`, `open` and
  `image` per view according to the user's settings, and log every grant.
- Provide one `SharedStore` per artifact, shared by all its viewers.
- Keep `local` state per viewer and discard it when the viewer closes the view.
- Never fetch a URL on a view's behalf. Views cannot name URLs; images come only from assets
  the host resolves.
