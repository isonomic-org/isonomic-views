# Formula language (views-expr)

Expressions appear between braces in markup: `value={currency(shared.seats * shared.price * 12)}`.

## Syntax

JavaScript-like, deliberately small:

- Literals: numbers, `'text'` or `"text"`, `true`, `false`, `null`, `[lists]`, `{records}`.
- Names: `local`, `shared`, `data`, loop variables, `event` (in actions), lambda parameters.
- Access: `a.b`, `a[0]`, `a["key"]`.
- Operators by precedence (low to high): `??`, `||`, `&&`, `== !=`, `< <= > >=`, `+ -`,
  `* / %`, unary `! -`. Conditional `c ? a : b`.
- Calls to named functions only. Lambdas `x => ...` and `(a, b) => ...` only as arguments.
- Action lists: `;`-separated calls (`parse_seq`).

Parsing never recurses deeper than 48 levels. A parse error says whether the input simply
ended early (`incomplete`), so a streaming caller holds the expression back instead of
reporting it.

## Semantics

- Pure and total: no side effects, no I/O, no unbounded loops. Every evaluation runs under
  `Limits { max_steps, max_list, max_text }`; each node visited and each list element touched
  costs a step. Exceeding a limit is an error value, never a hang or a panic.
- Values: null, boolean, number (f64), text, list, record, date (days since 1970-01-01).
- Missing data is forgiving: a missing field is null, arithmetic with null is null, and most
  functions return null for null input, so a half-streamed view shows blanks, not errors.
- `+` concatenates when either side is text; dates take `date + n` and `date - date`.
- Formatting is fixed to English conventions in this milestone (`1,234.50`, `$`, `Nov 14, 2026`).

## Standard functions

Math, text, lists and aggregates, dates, formatting and logic. The authoritative list with
signatures is `views_expr::FUNCTIONS`, which also generates the function section of the
authoring guide.

## Dependencies

`deps(expr)` returns the state paths an expression reads (`shared.seats`), cut at the first
dynamic index and excluding lambda parameters. The runtime's reactive cache is built on it.
