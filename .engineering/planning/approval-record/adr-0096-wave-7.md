---
format: aep.planning-md/3
id: approval-record:adr-0096-wave-7
kind: approval-record
status: draft
title: 'Wave 7 of 2026-10-21: the trace formats beside trace-ir/1, decided per row'
relations:
- decides: story:retire-trace-formats-beside-trace-ir
revision: 4
---
## Status

Approved 2026-10-21 by `b10x-chef-orchestrator` under exception 5 of the owner's standing rules (ADR
0096–0101 work): the wave 7 commits, push, pull request and the bot fast-forward of `main`. Not a
release. Skill: aep:implementing 0.23.2, wave mode. Tree `mh-w7-trace-projections`, base `00cd0f8`
(`main`), branch `integration/adr-0096-wave-7`.

## Units

| unit | story | commit | review passes |
|---|---|---|---|
| 1 | `story:retire-trace-formats-beside-trace-ir` | `d54df1d` | adversary pass 1 (10 findings), revision 1; `review-result:adr0096-w7u1-review`, outcome fixed |
| 2 | `story:spec-auditor-pairing-checked-before-spawn` | `79567eb`, merged `1976f88` | adversary pass 1 (4 findings), revision 1; `review-result:adr0096-w7u2-review`, outcome fixed |
| 3 | `story:malformed-steering-line-answered-by-id` | `bddd79c` | adversary pass 1 (5 findings), revision 1; `review-result:adr0096-w7u3-review`, outcome fixed |
| 4 | `story:metaharness-live-scope-documented` | `f8756da`, merged `19f7e64` | none: documentation and one test |

Unit 1: the per-row decision in the story body; then `trace-ir-summary/1` declared in
`spec/domains/trace.yaml`, produced in `metaharness-protocol`, exposed as `metaharness project --to
trace-ir-summary`, with one conformance scenario per recorded source run by `ess_conformance.rs`.
`aep:implementor`, then `aep:adversary`.

Units 2–4: three stories from a user trial of release 0.12.0, approved 2026-10-21 by
`b10x-chef-orchestrator` into wave 7 before its release, under exception 6 of the owner's standing rules
(they are not ADR work); unit 1 stays under exception 5. One managed tree each from `00cd0f8`
(`mh-w7-u2-spec-auditor`, `mh-w7-u3-malformed-steer`, `mh-w7-u4-live-docs`), merged into the integration
branch by the coordinator. Units 2 and 3: `aep:implementor` at xhigh, then `aep:adversary`. Unit 4 is
documentation and one test. Amendments: a43 unit 2, a44 unit 3.

Wave 8 drafts from the same trial: `story:mcp-arguments-checked-against-declared-types`,
`story:first-user-documents-agree-with-cli`.

## Proposal and approval

Sequencing from `b10x-chef-orchestrator` (2026-10-21):

1. Wave 7, one unit in metaharness: decide projection or retire per row, spec first.
2. Then hand the per-repository moves (harness, takt, loom) to the orchestrator as draft story bodies with
   their expected_before; metaharness does not edit those repositories.
3. If a consumer needs a metaharness crate to project from, name it; the orchestrator approves that
   release separately.

## Log

- 2026-10-21: story body carries the four row decisions; story moved to `active`.
- 2026-10-21: unit 1 implemented; adversary pass 1: 10 findings, revision 1 dispatched.
- 2026-10-21: units 2–4 stories created and moved to `active`; wave 8 drafts filed; units 2 and 3
  dispatched.
- 2026-10-22: units 1, 2 and 4 merged into the integration branch; unit 3 committed at `bddd79c` with
  the ESS conformance probe polling on `EventPoll::Progress`; `metaharness-aep` 93 passed, 0 failed.
- 2026-10-22: unit 3 merged at `ff8892c`; integration gate on that tree: 6 steps exit 0, 105 test binaries, 1210 passed, 0 failed, 49 ignored, conformance 107/107.
- 2026-10-22: wave 8 approved (`approval-record:adr-0096-wave-8`).
