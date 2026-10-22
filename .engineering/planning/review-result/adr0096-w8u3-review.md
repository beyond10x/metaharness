---
format: aep.planning-md/3
id: review-result:adr0096-w8u3-review
kind: review-result
status: active
title: 'Wave 8 unit 3 review: Codex command outcomes reach tool.result'
relations:
- reviews: story:codex-command-outcomes-reach-tool-result
revision: 1
---
# Review: wave 8 unit 3, Codex command outcomes reach tool.result

`aep:adversary` pass 1 over the uncommitted unit 3 change (base `8444f4c`), 2026-10-22. Ten new cases in
`crates/metaharness-codex/tests/adversary_w8u3.rs`, none red; four mutants of the join that the unit's own
tests missed (drop the `turn_id` absence check, drop the family check on the hook path, reuse check on input
only, hooks looked up before rollout calls), all caught by those cases. Every a47 citation checked holds at
Codex `rust-v0.153.4` (commit `3d2ee51c`).

Findings and what the coordinator decided:

- Warning, present before the unit and widened by it: Codex writes `exit_code: -1` when a command failed to
  start, was declined or lost its exit status, and the adapter published it as a numeric exit. D14: the
  result carries `exit_code` null and `is_error` true on the direct and the mediated path; a47 § 5 amended.
- Note: the acceptance names a recorded rollout, while only the live seam joins. D15: the acceptance is met
  on the live seam; a rollout read on its own stays null plus `TOOL_OUTCOME_UNVERIFIED` (invariant 3); a47
  § 7 says so.
- Note: the four surviving mutants. D16: the adversary's cases stay in the change.

Revision 1 by `aep:implementor`: two new tests red before the fix and green after; gate `w8u3r` exit 0 on
every step, 1226 passed, 0 failed, 49 ignored; ESS conformance 108/108.

```findings
[
{"file":"crates/metaharness-codex/src/observations.rs","line":223,"category":"correctness","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"the vendor's -1 placeholder (failed to start, declined, or exit status unknown) was published as a numeric exit_code, and a47 extended this to code-mode commands; revision 1 publishes exit_code null with is_error true"},
{"file":"docs/design/metaharness-protocol-v0.1.md","line":null,"category":"acceptance","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the acceptance promised exits from a recorded rollout, but only the live seam can join; a47 section 7 now states the acceptance is met on the live seam"},
{"file":"crates/metaharness-codex/tests/observations.rs","line":null,"category":"test-coverage","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the unit's tests missed four mutants of the join; the adversary's cases in adversary_w8u3.rs catch them and are kept"}
]
```
