---
format: aep.planning-md/3
id: release-plan:release-0-29-0
kind: release-plan
status: draft
title: 'Publish Metaharness 0.29.0: the 0.29.0 wave'
summary: hook refusals reach the record, catalogue road remainder, H11 claims as measured, aep-eval scope through Substrate
relations:
- supersedes: release-plan:release-0-28-0
revision: 1
---
## Why this release

The 0.29.0 wave delivers five stories in four units on `integration/wave-0-29-0`, approved in
`approval-record:wave-0-29-0`:

- U1 `story:claude-hook-refusals-reach-the-record`, `story:claude-seam-abstain-is-recorded-as-vendor-decided`:
  the Claude hook program's own refusals are recorded as its deny, an abstain names the vendor pipeline as
  decider, and a41 § 11 is written.
- U2 `story:catalogue-road-leftovers`: under ask a catalogue miss, or an entry nothing performs, is denied
  without the embedder, naming the catalogue; the catalogue open does not block on a FIFO. The `rev` pin is
  withdrawn.
- U3 `story:h11-claims-match-what-was-measured`: Claude's `ok` H11 says it is the canary row for its version
  and names every unprobed surface; a silently copied canary row fails a named check.
- U4 `story:aep-eval-scope-through-substrate`: the AEP eval takes its delegated scope from Substrate's
  `systemd` adapter (substrate-host 0.19.2, by rev), with no `systemd-run` of its own.

The ESS pin stays at 0.86.0, because ESS 0.87.0 was not released before the gate.

## Change

- Workspace version `0.28.0` to `0.29.0`: Cargo.toml and the 13 workspace entries of Cargo.lock.
- The CHANGELOG `[0.29.0]` section.
- The README status line.

## Procedure

As in `release-plan:release-0-28-0`:
1. The local staged full gate runs once on the integration branch.
2. One PR goes through the bot.
3. After CI is green, the bot fast-forwards `main` and pushes the annotated tag `0.29.0`.
4. The release artifacts are checked and scanned, then the bot publishes the release.
