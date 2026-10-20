---
format: aep.planning-md/3
id: review-result:adr0096-w2-strict-cwd-adversary-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:strict-hermetic-with-embedder-scratch-cwd: NEEDS-CHANGE on the test file, 1 pre-existing warning'
owner: metaharness
relations:
- reviews: story:strict-hermetic-with-embedder-scratch-cwd
revision: 1
---
unit: story:strict-hermetic-with-embedder-scratch-cwd, pass 2, working tree wt-5168648c7324 (uncommitted over 972a580; 25 tracked files, 2872+/43-)
verdict: NEEDS-CHANGE
cases: executed 650→651, red 1
origin: introduced 1 / pre-existing 1 / undecided 0
wrote-outside-worktree: 2 new paths (7 total), all under metaharness-wave-adr0096/w2-strict-cwd/adversary-scratch/ in the operator's cache directory
needs-coordinator: `story:b10x-record-carries-the-loops-cwd` is named by CHANGELOG.md:55, the design a27 "The b10x kind" and website/docs/hermetic.md but `aep plan artifact show` finds no such story in this tree's store; and whether my red case 3 moves into that story as its acceptance pin or stays here (not mine to decide)

## 1. Diff proof

Tracked diff against 972a580 is the implementor's (25 files); my only path is untracked:

```
?? crates/metaharness/tests/adversary_scratch_cwd.rs
```

Fixture bound: `git diff 972a580 --stat -- crates evals | grep fixtures` lists only the two `contract-result-*.json` goldens. No mutant copy was made; nothing to delete.

## 2. Cases (file crates/metaharness/tests/adversary_scratch_cwd.rs in the unit tree)

| case | asserts | now |
|---|---|---|
| `b10x_a6_named_directory_is_a_gap_on_h7_in_metaharness_own_words` (:123, pass 1) | a6 on b10x: H7 attested unavailable, floor gap, no "which metaharness created" | green (correction holds) |
| `b10x_declared_copy_h7_reads_ok_on_the_echoed_plan_until_the_loop_reports_its_cwd` (:156, rewritten by the implementor) | today's state: record cwd = echoed plan, H7 ok | green |
| `b10x_declared_copy_h7_is_a_gap_when_the_loops_record_names_another_directory` (:196, new) | a27 §4's comparison on b10x: loop line `"cwd":"/somewhere/else"`, plan differs, H7 must be gap | red |

Red output, run alone before the suite (exit 101):

```
thread 'b10x_declared_copy_h7_is_a_gap_when_the_loops_record_names_another_directory' panicked at crates/metaharness/tests/adversary_scratch_cwd.rs:203:5:
assertion `left == right` failed: the loop reported /somewhere/else, the plan was <scratch>/copy, the record carries Some("<scratch>/copy"); the floor compared the plan with its own echo: the session ran in ..., which was declared, not created: ...
  left: Ok
 right: Gap
```

Probe run in the same invocation and then removed (green, so not a case): a `SeamFactory` wrapper that drops `observe_launch` (the no-echo shape) gives record cwd `None` and H7 `Unk`, so the rewritten case 2's `assert_eq!(event_cwd, Some(planned))` does go red in that shape.

## 3. Suite

`cargo-watched test --no-fail-fast -p metaharness-protocol -p metaharness -p metaharness-claude -p metaharness-codex -p metaharness-aep` → exit 101; 32 binaries, 650 passed, 1 failed (case 3), 21 ignored, executed 651; 0 compiler warnings. Before = 650, the implementor's gate count, confirmed by 651 − 1.

## 4. Findings

