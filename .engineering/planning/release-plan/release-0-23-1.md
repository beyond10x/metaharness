---
format: aep.planning-md/3
id: release-plan:release-0-23-1
kind: release-plan
status: draft
title: 'Publish Metaharness 0.23.1: ESS 0.76.0'
summary: Specification toolchain moves to ESS 0.76.0; no public surface change
revision: 2
---
## Why this release

Wave D193, approved under the delegated grant: the stories `story:caller-supplied-catalogue-on-every-kind`
and `story:independent-documentation`, plus the ESS pin as a ride-along.

- `story:caller-supplied-catalogue-on-every-kind` was dropped as blocked. Its open Codex arm is a live,
  spending probe and goes to `decision-blocker:codex-catalogue-live-probe`.
- `story:independent-documentation` was dropped as done: the live site and redirects were verified and
  the story moved to `implemented`.
- `task:ess-pin-0-76-0`: ESS 0.76.0 is the specification toolchain.

## Version

`0.23.1`. The only change is the specification toolchain pin, which touches no public Rust API, wire
format or command-line surface. Loom's session executor pins `0.23.0` by git tag and needs no change.

## Procedure (as for 0.23.0)

1. On `integration/d193`: "Release 0.23.1" holding `CHANGELOG.md`, the workspace version, the
   `Cargo.lock` workspace members, the `README.md` status line and this record. The full gate runs on
   that tree.
2. Open the PR through the bot. After CI passes, the bot fast-forwards `main` and pushes the annotated
   tag `0.23.1`.
3. Publish: check the Release workflow's four archives and `SHA256SUMS` by run id, run `b10x-gates
   scan-artifact` on both Linux executables, and have the bot publish the GitHub Release.
4. Write the announcement and run `verify-release.sh metaharness 0.23.1 <file>`.
