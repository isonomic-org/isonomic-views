# Renderer and playground (views-gpui, views-playground)

## ViewPane

`ViewPane` is a component. It owns an `Artifact` (source and node map), a `Runtime` and the
component state of every rendered node. Feed it source three ways: `set_source` (whole),
`append` (streaming) and `apply_edits` (find/replace update). Streaming parses only what is new
and edits re-parse aligned with the previous node map; either way IDs and therefore component
state survive (see [ir.md](ir.md)).

## The keyed reconciler

Each frame walks the node map from the root and builds elements through the `ui` crate.
`If` and `For` expand in place (no wrapper element). Component state lives in a map keyed by
`(node ID, loop instance)`:

| Component | State kept |
| --- | --- |
| TextField, NumberInput | input entity (text, cursor, focus) |
| TextArea | textarea entity |
| Slider | slider entity |
| Select | select entity (rebuilt only when the options change) |
| DatePicker | date picker entity |
| Tabs, Accordion | selected tab, open sections |

State for a node is dropped when the node leaves the document, and for loop instances that
were not rendered in the frame. Bound values flow in every render (the widget is updated only
when it differs); user changes flow out through `Runtime::write_bind` and the node's
`onChange` actions, then the pane redraws. Moving a slider updates every dependent binding in
the same frame; only bindings that read the changed path are re-evaluated.

## Mapping

Layout maps to flex and grid containers; utility classes map to the theme's spacing scale,
radii and colors (`style.rs`). Tones map to the theme's semantic colors, palette names to its
named colors and chart series. Fields fill the width of a column and share the width of a row;
buttons and toggles keep their natural size. An unknown tag renders as a visible placeholder.
`Code` and `Markdown` keep line breaks; `Code` content is literal. Charts have their appear
animation off (animation is out of scope for this milestone). `Kbd` key caps are drawn from
theme tokens because the library's `Kbd` only accepts keys known at compile time.

## Tokens and icons

`style.rs` is the only file that touches the library's theme or icon set. `style::tokens`
copies the theme values the renderer uses into a `Tokens` struct, and every other file draws
with `Tokens`; a theme rename in the library changes that one function. `style::icon` is the
one table from catalog icon names to the library's `IconName`; a test checks that every name in
the catalog's icon list has an entry. A name outside the catalog renders as `square`.

## Limits

The pane checks a `FrameClock` before each node. Past `Limits::max_render_us` the rest of the
frame is skipped, a notice is shown, and `truncated()` is true. Tests inject a fake clock.

## Headless rendering

With the `headless` feature, `headless::render` opens a hidden window, draws, captures a
screenshot and crops it to its content. `views-shots` renders files to PNG in light and dark;
`views-report` prints timings. On Linux this needs a Vulkan driver; Mesa's software driver is
enough.

## Playground

`cargo run -p views-playground`: source on the left (edits re-render live), the view on the
right, diagnostics with line and column below, example views from the corpus, and "Replay as a
stream", which feeds the current source in six characters every 16 ms. The playground grants
sample data (`fixtures::host`) so data-bound views show content.
