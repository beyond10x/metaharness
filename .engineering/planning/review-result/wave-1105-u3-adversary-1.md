---
format: aep.planning-md/3
id: review-result:wave-1105-u3-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on story:claude-attestation-names-actor
relations:
- reviews: story:claude-attestation-names-actor
- reviews: story:vendor-claims-labelled
revision: 1
---
unit: story:claude-attestation-names-actor, story:vendor-claims-labelled, impl/vendor-claims-and-actor at fc44860 (merged 03a760e); cases 68f2b79
verdict: CONFIRMED (notes only; no red case)
cases: executed 160→164, red 0
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: wave scratch only

The H3 change held on every line attacked: no value reaches the attestation, the listed names equal the
added keys in all 96 worlds, the old sentence stands on a plain plan, and the floor reads only H4.

Both notes are closed: the four new cases pin the H3 row exactly (merged 91f62dc), and the a37 Codex
citation now says the runs were on the tree later committed as 7164df8.

```findings
[
{"file":"crates/metaharness-claude/src/launch.rs","line":2363,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the unit H3 tests read the row only through contains, so a value printed beside each name, a separator printed with an empty list, or a reordered list would pass them; adversary_w1105_u3_pass1.rs now pins all three"},
{"file":"docs/design/metaharness-protocol-v0.1.md","line":3279,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"ran at the pin commit 7164df8 is inferred, since the research page names no commit and its lanes started about two hours before 7164df8 was committed; the tree committed as 7164df8 is what the evidence supports"}
]
```
