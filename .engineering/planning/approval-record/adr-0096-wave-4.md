---
format: aep.planning-md/3
id: approval-record:adr-0096-wave-4
kind: approval-record
status: draft
title: 'Wave 4 of 2026-10-20: ESS 0.66.0, Claude Code 2.1.304, Codex record rows, release 0.11.0'
tags:
- wave
refs:
- provider: atlas
  reference: adr-0096
- provider: atlas
  reference: adr-0098
relations:
- decides: story:ess-pin-0-66-0
- decides: story:claude-code-2-1-304-declared
- decides: story:codex-strict-record-rows
- decides: release-plan:release-0-11-0
revision: 1
---
## Status

Proposed 2026-10-20 by session `metaharness`; approved the same day by `b10x-chef-orchestrator` under the
owner's standing exception for ADR 0096–0101 work (ADR 0096 and 0098). Skill: aep:implementing 0.23.2,
wave mode. Integration tree `wt-ea1727bb9559`, branch `integration/adr-0096-wave-4`, base `6b984c46`
(`main`, wave 3). Pace (Timo, 2026-10-20): one sub-agent at a time; one unit implemented, reviewed and
merged before the next.

## Units

| unit | story | depends_on | release need |
|---|---|---|---|
| 1 | `story:ess-pin-0-66-0` | none | carried by 0.11.0 (Timo: each release moves the ESS pin to the newest release) |
| 2 | `story:claude-code-2-1-304-declared` | none (sequential before 3: shared `audit.rs`, `FloorInputs`) | takt pins it |
| 3 | `story:codex-strict-record-rows` | none | lands what invariant 3 allows (D1 = B) |
| — | `release-plan:release-0-11-0` | after unit 3 | tag and GitHub Release by the bot |

Each unit: `aep:implementor` in its own managed tree, then `aep:adversary` (at most 2 passes), the
coordinator verifies the last correction; the whole gate on the integration branch before close.

## Proposal and approval

Proposal: `metaharness-wave-adr0096/w4/proposal.md` in the operator's cache directory (2026-10-20, about
23:00). `b10x-chef-orchestrator`, 2026-10-20 (verbatim, first lines): "Wave 4 is approved: U1
ess-pin-0-66-0, U2 claude-code-2-1-304-declared, U3 codex-strict-record-rows, then release 0.11.0. Basis:
exception 5, ADR 0096 and 0098 … D1 for U3: B, no change to invariant 3 or a13. Strict stays exit 3 on
Codex; takt's Codex trials keep hermetic `on`. Reason: A changes a design invariant for one vendor, and
nothing now needs Codex strict. I record A as open for Timo, not as yours to decide later. U3 still lands
the record rows the invariant allows."

Approved commits: the opening store commit; per unit one commit and one merge into the integration
branch (formatting-only fixes where a merge needs one); the closing store commit; push, PR and bot
fast-forward of `main` after CI; release 0.11.0 through the repository's process. Not approved: a paid
run.

## Decisions

- D1 (`b10x-chef-orchestrator`): B. No change to invariant 3 or amendment a13; unit 3 lands H1b from the
  record (the launch writes `personality` and the row compares the record with the plan), the H2
  attestation fix (`hooks.json` is not written), and states why H1a, H4 and H5 stay `unk` on Codex. Option A
  (decide them from the launch's own observation) is open for Timo.
- D2 (coordinator): unit 2's H1a rule for built-in plugins: a built-in passes only when matched on
  `source == "<name>@builtin"` and on the pinned binary's compiled list; takt's operator-login list (four,
  `cc-plugin-sec-default` the fourth) is pinned as a vector.

## Log

- Scoping (2026-10-20): `aep:story-scoper` on `story:claude-code-2-1-304-declared` (12 compiled
  built-ins, 3 in a credential-free scratch home, takt's record 4) and on `story:codex-strict-record-rows`
  (0.153.4 rollout carries only `personality`; H1a and H5 under a ChatGPT login not provable from the
  scratch home); ESS 0.66.0 probe (validate passes; 3 `ESS-SYNTH-022` refusals).
- `story:session-request-carries-context-files` filed as a draft on Timo's idea relayed by
  `b10x-chef-orchestrator` (22:3x); to be scoped and placed as unit 4 before 0.11.0 if it is one unit,
  otherwise first in wave 5; its depends_on and acceptance go to `b10x-chef-orchestrator` before it is
  implemented.
