# State, bindings, actions and the host (views-runtime)

## Scopes

| Scope | Who sees it | Saved | Written by |
| --- | --- | --- | --- |
| `local` | one viewer | no | actions, binds |
| `shared` | every viewer, identically | yes, through `SharedStore` | actions, binds |
| `data` | every viewer | no (host-owned) | read-only |

Starting values come from `<View local={{...}} shared={{...}}>`. When the document changes,
new fields are added and existing values are kept. `SharedStore` has an in-memory
implementation (`MemoryStore`); a collaborative store implements the same trait later. Every
viewer holds the same store; `Runtime::sync` picks up other viewers' changes by version.

`data.<name>` is requested from the host once per name; a denied request yields null and a
`host-denied` diagnostic.

## Reactive cache

Each evaluated binding is cached per (node, prop, instance) with the state paths it read.
A write to a path invalidates only entries whose paths overlap it (one is a prefix of the
other). Inside `For`, the item variable maps to its stored path (`shared.items.3`) when the
list is a stored list, so `bind={item.done}` writes to the right place and dependencies are
precise. `Runtime::evals` counts cache misses; a test asserts that moving a slider re-evaluates
exactly the one binding that reads it.

## Actions

`set(path, value)`, `toggle(path)`, `append(path, item)`, `remove(path, index | x => cond)`,
`reset(path?)`, `query(name, args?, into)`, `call(name, args?, into?)`, `open(target)`.
Several actions are separated by `;`. `event` holds the new value in `onChange`.
Paths must name `local.*` or `shared.*` state (or a loop item standing for one).

## The host

`trait Host { data, query, call, open, image }`. Every method denies by default; `DenyAll`
is the default host. Views have no network, files, data or images of their own. `FixtureHost`
is an in-memory host for tests, screenshots and the playground.
