---
format: aep.planning-md/3
id: release-plan:release-0-27-0
kind: release-plan
status: draft
title: 'Publish Metaharness 0.27.0: the wave of 2026-11-04'
summary: 'four stories: failed b10x cause, aep 0.79.0, bounded version probes, actor on every launch'
relations:
- supersedes: release-plan:release-0-26-2
revision: 1
---
## Why this release

The wave of 2026-11-04 delivers four stories on one integration branch
(`integration/wave-2026-11-04`), approved in `approval-record:wave-2026-11-04`:

- `story:b10x-failed-run-names-its-cause`: a failed b10x run names its cause in `session.ended`
  (Harness 0.23.0's failed stop; unverified at the 0.22.0 pin).
- `story:aep-pin-0-77-1`: the AEP crates link aep 0.79.0, so a run with `--approver` gets past an
  operator step.
- `story:version-probe-everywhere-bounded`: `doctor` and every b10x launch read `--version` under the
  probe deadline.
- `story:actor-reaches-every-launch`: `--actor` reaches the Codex child and is refused on b10x.

The ESS pin stays at 0.77.0.

## Change

Workspace version `0.26.2` to `0.27.0` (Cargo.toml, the 13 workspace entries of Cargo.lock),
the CHANGELOG `[0.27.0]` section, the README status line.

## Procedure

As in `release-plan:release-0-26-0`: the local staged full gate runs once on the integration branch
(with ess 0.77.0 first on PATH); one PR through the bot; after CI, the bot fast-forwards `main` and
pushes the annotated tag `0.27.0`; the artifacts are checked and scanned, then the bot publishes the
release.
