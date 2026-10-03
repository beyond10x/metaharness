---
format: aep.planning-md/3
id: review-result:observation-local-adversary
kind: review-result
status: active
title: 'Codex observations: local adversarial finding and security boundary review'
relations:
- reviews: story:codex-tool-outcomes
revision: 1
---
needs-revision at the adversarial checkpoint; correction verified before integration

Coordinator review of amendment a22, the source-backed rollout mapping and the executable regressions. This is not independent review: worker quota was exhausted. No claim of fresh native qualification is made. Supported completions remain correlated by call identifier, tool family and turn evidence. Requested models, prose exits and authorization are not substituted for observations. Old serialized records remain readable through optional fields. Raw vendor records remain sensitive and no native transcript was added to source. Frame bytes and authorization decisions are unchanged.

The planted reused-call regression failed because an internal invalidation did not invalidate the last public success. The fix emits an unknown tool result and warning when the same identifier is reused with another request or turn. Ten regression tests then passed. The final offline gate passed 761 tests with 13 ignored; 36 ESS scenarios passed. The production final-answer defect failed ReadFinalAnswer/outcome/authoritative and restoration passed. The expanded guard-negation audit killed all 18 mutants with no survivors or inconclusive results.

Remaining native acceptance stays with the open stories and dependency-blocker:adapter-probe-budget. In particular, default legacy Codex history has no structured command outcome; it is unverified until an additional observation path is qualified. No current vendor pin advances and no issue is closed on synthetic evidence alone.

Private evidence: ~/.cache/metaharness-issue-repair/observations1820/{adversary-red.log,adversary-green.log,task-check-2.log,planted-answer.log,mutation-report.json}.

```findings
[
{"file":"crates/metaharness-codex/src/observations.rs","line":77,"category":"design","severity":"blocker","verdict":"needs-revision","origin":"introduced","message":"Reusing a call identifier after a successful completion invalidated only internal state; a consumer still saw the earlier public success. Emit an explicit unknown result and warning when invalidating the correlated public outcome."}
]
```
