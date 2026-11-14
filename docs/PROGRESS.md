# Progress

A successor session can resume from this file alone.

## Step 0: prove push, build and see (2026-11-14)

Status: reported to Omar, waiting on the CI item before any code is written.

| Item | Result |
| --- | --- |
| Repo | Created isonomic-org/isonomic-views, private, with README. |
| Push | Works. Branch `step-0/connector-check` and commits made through the connector. |
| Build and see | Works. The pinned component library (0.9.1) renders offscreen on Linux; light and dark screenshots were written and looked at. |
| File size | Rule adopted: every file under 20 KB. |
| CI | Blocked. See below. |

### Offscreen rendering recipe

- Ubuntu 24.04, Rust 1.97, software Vulkan (package `mesa-vulkan-drivers`). No display server needed.
- Headless app context with the platform's headless renderer, open a hidden window, render a frame, capture a screenshot, save a PNG.
- Cold build of the component library: about 9.5 minutes on 2 cores. A render run: under 1 second.
- Text renders in the sandbox's fallback font, so screenshots will not match macOS type exactly.

### CI: what is missing

1. The connector cannot write workflow files (403 on `.github/workflows/ci.yml`).
2. The shared workflow repo isonomic-org/.github does not exist yet, so there is nothing to call.

Omar needs to add this file by hand as `.github/workflows/ci.yml`, once the shared workflow exists:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
  rust:
    uses: isonomic-org/.github/.github/workflows/rust-ci.yml@main
```

### Note on authorship

Connector commits are recorded as `Omar <omar@isonomic.com>`. The email is right; the name comes from the GitHub profile, which says "Omar", not "Omar Qadmiri".
