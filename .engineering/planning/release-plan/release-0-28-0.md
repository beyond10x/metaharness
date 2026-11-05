---
format: aep.planning-md/3
id: release-plan:release-0-28-0
kind: release-plan
status: draft
title: 'Publish Metaharness 0.28.0: the wave of 2026-11-05'
summary: bounded probes, b10x subjects, Claude H3 names its environment, test controls
relations:
- supersedes: release-plan:release-0-27-0
revision: 1
---
## Why this release

The wave of 2026-11-05 delivers seven stories in four units on `integration/wave-2026-11-05`, approved
in `approval-record:wave-2026-11-05`:

- U1 `story:aep-version-reads-bounded`, `story:doctor-run-help-probe-bounded`: every `--version` and
  `run --help` read goes through the public bounded probe `metaharness::probe_command`.
- U2 `story:b10x-record-keeps-the-loops-subjects`: a b10x `tool.requested` keeps the loop's subjects, read
  on the surface the loop published (amendment a75).
- U3 `story:claude-attestation-names-actor`, `story:vendor-claims-labelled`: the Claude H3 row names every
  variable the child gets beyond its allowlist; the vendor hook claims are cited or labelled.
- U4 `story:adversary-tests-have-positive-controls`, `story:cli-reference-effort-row-names-b10x`: positive
  controls in two adversary tests; the CLI reference's rows are checked against `check_spec`.

The ESS pin stays at 0.86.0.

## Change

Workspace version `0.27.0` to `0.28.0` (Cargo.toml and the 13 workspace entries of Cargo.lock), the
CHANGELOG `[0.28.0]` section, the README status line.

## Procedure

As in `release-plan:release-0-27-0`:
1. The local staged full gate runs once on the integration branch.
2. One PR goes through the bot.
3. After CI, the bot fast-forwards `main` and pushes the annotated tag `0.28.0`.
4. The release artifacts are checked and scanned, then the bot publishes the release.
