# Progress

A successor session can resume from this file alone.

## Milestone 0: merged

All eight pull requests (#1 to #7 and #11) are merged into `main`, in order. The final report is
the comment on #11; every deviation is in docs/status.md.

## Milestone 1, in this order

| Step | Issue | State |
| --- | --- | --- |
| 1. Switch `ui` from gpui-kit to isonomic-ui | #12 | merged (#14) |
| 2. Incremental re-parse while streaming | #8 | merged (#15) |
| Shared state pushed to viewers | #9 | on hold until the engine has a shared-state store |
| macOS CI | #10 | closed: folds into the shared CI's macOS job |

Both are merged. CI now runs the shared `rust-ci.yml` (#13, the master session's pull request)
with `ui: true`. #9 waits for the engine's shared-state store.

## Working rules from Omar (November 14, 2026)

- Merge your own pull requests once every check is green. A change to a shared contract (the
  session event schema once frozen, the SDK wire interface, or `isonomic-org/.github`) gets the
  label `needs-master` and waits for the master session to merge it.
- Merge the master session's CI pull request in this repo once it is green.
- Before each new task, handle open issues labelled `master` first: that is how the master
  session reaches this repo.

## Decisions from the master session (November 14, 2026)

- Shared CI: `isonomic-org/.github` has `rust-ci.yml`; `.github/workflows/ci.yml` calls it.
  Its `ui: true` input checks out isonomic-ui next to the repository at the commit in `UI_REF`
  with the `UI_DEPLOY_KEY` secret, the same sibling layout the Desktop and Capsule repos use
  for the SDK.
- Isonomic UI: depend on the `isonomic-ui` crate pinned to a git revision (now `UI_REF`, with
  a path dependency on the sibling checkout). Catalog icon names map to `IconName` in one
  table and theme reads stay in `views-gpui/src/style.rs`.
- `Cargo.lock` is exempt from the 20 KB rule. Commit it, pushing with git from Omar's Mac when
  the connector can't take it. The shared workflow does not pass `--locked`; `cargo xtask`
  does.
- Host and SharedStore for Chat artifacts: the engine side (grants, persistence, push) is the
  Desktop session's; binding artifacts to the SDK is the Chat session's. Grants are per
  artifact, start empty, are always shown to the user, and are decided and stored by the
  engine, never by the artifact. Keep the fixture host for tests.

## Environment notes

- The repo needs the private `isonomic-org/isonomic-ui` checked out next to it at `UI_REF`.
  CI reads it with the read-only deploy key in the `UI_DEPLOY_KEY` secret.
- Building in the cloud sandbox: it cannot reach isonomic-ui directly. Make a git bundle of the
  pinned commit on Omar's Mac in `~/Code/isonomic/views-share` (a folder shared with the
  session), stage it in, and clone it to `isonomic-ui` next to the working copy.
- Pushing: Omar's Mac has a clone at `~/Code/isonomic/repos/isonomic-views`. Source
  `~/Code/isonomic/repos/.views-env.sh` first: it routes git over HTTPS through `gh` (signed in
  as QadmiriOmar) and sets the commit identity to Omar Qadmiri <omar@isonomic.com>. The Mac's
  global git identity and keychain belong to a different account; never use them. Bring changes
  over as a patch through the shared folder and `git apply` them there.
- The connector (`push_files`) still works for small files but cannot write
  `.github/workflows/*` and fails on files over about 25 KB. Check runs are not readable through
  it; `mergeable_state` from `pull_request_read` (`clean`) shows checks are green.
- Rendering on Linux: install `mesa-vulkan-drivers libxkbcommon-dev libwayland-dev
  libxcb1-dev libx11-xcb-dev libasound2-dev libfontconfig-dev libfreetype-dev libvulkan-dev`.
