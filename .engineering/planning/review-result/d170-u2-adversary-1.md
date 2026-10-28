---
format: aep.planning-md/3
id: review-result:d170-u2-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on story:version-probe-under-load
relations:
- reviews: story:version-probe-under-load
revision: 1
---
unit: story:version-probe-under-load, change e83f017a; cases at 36636ca6
verdict: CONFIRMED (no blocker: two red cases, both INFEASIBLE, plus two docs/robustness notes)
cases: executed 249→255, red 2
origin: introduced 3 / pre-existing 1 / undecided 0
wrote-outside-worktree: wave scratch only
needs-coordinator: none

Red: `adv_output_a_leftover_writes_after_the_binary_exited_is_not_its_answer` (a version a leftover
printed 1.02 s after the binary exited was read) and
`adv_a_version_token_cut_by_the_64_kib_cap_is_not_reported_as_a_version` (65532 spaces then
`2.1.304` read as `2.1.`). Green: a 7 s pinned binary admitted through `qualify`; a `yes` leftover
holds `qualify` only for the grace; 1 MiB before exit read in under 5 s; stdout closed before exit
read. What reaches the two red cases: nothing found; no vendor binary was shown to print its version
from a background process or after 64 KiB of output (the adversary built both states).

Coordinator routing: finding 1 decided as a doc correction (the pipe cannot tell the bytes apart;
the base read to end of file), case rewritten with the story id; finding 2 fixed in 9c030083;
finding 3 fixed in 9c030083; finding 4 pre-existing, filed as story:version-probe-everywhere-bounded.

```findings
[
{"file":"crates/metaharness/src/builder.rs","line":2551,"category":"docs","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"a65 point 2 (docs line 5445) says output a leftover writes after the binary exits is not its answer, but the drain loop keeps every chunk until EOF or the grace ends, so a version a leftover prints 1 s after exit is reported as the binary's (red: adv_output_a_leftover_writes_after_the_binary_exited_is_not_its_answer); the base read to EOF too, so the fix is the doc sentence"},
{"file":"crates/metaharness/src/builder.rs","line":2510,"category":"correctness","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"the new 64 KiB cap cuts a token that straddles byte 65536 and reports the prefix as the version (65532 spaces then 2.1.304 reads as Some(\"2.1.\")) (red: adv_a_version_token_cut_by_the_64_kib_cap_is_not_reported_as_a_version); fix: when the cap was hit, drop the last token unless whitespace follows it"},
{"file":"docs/design/metaharness-protocol-v0.1.md","line":2940,"category":"docs","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the a34 point 6 text still says the Codex probe reads --version within 5 seconds, with no a65 annotation, while line 2748 got one"},
{"file":"crates/metaharness/src/doctor.rs","line":99,"category":"robustness","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"doctor (doctor.rs:99) and the b10x launch's b10x_version (builder.rs:862) run --version through .output() with no deadline, so a hanging binary or a leftover holding stdout blocks them without bound; these lines are untouched by e83f017a"}
]
```
