---
format: aep.planning-md/3
id: review-result:adr0096-w8u4-review
kind: review-result
status: active
title: 'Wave 8 unit 4 review: exit verdicts reported apart'
relations:
- reviews: story:exit-status-separates-task-control-audit
revision: 1
---
# Review: wave 8 unit 4, child exit, control and audit verdicts reported apart

`aep:adversary` pass 1 over the uncommitted unit 4 change, 2026-10-22. New cases in
`crates/metaharness/tests/adversary_w8u4.rs` and `crates/metaharness-cli/tests/adversary_w8u4.rs`, five
cases, three red.

Findings and what the coordinator decided:

- Note: `README.md` row for `run --audit` did not name the three verdict lines. D40: the row names
  `child exit:`, `control verdict:` and `audit verdict:`.
- Note: the exit-code doc comment in `crates/metaharness-cli/src/lib.rs` described exit `3` without the
  auditor and without saying the child may have exited `0`, unlike section 9.4 and the CLI reference. D40:
  the row now matches them.
- Note: the `stream.closed` row in `website/docs/protocol/events.md` showed the process shapes without the
  `kind` tag key. D40: the row names `{"kind": "exited", "code": N}` and the other shapes.
- Warning: the unit's tests did not cover the branch of `child_exit` with no `stream.closed.process` marker;
  a mutant there survived. The adversary's case is kept and catches it.

Before the revision the unit gate failed one test, the authored-scenario count in the ESS pin adversary
test (3 to 4); D37: the coordinator applied that count change. Revision 1 by `aep:implementor` turned the
three red cases green. Gate `w8u4r` exit 0 on every step, 1225 passed, 0 failed, 49 ignored; ESS
conformance 111/111 on the unit. Merged into the integration branch with the suite regenerated to 116.

```findings
[
{"file":"README.md","line":null,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the run --audit row did not name the child exit, control verdict and audit verdict lines; revision 1 names them"},
{"file":"crates/metaharness-cli/src/lib.rs","line":null,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the exit-code doc comment row for 3 did not name the auditor or say the child may have exited 0; revision 1 matches section 9.4 and the CLI reference"},
{"file":"website/docs/protocol/events.md","line":null,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the stream.closed row showed the process shapes without the kind tag key; revision 1 names it"},
{"file":"crates/metaharness/tests/adversary_w8u4.rs","line":null,"category":"test-coverage","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"the no-marker branch of child_exit was untested and a mutant there survived; the adversary's case is kept and catches it"}
]
```
