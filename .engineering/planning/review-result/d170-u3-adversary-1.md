---
format: aep.planning-md/3
id: review-result:d170-u3-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on story:b10x-harness-pin-0-22
relations:
- reviews: story:b10x-harness-pin-0-22
revision: 1
---
unit: story:b10x-harness-pin-0-22, commit e90e2470 on unit/d170-u3-harness-0-22 (base ca82f28b)
verdict: nothing found in this unit's change (one older CHANGELOG miscount noted)
cases: executed n/a→n/a, red 0 (no case added, so no suite was run)
origin: introduced 0 / pre-existing 1 / undecided 0
wrote-outside-worktree: wave scratch only
needs-coordinator: none

Attacked and not broken: no current pin names `ef6df52e` or 0.21.0 (history mentions only); the six
fixtures differ from 0.21.0's only in `captured_at`, `environment.workspace` and `harness_version`;
the committed records are byte-identical to the capture output, the archive passes `SHA256SUMS`;
`golden/loop.jsonl` sha256 `7233a137...` equals `trace.yaml:627` and `conformance.json`; 0.21.0 and
0.22.0 `--help` differ only by `--substrate-adopt`; `metaharness doctor b10x` on 0.22.0 exits 0;
`0.22.0^{commit}` is `3eca074a`, an ancestor of harness main; the lock test still proves one
harness revision.

Coordinator routing: the one finding is fixed on the integration branch (CHANGELOG 117 -> 120).

```findings
[{"file":"CHANGELOG.md","line":20,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"The ESS 0.75.0 entry says spec/conformance.json has 117 scenarios; it has 120 at 9ff83d98, its parent and e90e2470, and the unit fixed the same miscount in spec/coverage.md but not here."}]
```
