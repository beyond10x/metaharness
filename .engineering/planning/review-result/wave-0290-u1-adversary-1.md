---
format: aep.planning-md/3
id: review-result:wave-0290-u1-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on the U1 hook record stories
summary: Census and ordering CONFIRMED, fixed in correction 1; planted outcome file accepted as a same-uid limit
relations:
- reviews: story:claude-hook-refusals-reach-the-record
- reviews: story:claude-seam-abstain-is-recorded-as-vendor-decided
revision: 1
---
unit: story:claude-hook-refusals-reach-the-record + story:claude-seam-abstain-is-recorded-as-vendor-decided, impl/hook-refusals-and-abstain at 1d15a19 (base 94e2d94, merged 47f8196 into integration/wave-0-29-0); cases 1a494f8
verdict: CONFIRMED (strongest)
cases: executed 7→10, red 3
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: wave scratch only

The red cases are in `crates/metaharness/tests/adversary_claude_hook_record_pass1.rs`:

- `the_census_in_session_ended_counts_a_decision_no_hook_program_reported_on` (CONFIRMED): a decision held
  for a hook report that never comes was released only at wind-up, so `session.ended`'s census omitted it.
- `a_decision_no_hook_program_reported_on_still_precedes_the_calls_result` (CONFIRMED): on that path,
  `tool.decided` landed after the call's `tool.result` and after `session.ended`.
- `an_outcome_the_program_did_not_write_does_not_record_a_deny_for_a_call_it_forwarded` (INFEASIBLE): a
  well-named file planted in `outcomes/` is trusted. No real writer was shown.

There is also a note: the ask-mode row of a41 § 11 did not say what ask mode does.

## Outcome

Correction 1 (`e492f8c`, merged with the cases at `52bc3a8`):
- An unreported decision is recorded with `HOOK_OUTCOME_UNSEEN` before the call's `tool.result`, and before
  `session.ended`, whose census counts it.
- The planted-file case was decided by the orchestrator to be an accepted limit: `outcomes/` has the same
  same-uid trust as `responses/`. a41 § 11 names the limit, and the case now pins today's behaviour.
- The ask-mode row is corrected.

All three cases are green.

```findings
[
{"file":"crates/metaharness/tests/adversary_claude_hook_record_pass1.rs","line":244,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"A decision held for a hook report that never comes is counted only at wind-up, so session.ended's census omits a tool.decided the record holds."},
{"file":"crates/metaharness/tests/adversary_claude_hook_record_pass1.rs","line":271,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"On the HOOK_OUTCOME_UNSEEN path, tool.decided lands after the call's tool.result and after session.ended, against a41 section 11's ordering claim."},
{"file":"crates/metaharness/tests/adversary_claude_hook_record_pass1.rs","line":291,"category":"judgement","severity":"warning","verdict":"INFEASIBLE","origin":"introduced","message":"collect_outcomes trusts any well-named file in outcomes/, so a planted file records a deny for a call the program forwarded; accepted as the same same-uid trust as responses/."},
{"file":"docs/design/metaharness-protocol-v0.1.md","line":3851,"category":"judgement","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"In ask mode a call the program refused before it was presented is presented as decision_required true, not denied at presentation as the table row said."}
]
```
