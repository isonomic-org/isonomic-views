# Markup and the streaming parser (views-markup)

## Grammar

```
element   = "<" Name attr* ( "/>" | ">" child* "</" Name ">" )
attr      = name ( "=" ( "\"text\"" | "'text'" | "{" expression "}" ) )?
child     = element | text | "{" expression "}" | "<!--" comment "-->"
```

- Tag names are case-sensitive. A bare attribute means `true`.
- Text: whitespace runs collapse to one space; whitespace-only runs are dropped; a single
  boundary space is kept when it is not part of a line break (so `Total: {x}` keeps its gap).
  Entities: `&lt; &gt; &amp; &quot; &apos; &lbrace; &rbrace; &nbsp;`.
- Expression braces are matched across nested brackets and string literals.

## Never failing

`parse(src)` returns a tree for every input, including every prefix of a document:

| At the end of the input | Behaviour |
| --- | --- |
| Inside a tag name (`<Sli`) | The element is held back. |
| Inside an attribute or its value | The element is shown with its complete attributes; the partial one is held back. |
| Inside `{expression}` text | Held back. |
| Inside a closing tag | Ignored; the element stays open. |
| `<!` or `<!-` | Held back: it may become a comment. |
| Inside text, or a literal body | Shown, marked not complete: more of it may follow. |
| Open elements | Closed, with an `unclosed-tag` warning. |

Malformed input becomes diagnostics with byte spans (`stray-close`, `unquoted-value`,
`stray-lt`, `duplicate-attribute`, `bad-attribute`, `too-deep`, `empty-expression`). A closing
tag that matches an ancestor closes everything above it. Nesting deeper than 128 is ignored.
The scanner is iterative and linear, so hostile input cannot overflow the stack.

## Streaming

`Stream` parses a source that grows at the end. The scanner's state (open elements with their
final children, final top-level nodes, diagnostics) persists between calls; whatever the end of
the input cut off is kept apart and scanned again from its first byte when more arrives, so
each call costs the new text plus the construct that was cut off. `parse` is a stream fed once,
and tests check that a stream fed in random chunks shows the same tree as `parse` at every
step.