| # | file:line | what is wrong | measured | reaches it | verdict | origin |
|---|---|---|---|---|---|---|
| A | crates/metaharness/tests/adversary_scratch_cwd.rs:156-189 (rewritten case 2) | it goes red when the story lands only if the landing writes `None` for a loop line without cwd; the same function already reads `model: string("model").or_else(\|\| self.model.clone())` (seam.rs:155), and the matching `string("cwd").or_else(\|\| self.cwd.clone())` keeps the echo for a line that says nothing, under which this case stays green and never announces the landing | probe: no-echo shape → `None`/`Unk` (red by construction there); fallback shape → unchanged event, case green; case 3 is red under both shapes until the seam reads the line's cwd | the test itself, on the day the story lands | NEEDS-CHANGE | introduced |
| B | crates/metaharness-b10x/src/seam.rs:163 (read by audit.rs:626-634) | the seam never reads the loop line's cwd; with the line naming `/somewhere/else` the record carries the planned path and a declared H7 reads `ok`, so a27 §4's record-versus-plan comparison is not performed on b10x (same signature as pass-1 finding 2, now with the story filed and the case in the tree) | case 3 red: record `Some(planned)`, H7 `Ok` | driven b10x arm once `--scratch-cwd` is passed (drive.rs:2413-2420 already passes `--hermetic --cwd`); nothing passes the flag today | CONFIRMED | pre-existing |

## 5. Attacked, could not break

- New b10x H7 `unavailable` wording vs a6/a6.1: it matches Claude's a6 reason word for word up to the appended clause; a6.1's explicit write statement exists only on codex (`sandbox_mode`), so neither Claude nor b10x states "may write" — pre-existing scope of a6.1, not raised. "nothing in this launch widens it" holds: `--process-write-subtree` requires substrate and names subtrees inside the workspace (check_spec, builder.rs:1628-1640); `--substrate-embedded` serves the parent as root with the workspace adopted as a component (builder.rs:1836).
- audit.rs:871-910 guard order: declared H11 alone → gap, declared + imposed H11 → gap (pins declared-before-imposed; a swap would read `Ok`), honest → unk; a mutant making `admits_declaration` true everywhere drops the first assertion to `Unk` and is caught, and `only_h7_admits_a_declaration` in the protocol crate pins it a second time.
- Vectors: the correction changed only the b10x a6 branch; the b10x golden emits over a scratch cwd (contract_golden.rs names no cwd), both contract-result goldens unchanged since pass 1, `ess_suite_matches_the_current_specification` green in the lane (conformance regenerated byte-identical in pass 1, spec unchanged since).
- The exact words of the b10x H7 reasons (unavailable and declared) are pinned by no vector, only by case 1's presence/negative assertions — same standing as Claude's a6 words; not raised.

## 6. Paths written outside the worktree

metaharness-wave-adr0096/w2-strict-cwd/adversary-scratch/pass2-cases-alone.log, pass2-suite-nff.log (new); cases-alone.log, cases-alone-2.log, suite.log, suite-nff.log, conformance.regen.json (pass 1)

## Commands and exit codes

| command | exit |
|---|---|
| `aep plan artifact show story:b10x-record-carries-the-loops-cwd` (read-only) | 0, "holds no story" |
| `cargo-watched test -p metaharness --test adversary_scratch_cwd` (3 cases + probe) | 101: 3 passed, 1 failed |
| `cargo-watched test --no-fail-fast -p metaharness-protocol -p metaharness -p metaharness-claude -p metaharness-codex -p metaharness-aep` | 101: 650 passed, 1 failed, 651 executed |
| `git diff 972a580 --stat -- crates evals \| grep fixtures` | two goldens only |

```findings
[
  {"file": "crates/metaharness/tests/adversary_scratch_cwd.rs", "line": 156, "category": "judgement", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The rewritten case pins the echo through assert_eq!(event_cwd, Some(planned)), which stays satisfied if the story lands with the fallback shape seam.rs:155 already uses for model, so it can pass silently; case 3 (a loop line whose cwd differs from the plan, asserting gap) is red under both shapes until the seam reads the line."},
  {"file": "crates/metaharness-b10x/src/seam.rs", "line": 163, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "The b10x seam writes the planned cwd into session.started and ignores the loop line's own cwd, so with the line naming /somewhere/else a declared H7 still reads ok and a27 §4's record-versus-plan comparison is not performed on this kind."}
]
```
