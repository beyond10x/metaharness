---
format: aep.planning-md/3
id: review-result:adr0096-w6u1-review
kind: review-result
status: active
title: 'Review of wave 6 unit 1: security review and independent slices s6 and s10-6'
owner: metaharness
refs:
- provider: atlas
  reference: adr-0096
relations:
- reviews: story:claude-seam-fails-closed
revision: 1
---
unit: wave 6 unit 1 of the ADR 0096 work (`story:claude-seam-fails-closed`), reviewed twice: the security review of the implementor's tree on base `3bc0de4`, and slice s6 of the independent review at `cacb923`, 2026-10-21
verdict: no blocker; 12 findings recorded here, 5 warnings and 7 notes
cases: security review executed 1051 → 1054 with 3 added cases, red 0 against the tree; R1 and R2 go red under one mutant each. Slice s6 executed none (read-only)
origin: introduced 2 / pre-existing 1 / undecided 9
wrote-outside-worktree: the reviewers' scratch files under metaharness-wave-adr0096/w6/u1-rev1 and the slice file under metaharness-wave-adr0096/review in the operator's cache directory
needs-coordinator: triage below, taken by the coordinator

## Reviews

| review | covers | findings here |
|---|---|---|
| security review | the unit's tree: `crates/metaharness-claude/src/hook.rs`, `launch.rs`, amendment a41 of `docs/design/metaharness-protocol-v0.1.md`, the CHANGELOG line | the first three |
| independent review, slice s6 | the seam at `cacb923`: `crates/metaharness/src/run.rs`, `spawn.rs`, the Claude adapter's `hook.rs`, `launch.rs`, `builder.rs`, `metaharness-protocol/src/spec.rs` | s6-1 to s6-7 and one outside the slice |
| independent review, slice s10 | the tests the wave added | s10-6 |

The security review measured eight mutants of the hook program. Two of them (M-SUF, M-NL) left every unit
test green; its added cases R1 and R2 turn red under them. Dash, `/bin/sh` on the CI runner, was not
installed locally; the hook tests ran under dash in a Debian container at `cacb923` and at `5ddebbd`, the
combined head of waves 5 and 6.

## Disposition (coordinator)

- Fixed: the two security-review warnings. `crates/metaharness-claude/tests/review_w6u1.rs` (R1, R2, R3) was
  kept and landed with the unit.
- No action: the a37 § 5 note. It went out in the same push as a41, which marks it superseded.
- Became stories: s6-1 → `story:claude-seam-abstain-is-recorded-as-vendor-decided`; s6-2 →
  `story:claude-seam-runs-only-a-pinned-binary`; s6-3 → `story:claude-hook-refusals-reach-the-record`.
- Open, no story: s6-4, s6-5, s6-6 (covered by the dash runs above), s6-7, the outside-slice note, s10-6.

```findings
[
{"file":"crates/metaharness-claude/src/hook.rs","line":276,"category":"mutant","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"security review: the a41 § 5 single-line rule is measured by no unit test; removing it leaves the unit's tests green and only review case R2 turns red; fixed by keeping tests/review_w6u1.rs"},
{"file":"crates/metaharness-claude/src/hook.rs","line":279,"category":"mutant","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"security review: the a41 § 5 closing-with-}} rule is measured by no unit test; removing it leaves the unit's tests green and only review case R1 turns red; fixed by keeping tests/review_w6u1.rs"},
{"file":"docs/design/metaharness-protocol-v0.1.md","line":3279,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"security review: a37 § 5 states the earlier hook default outside a41; it went out in the same push as a41, which marks it superseded; no action"},
{"file":"crates/metaharness/src/run.rs","line":962,"category":"security","severity":"warning","verdict":null,"origin":"undecided","message":"s6-1 (at cacb923; also hook.rs:267-283): an abstain is an empty answer, the hook exits 0 and the vendor's own permission pipeline decides the call; a run with decisions frame and no frame abstains on every call; the a41 fail-closed claim does not cover abstain; became story:claude-seam-abstain-is-recorded-as-vendor-decided"},
{"file":"crates/metaharness-claude/src/launch.rs","line":1638,"category":"security","severity":"warning","verdict":null,"origin":"undecided","message":"s6-2 (at cacb923; also builder.rs:2261-2263, :888-894, metaharness-protocol spec.rs:600): refusing on hook start failure, timeout or an unreadable answer rests on the vendor honouring onFailure; strict_version defaults to false and nothing else checks the pin, so an off-pin binary may ignore the key; became story:claude-seam-runs-only-a-pinned-binary"},
{"file":"crates/metaharness/src/spawn.rs","line":142,"category":"auditability","severity":"warning","verdict":null,"origin":"undecided","message":"s6-3 (at cacb923; also spawn.rs:447-462, run.rs:1114-1133, hook.rs:117-140): the hook program's own refusals (no channel, backstop, unreadable answer) never reach metaharness, so tool.decided records allow for a call the program refused; became story:claude-hook-refusals-reach-the-record"},
{"file":"crates/metaharness-claude/src/hook.rs","line":58,"category":"correctness","severity":"note","verdict":null,"origin":"undecided","message":"s6-4 (at cacb923): the backstop counts 570 ticks of sleep 0.1 plus fork overhead, and the fallback sleep 1 makes it about 570 s; it is not bounded by wall time; the call is still refused under onFailure block; open"},
{"file":"crates/metaharness-claude/src/hook.rs","line":331,"category":"auditability","severity":"note","verdict":null,"origin":"undecided","message":"s6-5 (at cacb923): the deadline test asserts a literal 55 instead of metaharness_deadline_ms(...); open"},
{"file":"crates/metaharness-claude/src/hook.rs","line":229,"category":"correctness","severity":"note","verdict":null,"origin":"undecided","message":"s6-6 (at cacb923): the hook tests run /bin/sh, which is bash on the review machine; covered by the dash runs in a Debian container at cacb923 and 5ddebbd"},
{"file":"crates/metaharness-claude/src/hook.rs","line":267,"category":"correctness","severity":"note","verdict":null,"origin":"undecided","message":"s6-7 (at cacb923): [ -s ] on a FIFO or zero-size response exits 0 as abstain; the run loop publishes by rename and never reaches it; open"},
{"file":"crates/metaharness/src/run.rs","line":747,"category":"auditability","severity":"note","verdict":null,"origin":"undecided","message":"outside slice s6 (at cacb923), inferred: the hook's tool_input is never compared with the transcript input the decision used; open"},
{"file":"crates/metaharness-claude/tests/review_w6u1.rs","line":183,"category":"auditability","severity":"note","verdict":null,"origin":"undecided","message":"s10-6 (at cacb923): the byte-for-byte case R3 drives a test-side copy of the writer path, not the production writer; open"}
]
```
