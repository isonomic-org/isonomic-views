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

The batch the master session approved (Milestone 0, then these two) is merged. Every merge from
here on needs Omar's go-ahead in chat. Next: switch CI to `isonomic-org/.github`'s
`rust-ci.yml` once the Desktop session creates it; #9 waits for the engine's shared-state store.

## Decisions from the master session (November 14, 2026)

- Shared CI: the Desktop session creates `isonomic-org/.github` with `rust-ci.yml` (inputs:
  apt-packages, extra-steps, upload-artifact-path, rust-toolchain, a macOS job toggle). Switch
  `.github/workflows/ci.yml` to it with a one-line change once it exists.
- Isonomic UI: depend on the `isonomic-ui` crate pinned to a git revision. Catalog icon names
  map to `IconName` in one table and theme reads stay in `views-gpui/src/style.rs`.
- `Cargo.lock` is exempt from the 20 KB rule. Commit it, pushing with git from Omar's Mac when
  the connector can't take it, and use `--locked` in CI.
- Host and SharedStore for Chat artifacts: the engine side (grants, persistence, push) is the
  Desktop session's; binding artifacts to the SDK is the Chat session's. Grants are per
  artifact, start empty, are always shown to the user, and are decided and stored by the
  engine, never by the artifact. Keep the fixture host for tests.

## Environment notes

- The repo needs the private `isonomic-org/isonomic-ui`. CI reads it with the read-only deploy
  key in the `UI_DEPLOY_KEY` secret (`.github/ui-access.sh`).
- Building in the cloud sandbox: it cannot reach isonomic-ui directly. Make a git bundle of the
  pinned commit on Omar's Mac in `~/Code/isonomic/views-share` (a folder shared with the
  session), stage it in, clone it bare, and point git at it with
  `git config --global url."file:///<mirror>.git".insteadOf "https://github.com/isonomic-org/isonomic-ui"`
  plus `CARGO_NET_GIT_FETCH_WITH_CLI=true`. Cargo.lock still records the real URL.
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
