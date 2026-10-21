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
revision: 3
---
## Status

Proposed 2026-10-21 by session `metaharness`; approved the same day by `b10x-chef-orchestrator` with
conditions, under exception 5 of the owner's standing rules (ADR 0096–0101 work) for the wave and exception 6
for the release-tree cleanup. Skill: aep:implementing 0.23.2, wave mode. Integration tree `wt-a9e1df7e09eb`,
branch `integration/adr-0096-wave-5`, base `16340162` (`main`, release 0.11.0). Pace: one sub-agent at a
time; one unit implemented, reviewed and merged before the next.

## Units

Units ran in parallel from unit 2's first adversary pass on (`b10x-chef-orchestrator` rule 5, 2026-10-21: at
most 3 Opus or 5 Haiku agents at once), each in its own managed tree; merges went into
`integration/adr-0096-wave-5` in the order below.

| unit | story | commit | merge | adversary passes |
|---|---|---|---|---|
| 1 | `story:claude-h11-memory-walk-shapes` | `48668eb` | `4a72d8c` | 1 (`review-result:adr0096-w5-h11-adversary-1`) |
| 6 | `story:website-drops-npm-leftovers` | `7b96b12` | `83d15d0` | 0 (coordinator decision: two deleted config files, built site identical) |
| 3 | `story:session-close-decides-absent-fields` | `ae0b45f` | `bc3f398` | 1 (`review-result:adr0096-w5-session-close-adversary-1`) |
| 2 | `story:hook-program-quotes-channel-paths` | `aff22dc` | `6ad241c` | 2 (`review-result:adr0096-w5-hook-quoting-adversary-1`, `-2`) |
| 5 | `story:loopback-tls-leg-tested` | `5c4de3f` | `5b6d2b4` | 1 (`review-result:adr0096-w5-loopback-tls-adversary-1`) |
| 7 | TLS dependency update (no story) | `ceffb6d` | `3bc0de4` | 0; approved by `b10x-chef-orchestrator` under exception 6 |
| 4 | `story:caller-supplied-catalogue-on-every-kind` | `1bdc5b2` | `875fc71` | 2 (`review-result:adr0096-w5-catalogue-adversary-1`, `-2`) |

Across units: `review-result:adr0096-w5-independent-review` (10 read-only slices over units 1-5 and unit 4 at
fix round 1, 2026-10-21, on the operator's request).

Each unit: `aep:implementor` in its own managed tree, then `aep:adversary` (at most 2 passes), the
coordinator verified the last correction; the whole gate on the integration branch before close.

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
- Opening store commit `ca40260`.
- Unit 1 committed `48668eb` (gate 81 test binaries, 1012 passed, 0 failed, 49 ignored; conformance 103/103),
  merged `4a72d8c`; `b10x-chef-orchestrator` verified it and accepted D3. The H11 consumer list went to the
  orchestrator before the merge (condition 1).
- `b10x-chef-orchestrator` answered B to "close after unit 2 or run units 3-6": run units 3-6, then close; the
  push to `main` and release 0.12.0 need its word at close.
- Unit 6 committed `7b96b12` (82 binaries, 1014/0/49, 103/103; built site digest unchanged), merged `83d15d0`;
  no adversary pass (coordinator decision).
- Unit 3 committed `ae0b45f` (82 binaries, 1017/0/48, 103/103), merged `bc3f398`; adversary pass 1 found
  nothing and added 3 cases.
- Unit 2: adversary pass 1 NEEDS-CHANGE, fix round 1 (86 binaries, 1029/0/49), adversary pass 2 PASS;
  committed `aff22dc` (87 binaries, 1031/0/49, 103/103), merged `6ad241c`. The callers that can reach the
  non-UTF-8 refusal went to the orchestrator (condition 2). One pre-existing warning was routed outside the
  unit, withheld until release 0.12.0.
- Unit 5 committed `5c4de3f` (82 binaries, 1018/0/49, 103/103), merged `5b6d2b4`; the adversary's
  production-line test kept. The TLS stack on the credential leg was behind its current releases (pre-existing).
- Unit 7 (TLS dependency update, exception 6): rustls 0.23.46, rustls-webpki 0.103.16, webpki-roots 1.0.10 and a
  CHANGELOG Security line; committed `ceffb6d` (90 binaries, 1044/0/48, 103/103), merged `3bc0de4`.
- `b10x-chef-orchestrator` (on the operator's "A"): wave 5 is not pushed until the wave-6 fix is in the same
  push; wave 6 starts from the wave-5 integration head; the release trigger moves after wave 6 merges.
- Unit 4: implementor partial (Claude met with synthetic vectors, b10x refused, Codex refused); the orchestrator
  corrected a40 § 2 with Codex source citations at `rust-v0.153.4` and set the rule that a claim about a vendor
  binary cites its source at the pinned tag as file:line. Adversary pass 1 NEEDS-CHANGE; fix round 1 (94
  binaries, 1100/0/49); findings 5 and 7 became `story:codex-catalogue-on-app-server` and
  `story:several-tool-contract-families-per-session` on the orchestrator's word. Adversary pass 2 NEEDS-CHANGE;
  fix round 2 also took the independent review's s7-1, s7-2, s9-1, s10-5 (94 binaries, 1112/0/49, 103/103);
  committed `1bdc5b2`, merged `875fc71`. The story stays active: the Codex arm is open.
- Independent review (10 slices, operator's request): 0 blockers; findings triaged in the review-result;
  w5 warnings filed as `story:h11-claims-match-what-was-measured`, `story:vendor-claims-labelled`,
  `story:adversary-tests-have-positive-controls` and `story:catalogue-road-leftovers`.
- `b10x-chef-orchestrator` (ADR 0096 decision 7): `story:retire-trace-formats-beside-trace-ir` filed as a draft.
- State at close: wave 5 is not pushed. It goes to `main` in one push with wave 6 (`approval-record:adr-0096-wave-6`).
  Release 0.12.0 is held for `b10x-chef-orchestrator`'s word.
