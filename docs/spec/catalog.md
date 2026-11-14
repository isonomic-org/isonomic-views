# Catalog, schema, guide and validator (views-catalog)

## Catalog 0.1

A static registry: tag → category, description, typed props (required, literal-only), events
(props of type `Action`), accessibility role, children policy, required parent and an example.
Every element also accepts `class` and `key`. A view declares `<View version="0.1">`.

Components: layout (View, Row, Col, Grid, Card, Tabs + Tab, Accordion + Section, Divider,
Spacer, ScrollArea); text (Heading, Text, Markdown, Code, Badge, Kbd, Link); input (Button,
TextField, TextArea, NumberInput, Checkbox, Switch, Slider, Select, SegmentedControl,
DatePicker); display (Stat, Table, List, Progress, Chart (bar, line, area, pie), Icon, Avatar,
Callout, Tooltip, Skeleton, Spinner); control (If, For). `Tab` and `Section` are child tags of
Tabs and Accordion (a deviation recorded in status.md).

## Tokens

Tones (`neutral info success warning danger accent`), a named palette, spacing steps
(0 1 2 3 4 5 6 8 10 12 × 4 px), size steps for width and height, height words
(`xs sm md lg xl`), and a curated icon list. The class subset covers layout and spacing only;
anything else, including bracketed arbitrary values, is a `bad-class` error.

## Generated artifacts

- `json_schema()`: JSON Schema (2020-12) for the tree form of a view, one definition per
  component. Literal props are typed; expressions are `{"$expr": "..."}`.
- `authoring_guide(examples)`: plain text for a model: rules, every component and prop,
  functions, actions, class utilities, tokens, limits, and examples from the corpus.

Both are written to `docs/` by `cargo xtask guide`; a test fails if they are stale.

## Validator

`validate(&Doc)` and `check(src) -> Report { ok, catalog, diagnostics }`. Each diagnostic has
`code`, `severity`, `message`, `span`, `node` and an optional `hint`, and serializes to JSON.
Codes include `root-not-view`, `missing-version`, `unknown-component`, `misplaced`,
`invalid-child`, `bare-text`, `missing-prop`, `unknown-prop`, `bad-value`, `bad-class`,
`literal-required`, `url-not-allowed`, `expr-syntax`, `unknown-name`, `unknown-function`,
`bad-bind`, `bad-action`, and the parser's and limits' codes. Hints suggest the nearest
valid name.
