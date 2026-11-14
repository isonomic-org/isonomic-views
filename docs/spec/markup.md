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
| Open elements | Closed, with an `unclosed-tag` warning. |

Malformed input becomes diagnostics with byte spans (`stray-close`, `unquoted-value`,
`stray-lt`, `duplicate-attribute`, `bad-attribute`, `too-deep`, `empty-expression`). A closing
tag that matches an ancestor closes everything above it. Nesting deeper than 128 is ignored.
The scanner is iterative and linear, so hostile input cannot overflow the stack.
