---
format: aep.planning-md/3
id: review-result:adr0096-w2-strict-cwd-adversary-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:strict-hermetic-with-embedder-scratch-cwd: NEEDS-CHANGE, 1 blocker, 1 pre-existing warning, 1 note'
owner: metaharness
relations:
- reviews: story:strict-hermetic-with-embedder-scratch-cwd
revision: 1
---
unit: story:strict-hermetic-with-embedder-scratch-cwd, working tree wt-5168648c7324 (branch impl/strict-hermetic-with-embedder-scratch-cwd, uncommitted over 972a580)
verdict: NEEDS-CHANGE
cases: executed 647→649, red 2
origin: introduced 2 / pre-existing 1 / undecided 0
wrote-outside-worktree: 5 paths, all under metaharness-wave-adr0096/w2-strict-cwd/adversary-scratch/ in the operator's cache directory
needs-coordinator: the origin call on finding 1 (the b10x omission reproduces at base; the claim that it is corrected is this unit's); clippy was not run over my file (two `unused` warnings removed by inspection)

## 1. Diff proof

`git --no-pager diff 972a580 --stat` is unchanged from the handed tree (25 files, 2811+/43-). My only addition is untracked:

```
?? crates/metaharness/tests/adversary_scratch_cwd.rs
```

No tracked file touched. No mutant copy made, so no target/ to delete.

## 2. Cases added (file crates/metaharness/tests/adversary_scratch_cwd.rs in the unit tree)

Both drive `Metaharness::new(Kind::B10x).with_hermetic(Strict).with_cwd(copy)` through the production `metaharness_b10x::B10xSeams` and `ScriptedRunner`, then read `run.hermetic_floor()`.

| case | asserts | now |
|---|---|---|
| `b10x_a6_named_directory_is_a_gap_on_h7_in_metaharness_own_words` (:88) | a27 §4 / CHANGELOG:49-53: an a6 `--cwd` H7 is `gap`, never "which metaharness created"; H7 attested unavailable | red |
| `b10x_declared_copy_h7_is_not_ok_on_a_record_the_loop_never_wrote` (:115) | a27 §4: `declared` is `ok` only on a record cwd; the loop's `started` line carries none | red |

Red output, run alone before the suite (`cargo-watched test -p metaharness --test adversary_scratch_cwd`, exit 101):

```
thread 'b10x_a6_named_directory_is_a_gap_on_h7_in_metaharness_own_words' panicked at crates/metaharness/tests/adversary_scratch_cwd.rs:92:5:
assertion `left == right` failed: the session ran in <scratch>/copy, which metaharness created
  left: Ok
 right: Gap
thread 'b10x_declared_copy_h7_is_not_ok_on_a_record_the_loop_never_wrote' panicked at crates/metaharness/tests/adversary_scratch_cwd.rs:124:5:
assertion `left != right` failed: H7 read ok on b10x from a cwd the loop never reported: the session ran in <scratch>/copy, which was declared, not created: the embedder declared ... as its disposable scratch copy (--cwd --scratch-cwd); ...
  left: Ok
 right: Ok
```

(First run of the file also carried three Claude probes, all green; removed, listed in § 5.)

## 3. Suite (after the cases existed)

`cargo-watched test --no-fail-fast -p metaharness-protocol -p metaharness -p metaharness-claude -p metaharness-codex -p metaharness-aep` → exit 101; 32 binaries, 647 passed, 2 failed (mine), 21 ignored, executed 649. Before = 647, the implementor's reported after for this lane, confirmed by 649 − 2. All other binaries green, including `ess_generated_scenarios_drive_production_targets` and both contract goldens.

## 4. Findings

| # | file:line | what is wrong | measured | reaches it | verdict | origin |
|---|---|---|---|---|---|---|
| 1 | crates/metaharness/src/builder.rs:670-677 (a6 branch of `start_b10x`) vs docs/design/metaharness-protocol-v0.1.md a27 §4 + "The b10x kind", CHANGELOG.md:49-53 | the a6 branch on b10x pushes H11 unavailable and never H7, so the floor's new H7 arm (audit.rs:647-653) has nothing to read and still prints `ok "which metaharness created"` over an operator's `--cwd` tree; the design and changelog say that wording is gone and "the same rule applies" to b10x | case 1 red: H7 `Ok`, detail "which metaharness created"; `git show 972a580:…/builder.rs` has no `HermeticRow::H7` at all | `metaharness aep drive` b10x arm, crates/metaharness-aep/src/drive.rs:2413-2420 (`run b10x --hermetic --cwd <dir>`) | NEEDS-CHANGE | introduced (the omission reproduces at base; the claim it is corrected is this unit's, and the block is in its scope) |
| 2 | crates/metaharness-b10x/src/seam.rs:163 read by crates/metaharness/src/audit.rs:626-634 | on b10x `session.started.cwd` is metaharness's echo of its own plan (`observe_launch`), the loop's `started` line names no cwd; `record == planned` is a tautology, so `declared → ok` is decided by the flag plus metaharness's own echo (invariant 3 in letter; a27 §4 "the record is still the evidence" is vacuous here) | case 2 red: loop line without cwd, event cwd = planned, H7 `Ok` | same drive arm once it adds `--scratch-cwd` (intended caller: Loom's executor, ADR 0098); nothing passes the flag today | CONFIRMED | pre-existing (seam.rs:163 identical at 972a580; at base the echo already made H7 `ok` under `--cwd`) |
| 3 | crates/metaharness/src/audit.rs:728-740 | the "declared on a row that admits no declaration → gap" arm a27 §2 promises is pinned by no test: every `DeclaredControl` literal in the tree names H7 (grep, 4 sites) | grep only; no production path builds such a row | nothing found | INFEASIBLE | introduced |

Fix for 1, not applied: push `UnavailableControl { row: H7, why: … }` in the `else if spec.cwd.is_some()` branch of `start_b10x`, or state in a27 and cli.md that b10x keeps the old wording.

## 5. Attacked, could not break

- (a) Claude: declared copy with record cwd elsewhere → H7 gap, exit 1; record without cwd → H7 unk, exit 3 (green probes, removed).
- (b) no `"declared"` key in any file under `crates/*/fixtures` or `evals`; `git diff 972a580 --stat -- crates evals` limited to the two `contract-result-*.json` goldens; every stream reader in the lane green.
- (c) copy named through a symlink whose real parent holds AGENTS.md → H11 gap (`resolve_cwd_named` canonicalizes; green probe, removed); memory file above the copy already pinned by the unit and the ESS scenario; `/home` or `/` as the declared copy is admitted by a27 §1 ("not a confinement"), not raised.
- (d) b10x declaration attests H7 declared, H11 imposed (seen in case 2's output); stated in a27 "The b10x kind".
- (e) `scratch_cwd_plan` maps `(h7, h11_imposed, exit_code)` read off production; a refusal reaches the error outcome only through the refusal; no arm reads the probe for its outcome.
- (f) a6 exit codes unmoved (H11 gap already gives 1); the a6 H7→gap floor path is pinned on Claude by `a_named_directory_without_the_declaration_keeps_the_a6_trade`.
- (g) `ess verify conform synthesize … --out <scratch>/conformance.regen.json` then `cmp` against spec/conformance.json: identical (83 scenarios).

## 6. Paths written outside the worktree

metaharness-wave-adr0096/w2-strict-cwd/adversary-scratch/cases-alone.log, cases-alone-2.log, suite.log, suite-nff.log, conformance.regen.json

## Commands and exit codes

| command | exit |
|---|---|
| `cargo-watched test -p metaharness --test adversary_scratch_cwd` (5 cases, first draft) | 101 |
| `ess verify conform synthesize --path spec --suite-format 5 --strict-requires --out <scratch>/conformance.regen.json` | 0 |
| `cmp spec/conformance.json <scratch>/conformance.regen.json` | 0 |
| `grep -rl '"declared"' crates/*/fixtures evals` | no matches |
| `cargo-watched test -p metaharness --test adversary_scratch_cwd` (2 cases) | 101 |
| `cargo-watched test -p … (five packages)` | 101 (stopped at my binary) |
| `cargo-watched test --no-fail-fast -p … (five packages)` | 101, 647 passed / 2 failed / 649 executed |

```findings
[
  {"file": "crates/metaharness/src/builder.rs", "line": 670, "category": "acceptance", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "start_b10x's a6 branch never attests H7 unavailable, so the floor still prints H7 ok \"which metaharness created\" over an operator's --cwd tree on the b10x arm that aep drive runs, while a27 §4, \"The b10x kind\" and CHANGELOG:49-53 state that wording is corrected."},
  {"file": "crates/metaharness-b10x/src/seam.rs", "line": 163, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "On b10x the opening record's cwd is metaharness's echo of its own plan and the loop's started line names none, so under --scratch-cwd H7 reads ok from the flag plus the echo and a27 §4's \"the record is still the evidence\" holds for no record."},
  {"file": "crates/metaharness/src/audit.rs", "line": 728, "category": "mutant", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "The declared-on-a-non-H7-row → gap arm a27 §2 promises is exercised by no test (every DeclaredControl literal names H7) and reached by no production path."}
]
```
