---
format: aep.planning-md/3
id: review-result:wave-0290-u4-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on story:aep-eval-scope-through-substrate
summary: Runtime preflight green inside the adapter scope; the source scan was too narrow, fixed in correction 1
relations:
- reviews: story:aep-eval-scope-through-substrate
revision: 1
---
unit: story:aep-eval-scope-through-substrate, impl/aep-eval-substrate-scope at 75594a5 (base ce3b05a, merged 84721c4 into integration/wave-0-29-0); cases 0a55939
verdict: CONFIRMED (strongest)
cases: executed 43→47, red 2
origin: introduced 2 / pre-existing 2 / undecided 0
wrote-outside-worktree: wave scratch only

Runtime: the adversary ran the free preflight (`aep-eval preflight`, no model contacted) inside the scope
that the `systemd` adapter opened. The harness ran in the scope's `embedder` leaf and published seven tools,
and no scope was left behind. The macOS release builds only `--bin metaharness`, which does not depend on
the eval.

Findings:
- The acceptance-1 source scan matched the program name only between double quotes. It missed an absolute
  path and a shell string (CONFIRMED, introduced; two red cases against a copy of the matcher).
- No test drove `ensure_cgroup_scope`'s adapter branch (note, introduced). Covered by the adversary's
  entry-point case.
- The ignored probe's reason still said to run under `systemd-run` (note, pre-existing).
- `below_cgroup_root` is a substring match, where Substrate's probe needs a canonical prefix (INFEASIBLE,
  pre-existing). Filed as `story:eval-cgroup-root-check-by-prefix`.

## Outcome

Correction 1 (`1fbc97e`, merged with the cases at `e6755a5`):
- The scan reads every code line for the program name in any form, with negative controls for each form.
- The two cases against the old matcher, and their helper, are removed.
- The ignore reason names a delegated user scope.

The coordinator corrected the adversary file's module comment at `2b295cc`.

```findings
[
{"file":"crates/metaharness-aep-eval/src/lib.rs","line":3130,"category":"mutant","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"The acceptance-1 source scan only matched the program name between double quotes, so an invocation by absolute path or through a shell string passed it."},
{"file":"crates/metaharness-aep-eval/src/lib.rs","line":1323,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"No test ran ensure_cgroup_scope's adapter branch; the adversary's entry-point case covers it."},
{"file":"crates/metaharness-aep-eval/src/lib.rs","line":3194,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"The ignored confined-probe test told the operator to run it under systemd-run."},
{"file":"crates/metaharness-aep-eval/src/lib.rs","line":1358,"category":"boundary","severity":"note","verdict":"INFEASIBLE","origin":"pre-existing","message":"below_cgroup_root uses a substring match while substrate's probe needs a canonical path prefix; filed as story:eval-cgroup-root-check-by-prefix."}
]
```
