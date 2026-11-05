---
format: aep.planning-md/3
id: release-plan:release-0-30-0
kind: release-plan
status: draft
title: 'Publish Metaharness 0.30.0: the 0.30.0 wave'
summary: substrate-host 0.20.0 confinement-only, the cgroup root by prefix
relations:
- supersedes: release-plan:release-0-29-0
revision: 1
---
## Why this release

The 0.30.0 wave delivers two stories in one unit on `integration/wave-0-30-0`. The wave was approved in
`approval-record:wave-0-30-0`, and `approval-record:hold-h11-out-of-0-30-0` holds U2 out of it.

U1 covers `story:substrate-host-0-20-confinement-only` and `story:eval-cgroup-root-check-by-prefix`:
- `metaharness-aep-eval` takes `b10x-substrate-host` 0.20.0 by rev with `default-features = false`, and the
  lock drops 128 entries.
- The eval decides "below the cgroup root" by canonical component prefix, as Substrate's probe does.

`story:h11-not-ok-on-unprobed-memory` is not in this release. The ESS pin stays at 0.86.0, because ESS 0.87.0
was not tagged before the gate.

## Change

- Workspace version `0.29.0` to `0.30.0`: Cargo.toml and the 13 workspace entries of Cargo.lock.
- The CHANGELOG `[0.30.0]` section.
- The README status line.

## Procedure

As in `release-plan:release-0-29-0`:
1. The local staged full gate runs once on the integration branch.
2. One PR goes through the bot.
3. After CI is green, the bot fast-forwards `main` and pushes the annotated tag `0.30.0`.
4. The release artifacts are checked and scanned, then the bot publishes the release.
