# Node map, IDs, patches and limits (views-ir)

## The node map

`Doc { root, nodes: BTreeMap<NodeId, Node>, next_id, diagnostics }`. Each node has a kind
(`element`, `text`, `expr`), a tag, literal `props`, expression `bindings`, text, children,
parent, source span and `complete` (the element was explicitly closed). The map serializes to
JSON with serde.

## Stable IDs

IDs are never derived from position. Each re-parse is aligned against the previous map, parent
by parent, and old IDs are carried over:

1. Children with an explicit `key` match the old child with the same tag and key.
2. The rest are aligned in order by a weighted longest common subsequence: an identical subtree
   (same content fingerprint) scores 3, the same tag scores 1, different tags never match.
3. Identical subtrees that moved out of order are matched afterwards (they become moves).
4. Anything unmatched gets a fresh ID. IDs are never reused within a document's life.

So: streaming only appends and every complete node keeps its ID; editing one element changes
only that element; inserting a sibling does not shift the others' IDs.

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
