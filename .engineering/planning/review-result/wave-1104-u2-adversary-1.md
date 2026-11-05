---
format: aep.planning-md/3
id: review-result:wave-1104-u2-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on story:version-probe-everywhere-bounded
relations:
- reviews: story:version-probe-everywhere-bounded
revision: 1
---
unit: story:version-probe-everywhere-bounded, impl/version-probe-everywhere-bounded at 6ec52f1 (base 2dc4d26, merged 6f0017a into integration/wave-2026-11-04); cases 15bc6ef
verdict: NEEDS-CHANGE
cases: executed 2→5, red 2
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: wave scratch only

The bound itself holds: through the binary at the production deadline, `doctor` and a strict b10x
launch over a `--version` that never exits stop at about 60 s with exit 2 (case
`doctor_over_a_version_that_never_exits_refuses_by_name_at_the_production_deadline`, green).

Red cases in `crates/metaharness-cli/tests/adversary_u2_pass1.rs`:

- `a_strict_b10x_launch_over_a_version_that_never_exits_refuses_naming_the_deadline`: the refusal
  says "reported no readable version" after the kill, where the Claude and Codex strict refusals
  name the deadline.
- `doctor_names_the_exit_status_and_stderr_of_a_version_that_fails`: doctor lost the exit status and
  stderr its base refusal named, and gives one text for three causes.

Both go back to the unit as correction 1.

Out of scope, filed as a draft: three `--version` reads in the AEP crates
(`metaharness-aep/src/drive.rs`, `metaharness-aep/src/eval.rs`, `metaharness-aep-eval/src/lib.rs`)
have no deadline; they were unbounded at base and outside this story's scope.

```findings
[
{"file":"crates/metaharness/src/builder.rs","line":904,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a strict b10x launch whose --version is killed at the 60 s deadline is refused as \"reported no readable version\" instead of naming the deadline, unlike the Claude and Codex strict refusals (a33, a34)"},
{"file":"crates/metaharness/src/doctor.rs","line":122,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"doctor's refusal for a failing --version no longer carries the exit status or stderr the base refusal named, and gives one text for could-not-start, failed and no-token"}
]
```
