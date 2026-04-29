# Governance

`sessionx` is a small open source project run by a single primary maintainer.
This document describes how decisions are made.

## Roles

- **Maintainer** — Has commit and admin rights. Reviews PRs, triages issues,
  cuts releases, and decides on scope.
- **Contributor** — Anyone who opens an issue or PR.

Current maintainers are listed in [`MAINTAINERS.md`](MAINTAINERS.md).

## Decision making

The project follows a **BDFL-lite** model: the primary maintainer has final
say on scope, design, and releases. In practice:

- Bug fixes and small improvements: any maintainer may merge after review.
- Behavior changes, new commands, or new config keys: discussed in a GitHub
  issue first; the primary maintainer signs off.
- Breaking changes: opened as a tracking issue with a migration plan before
  any PR lands.

Disagreements are resolved by discussion in the relevant issue or PR. If
consensus is not reached, the primary maintainer decides.

## Adding maintainers

A contributor may be invited to become a maintainer after a sustained record
of high-quality contributions and reviews. Invitations are at the discretion
of the existing maintainers.

## Removing maintainers

A maintainer may step down at any time by opening a PR removing themselves
from `MAINTAINERS.md`. Inactive maintainers (no activity for 12 months) may
be moved to an "emeritus" section by the primary maintainer.

## Project abandonment

If the primary maintainer becomes unable or unwilling to continue, they will
either:

1. Transfer ownership to another active maintainer, or
2. Mark the project as **archived** with a clear note in the README pointing
   to any active forks.

The repository will not be deleted.

## Changes to governance

Changes to this document are made via PR and require approval from the
primary maintainer.
