---
format: aep.planning-md/3
id: approval-record:adr-0096-wave-6
kind: approval-record
status: draft
title: 'Wave 6 of 2026-10-21: the Claude tool-call seam fails closed'
tags:
- wave
refs:
- provider: atlas
  reference: adr-0096
relations:
- decides: story:claude-seam-fails-closed
revision: 3
---
## Status

Approved 2026-10-21 by `b10x-chef-orchestrator` relaying the operator's "A", under exception 5 of the
owner's standing rules (ADR 0096–0101 work). Skill: aep:implementing 0.23.2, wave mode. Unit tree
`wt-e872ac68f40d`, branch `unit/claude-seam-hook-fails-closed`, base `3bc0de4` (the wave-5 integration
head). Integration branch `integration/adr-0096-wave-6`, created from the final wave-5 integration head.

## Units

| unit | story | commit | merge | review passes |
|---|---|---|---|---|
| 1 | `story:claude-seam-fails-closed` | `cacb923` | `5ddebbd` | 1 (`aep:security-reviewer`; its record waits until release 0.12.0 is out) |

Unit 1: `aep:implementor`, then `aep:security-reviewer` (at most 2 passes; 1 used); the coordinator verified
the last correction; the whole gate on the integration branch before close.

## Proposal and approval

Conditions (from `b10x-chef-orchestrator`, 2026-10-21):

- Wave 5 is not pushed until this fix is in the same push; this wave starts from the wave-5 integration
  head, and the TLS dependency update (wave 5 unit 7) rides the same push.
- Security-fix rule: pushed text gets one CHANGELOG line and no further detail until the release is out.
- The amendment's path and the unit's head sha go to the orchestrator before the merge.
- The combined head sha and the gate result go to the orchestrator before any push.
- Release 0.12.0 follows this wave and needs the orchestrator's word: the combined head sha, the gate
  result and the short release notes first.

## Decisions

- D1 (coordinator, accepted by the orchestrator): unit 1 ran in parallel with wave 5 unit 4; every cargo
  build went through the shared build slot.
- D2 (coordinator, accepted for the order only): the 0.12.0 release trigger is written after this wave
  merges, so one push carries waves 5 and 6.

## Log

- Unit 1 committed `cacb923` (91 test binaries, 1054 passed, 0 failed, 48 ignored; conformance 103/103);
  security review: confirmed, no change needed. Amendment path and head sha sent to `b10x-chef-orchestrator`
  before the merge.
- `integration/adr-0096-wave-6` created at the wave-5 head `875fc71`; unit 1 merged `5ddebbd` (parents
  `875fc71`, `cacb923`).
- Combined gate: task check, one step at a time, on `integration/adr-0096-wave-6` at `5ddebbd`: 6 steps exit 0,
  96 test binaries, 1128 passed, 0 failed, 49 ignored, conformance 103/103 on ESS 0.67.0 (gate log
  `combined-5-test.log` in the wave's cache directory `metaharness-wave-adr0096/w6/combined-gate`).
- Dash check at `5ddebbd`: the test binaries run in a `debian:stable-slim` container: claude lib 136 passed,
  0 failed, 4 ignored; the unit's review file 3 passed; all 10 named cases ok (`run-combined.out` in
  `metaharness-wave-adr0096/w6/dash`).
- State at close: not pushed. Waves 5 and 6 go to `main` in one push; the combined head sha and the gate result
  go to `b10x-chef-orchestrator` first. Release 0.12.0 is held for its word.
