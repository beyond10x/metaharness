---
format: aep.planning-md/3
id: review-result:wave-1104-u1-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on story:b10x-failed-run-names-its-cause
relations:
- reviews: story:b10x-failed-run-names-its-cause
revision: 1
---
unit: story:b10x-failed-run-names-its-cause, impl/b10x-failed-run-names-its-cause at 9ca39da (base 2dc4d26, merged b027c33 into integration/wave-2026-11-04); cases ec661ef
verdict: NEEDS-CHANGE (documentation only; no red case)
cases: executed 67→69, red 0
origin: introduced 1 / pre-existing 0 / undecided 1
wrote-outside-worktree: wave scratch only

Cases added in `crates/metaharness/tests/adversary_u1_pass1.rs`, both green:

- `a_recorded_failed_run_ends_in_one_session_ended_that_names_its_cause` drives a real record from an
  unpinned `b10x-harness 0.26.0` (`run --json` against a closed loopback port, no model or
  credential) through `B10xSeams` and the run loop: one `session.ended`, `subtype`
  `failed:provider-transport`, `terminal_reason` `failed: model wire refused: ...`, stream closes
  `Error`.
- `a_stop_that_is_not_failed_ignores_an_after_compaction_flag` kills the mutant that drops the
  `failed` guard on the compaction flag.

Finding 1 (introduced): the failed stop shipped in Harness 0.23.0, not 0.21.0, so at the adapter's
pin (0.22.0) the mapping never runs. The claim is corrected to 0.23.0 and labelled unverified at
the pin; moving the pin is not this unit's scope. Goes back to the unit as correction 2.

Finding 2 (undecided, infeasible input): closed by the adversary's own case; no change needed.

```findings
[
{"file":"crates/metaharness-b10x/src/seam.rs","line":481,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the code comment, amendment a73 (docs/design/metaharness-protocol-v0.1.md:6192,6216) and CHANGELOG.md:8 say the failed stop arrives from Harness 0.21.0, but it shipped in Harness 0.23.0, and the adapter's pin (0.22.0, enforced by the AEP eval) cannot emit it, so the claim is neither pinned nor labelled unverified (invariant 4)"},
{"file":"crates/metaharness-b10x/src/seam.rs","line":494,"category":"mutant","severity":"note","verdict":"INFEASIBLE","origin":"undecided","message":"dropping the failed guard on after_compaction leaves every unit case green; adversary_u1_pass1.rs now catches it, but no real loop writes after_compaction on a stop other than failed"}
]
```
