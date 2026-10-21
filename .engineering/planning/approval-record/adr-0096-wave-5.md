---
format: aep.planning-md/3
id: approval-record:adr-0096-wave-5
kind: approval-record
status: draft
title: 'Wave 5 of 2026-10-21: H11 rests on the imposed control, hook quoting, session close, caller catalogue, loopback TLS, website'
tags:
- wave
refs:
- provider: atlas
  reference: adr-0096
- provider: atlas
  reference: adr-0098
relations:
- decides: story:claude-h11-memory-walk-shapes
- decides: story:hook-program-quotes-channel-paths
- decides: story:session-close-decides-absent-fields
- decides: story:caller-supplied-catalogue-on-every-kind
- decides: story:loopback-tls-leg-tested
- decides: story:website-drops-npm-leftovers
revision: 1
---
## Status

Proposed 2026-10-21 by session `metaharness`; approved the same day by `b10x-chef-orchestrator` with
conditions, under exception 5 of the owner's standing rules (ADR 0096–0101 work) for the wave and exception 6
for the release-tree cleanup. Skill: aep:implementing 0.23.2, wave mode. Integration tree `wt-a9e1df7e09eb`,
branch `integration/adr-0096-wave-5`, base `16340162` (`main`, release 0.11.0). Pace: one sub-agent at a
time; one unit implemented, reviewed and merged before the next.

## Units

| unit | story | commit | merge | adversary passes |
|---|---|---|---|---|
| 1 | `story:claude-h11-memory-walk-shapes` | — | — | — |
| 2 | `story:hook-program-quotes-channel-paths` | — | — | — |
| 3 | `story:session-close-decides-absent-fields` | — | — | — |
| 4 | `story:caller-supplied-catalogue-on-every-kind` | — | — | — |
| 5 | `story:loopback-tls-leg-tested` | — | — | — |
| 6 | `story:website-drops-npm-leftovers` | — | — | — |

Each unit: `aep:implementor` in its own managed tree, then `aep:adversary` (at most 2 passes), the
coordinator verifies the last correction; the whole gate on the integration branch before close.

## Proposal and approval

Proposal: `metaharness-wave-adr0096/w5/proposal.md` in the operator's cache directory (2026-10-21).
`b10x-chef-orchestrator`, 2026-10-21 (verbatim, first line): "metaharness: wave-5 proposal approved with
conditions. Basis: exception 5 in ~/beyond10x/CLAUDE.md (ADR 0096 work) for items 1-4; exception 6 for the
cleanup you named."

Approved: the opening store commit (units 1-6 draft → proposed → active, this record,
`release-plan:release-0-11-0` → implemented with the ESS 0.67.0 correction); one commit and one merge per
unit into the integration branch, one implementor at a time, at most 2 adversary passes, no paid run; the
closing store commit; push, PR and bot fast-forward of `main` after CI (Gate, MSRV 1.98, Security and
privacy, Build documentation); `worktree gc --apply --id wt-0612033e7f95`, then `git branch -d
release/0.11.0` (`-d` only).

Conditions (verbatim):

- "Unit 1 changes the H11 result (ok, gap, walk shapes) and flips an ESS scenario. Before its merge, list
  every consumer of the H11 result in beyond10x and report. If a consumer breaks, stop unit 1 and come to me."
- "Unit 2 refuses non-UTF-8 scratch roots. Report which callers can reach that refusal."
- "Unit 4 is the release trigger. Release 0.12.0 at close is NOT approved by this message. Come back with the
  release PR, the notes in the short shape (summary, Breaking, Changed at most 8 lines each, CHANGELOG section
  link), and the ESS pin status. I approve or hold then."
- "Live launch stays unverified. No paid run."
- "Stop and report if any unit needs a change to a documented contract outside ADR 0096-0101 scope."

## Decisions

- D1 (coordinator): unit 2 takes option A (POSIX `'\''` quoting of the channel paths and of the vendor hook
  `command`, control characters escaped in the Codex TOML) plus a named refusal of a non-UTF-8 scratch root
  before spawn. The root comes only from `TMPDIR` (`builder.rs:847`, `:1128`).
- D2 (coordinator): unit 4's per-kind vectors are synthetic; the live launch is labelled unverified.
- Not in this wave: `story:claude-task-text-mentions` (needs a native measurement on both binaries),
  `story:inner-workings-modelled-in-ess`, `story:b10x-record-keeps-the-loops-subjects` (not scoped), and the
  rest of the ADR 0098 chain after unit 4.

## Log

- Cleanup (exception 6): `wt-0612033e7f95` gc'd (removed, archive kept); `release/0.11.0` deleted with `-d`
  against its upstream `origin/main` (the primary checkout's `main` is behind, so `-d` against `HEAD` refused).
