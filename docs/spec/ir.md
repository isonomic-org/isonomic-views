# Node map, IDs, patches and limits (views-ir)

## The node map

`Doc { root, nodes: BTreeMap<NodeId, Node>, next_id, diagnostics }`. Each node has a kind
(`element`, `text`, `expr`), a tag, literal `props`, expression `bindings`, text, children,
parent, source span and `complete` (the source for the node has fully arrived: an element was
explicitly closed, text was ended by a tag or an expression). The map serializes to JSON with
serde.

## Stable IDs

IDs are never derived from position. Each re-parse is aligned against the previous map, parent
by parent, and old IDs are carried over:

1. Children with an explicit `key` match the old child with the same tag and key.
2. The rest are aligned in order by a weighted longest common subsequence: an identical subtree
   (same content fingerprint, and the old node complete) scores 3, the same tag scores 1;
   different tags, or two different keys, never match. Ties go to the earliest pairs.
3. Identical subtrees that moved out of order are matched afterwards (they become moves).
4. Anything unmatched gets a fresh ID. IDs are never reused within a document's life.

So: editing one element changes only that element; inserting a sibling does not shift the
others' IDs; and a source that only grows keeps every node it had, at its place, however the
chunks fall. (Only complete nodes count as identical because a node still being written can
equal a later sibling: `Val` streaming into `Value {x}Val` must not hand its ID to the second
text.)

## Streaming

`Artifact::append` does not re-parse the whole source. `markup::Stream` keeps the scanner's
state and scans again only from the start of the construct the previous end of the source cut
off (text, a literal body, an opening tag, or anything held back). The builder keeps, for each
open element, its node and how many of its final children it has lowered; a step lowers only
the children finished since the last step, updates the open elements (their span grows) and
the text still being written, and emits patches in `diff` order. Complete subtrees are never
looked at again.

The result is exactly what a full re-parse aligned against the previous map gives, map and
patches (property tests over the corpus, random chunk sizes and tight limits). If the parser
state is not what the step expects, it falls back to that full re-parse; the one known case is
a duplicate `key` attribute that rewrites a node's key while its tag is still being written.
Edits and any change that is not text added at the end re-parse in full, then rebuild the
streaming state, so streaming can continue after them.

## Patches

`insert`, `remove`, `move`, `set_prop`, `set_text`, plus `set_meta` (span and completeness,
not visible). `diff(old, new)` emits removes first (only the top of each removed subtree), then
in new document order: inserts, moves and prop or text changes. Moves are minimal: siblings on
a longest increasing subsequence of old positions do not move. `apply(doc, patches)` replays
them; tests check `apply(old, diff(old, new)) == new` across streaming and random edits.

## Limits

`Limits` (defaults): 2,000 nodes, depth 32, 256 KB source, 16 KB per text, 500 list items,
20,000 expression steps, 8 ms render time per frame. The builder enforces nodes, depth, source
and text by cutting, with `limit-*` diagnostics; the runtime enforces list and steps; the
renderer enforces render time.
