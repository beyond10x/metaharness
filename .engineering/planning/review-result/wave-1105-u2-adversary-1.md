---
format: aep.planning-md/3
id: review-result:wave-1105-u2-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on story:b10x-record-keeps-the-loops-subjects
relations:
- reviews: story:b10x-record-keeps-the-loops-subjects
revision: 1
---
unit: story:b10x-record-keeps-the-loops-subjects, impl/b10x-record-subjects at 341b3db (base c076386, merged c86edbe into integration/wave-2026-11-05); cases 24b908a
verdict: INFEASIBLE (strongest; two CONFIRMED doc rows)
cases: executed 82→85, red 3
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: wave scratch only

The flat path held, the five regenerated streams are byte-identical to base without `subjects`, and no
fixture publishes `tool_invoke`.

Red cases in `crates/metaharness/tests/adversary_b10x_record_subjects_pass1.rs` concern the rule the unit
added beyond its acceptance. At harness `3eca074` the verbs surface routes a bare entry name to its entry
(`Verbs::invoked_entry`), and the loop performs it with `unpublished-tool-routed`. The unit reads such a call
as no operation and no subject:

- `a_bare_entry_name_the_verbs_loop_routed_is_recorded_as_the_operation_it_performed`
- `a_frame_admits_a_routed_write_on_a_verbs_b10x_run`
- `a_subject_scope_holds_for_a_routed_write_on_a_verbs_b10x_run`

Nothing reaches this rule today: no launch passes `--surface verbs`.

Decision: fix it in the unit. On a verbs run, a bare name resolves as the base did (the rendering table and
`subjects_of_entry`). The contrary unit test is rewritten. Amendment a75 § 2 and § 3 are corrected to what
the pinned loop does. This goes back to the unit as correction 1.

```findings
[
{"file":"crates/metaharness/src/run.rs","line":1105,"category":"contract-drift","severity":"warning","verdict":"INFEASIBLE","origin":"introduced","message":"on a b10x run that published the verbs, a bare entry name the pinned loop routes and performs is read as no operation and no subject, so observe drops the operation, frame denies an admitted write as UNCOVERED_TOOL without judging its path, and ask hands the embedder neither field; no launch passes --surface verbs today"},
{"file":"docs/design/metaharness-protocol-v0.1.md","line":6280,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"amendment a75 section 3 says a flat entry name on a verbs run is a tool the loop did not publish, but at harness 3eca074 Verbs::invoked_entry routes it to the entry, which the loop performs with unpublished-tool-routed"},
{"file":"docs/design/metaharness-protocol-v0.1.md","line":6278,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"amendment a75 section 2 promises UNCOVERED_TOOL under a frame for a flat-run tool_invoke, but the loop refuses that name before its approver, so the call reaches the run with Seam::None and no frame verdict or warning is produced"}
]
```
