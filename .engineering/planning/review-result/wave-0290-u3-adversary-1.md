---
format: aep.planning-md/3
id: review-result:wave-0290-u3-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on story:h11-claims-match-what-was-measured
summary: Three overclaiming H11 texts and three wording findings, fixed in correction 1; one out-of-scope finding filed as a draft
relations:
- reviews: story:h11-claims-match-what-was-measured
revision: 1
---
unit: story:h11-claims-match-what-was-measured, impl/h11-claims at 5fbb9af (base 47f8196, merged ce3b05a into integration/wave-0-29-0); cases cecc44a
verdict: CONFIRMED (strongest)
cases: executed 18→21, red 3
origin: introduced 1 / pre-existing 3 / undecided 0 (red cases and warnings); 2 introduced notes; 1 out of scope
wrote-outside-worktree: wave scratch only

The red cases are in `crates/metaharness-claude/tests/adversary_h11_claims_pass1.rs`. Three texts that predate
the unit claimed more ancestors than the canary probed:
- the H11 reason of a `--cwd` run;
- the Claude harness page (line 84);
- the conclusion of the 2.1.304 research page.

Findings the unit introduced:
- the website, CHANGELOG and a36 § 2 said a copied row fails the gate, but a typed record line passes it;
- the `doctor` path said "not measured in this run" where there is no run;
- a grammar slip in the unprobed list.

Out of scope, pre-existing: H11 stays `ok` when the walk finds memory in a surface the canary never probed.
This is filed as `story:h11-not-ok-on-unprobed-memory`.

## Outcome

Correction 1 (`c983d2a`, merged with the cases at `502240d`):
- the three texts say "the cwd and its parent";
- every statement of the copy check says that a copied row fails unless a matching record line is added,
  and that the gate cannot tell a typed line from a measured one;
- the shared `ok` sentence names no run, and a new `qualify` case pins that;
- the deep-rules wording is fixed.

All three adversary cases are green.

```findings
[
{"file":"crates/metaharness-claude/src/launch.rs","line":2000,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"The H11 reason of an operator-named --cwd run said memory files in and above the cwd were kept out at every pin, a surface now listed as unprobed above the parent."},
{"file":"website/docs/harnesses/claude.md","line":84,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"The page said no memory file in the working directory or above it reached the model, while the paragraph says ancestors above the parent were not probed."},
{"file":"docs/research/2026-10-21-claude-code-2.1.304-adapter-surface.md","line":103,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"The cited page concluded for every ancestor while recording that only the parent was probed."},
{"file":"website/docs/harnesses/claude.md","line":96,"category":"acceptance","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"A copied row passes once a matching record line is typed into a research page, yet the website, CHANGELOG and a36 section 2 said a copied row fails the gate."},
{"file":"crates/metaharness/src/audit.rs","line":1286,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"doctor's qualification rendered H11 ok as not measured in this run where there is no run."},
{"file":"crates/metaharness-claude/src/lib.rs","line":500,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"In .claude/rules files below its first level, its bound to the cwd's parent rather than to the rules directory."},
{"file":"crates/metaharness/src/builder.rs","line":3278,"category":"out-of-scope","severity":"note","verdict":"INFEASIBLE","origin":"pre-existing","message":"H11 stays ok when the walk finds a memory file in a surface the canary never probed; filed as story:h11-not-ok-on-unprobed-memory."}
]
```
