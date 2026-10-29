---
format: aep.planning-md/3
id: approval-record:wave-d221
kind: approval-record
status: draft
title: 'Wave D221: ride-along Codex live follow-ups, ESS 0.77.0, release 0.24.1'
tags:
- non-interactive
- wave
relations:
- decides: task:ess-pin-0-77-0
revision: 1
---
## Wave D221

Approved by `b10x-chef-orchestrator` under the delegated grant (class C: a wave, PR, merge and release
through the repository's bot route; a dependency upgrade), decision D221. Quote: "run one wave through
/aep:implementing with the active stories story:caller-supplied-catalogue-on-every-kind and
story:independent-documentation. Let the store-only branch plan/codex-live-20261029 (8e40e0b) ride along
... Move the ESS crates and pin to 0.77.0 in the same wave ... Release through the repository's own
process."

Non-interactive: run as a sub-agent of the orchestrator, with no operator turn.

- `story:caller-supplied-catalogue-on-every-kind`: already `implemented` on `main` (wave D193, release
  0.24.0, `b2558e7`). The orchestrator read a primary checkout behind `origin/main`. No unit is
  dispatched.
- `story:independent-documentation`: already `implemented` on `main` (re-checked in wave D193). No unit
  is dispatched.
- Ride-along: `plan/codex-live-20261029` at `8e40e0b` (story:codex-catalogue-live-follow-ups, D209),
  merged into `integration/d221`. Not store-only: it adds one test in
  `crates/metaharness/tests/caller_catalogue.rs` and corrects two website pages.
- `task:ess-pin-0-77-0`: ESS 0.77.0 ride-along.

Commits this record authorises: the ride-along merge, the ESS pin commit, the closing store commit and
the release commit on `integration/d221`, one PR to `main` through the bot, and release 0.24.1 by the
repository's own process.
