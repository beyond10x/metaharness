---
format: aep.planning-md/3
id: review-result:adr0096-w3-loops-cwd-adversary-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:b10x-record-carries-the-loops-cwd: 2 warnings, 3 notes, documents only'
relations:
- reviews: story:b10x-record-carries-the-loops-cwd
revision: 1
---
unit: story:b10x-record-carries-the-loops-cwd, pass 1 — uncommitted working tree of wt-e432eccdde80 (base 0a7b3885)
verdict: NEEDS-CHANGE (documents only; no behaviour defect found)
cases: executed 853→855, red 2
origin: introduced 5 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory (metaharness-wave-adr0096/w3/u2-adv1 under the operator's cache directory)
needs-coordinator: none

## Cases added (untracked)

| file / test | checks | state |
|---|---|---|
| `crates/metaharness/tests/adversary_b10x_loops_cwd.rs::the_cli_flag_table_does_not_promise_strict_passes_on_a_b10x_declared_copy` | a strict b10x declared copy over the 0.15.0 golden exits 3 with only H7 not `ok`; the `--scratch-cwd` row of `cli.md` must not promise strict passes on every kind | red at :87 |
| `…::the_changelog_names_the_audited_non_strict_b10x_run_whose_exit_moved_to_3` | `--hermetic --audit` on b10x exits 3 with only H7 not `ok`; the a30 changelog entry must name `--audit` | red at :115 |
| `crates/metaharness-b10x/tests/adversary_native_cli_exits.rs` (4 ignored) | through the production CLI and the 0.15.0 binary on a loopback-only netns: strict without `--cwd` exits 3; strict with `--cwd` (a6) exits 1; `--hermetic` without `--audit` exits 0; `--hermetic --audit` exits 3 | green 4/4 |

```
panicked at crates/metaharness/tests/adversary_b10x_loops_cwd.rs:85:5:
the --scratch-cwd flag-table row promises strict passes on the pair on every kind, and a b10x declared copy under strict exits 3: | `--scratch-cwd` | off | Declares … and `--hermetic strict` passes on the pair — see below. Refused without `--cwd`. |
panicked at crates/metaharness/tests/adversary_b10x_loops_cwd.rs:113:5:
the a30 behaviour-change entry names only strict runs, and `metaharness run b10x --hermetic --audit` also moved from exit 0 to 3: **Behaviour change:** `metaharness run b10x --hermetic strict --cwd <copy> --scratch-cwd` now exits **3** …
test result: FAILED. 0 passed; 2 failed
```

Suite with the cases: `cargo test --locked --workspace --no-fail-fast` 67 binaries, 853 passed, 2 failed
(the cases above), 36 ignored, exit 101.

## Attacked and not broken

No other path builds a b10x `session.started` with a cwd (viewer, `project --from harness-json`, aep
`vendor_emissions` pass `None`; `projection.rs:145` checks the key exists and `cwd: null` is written);
H7 cannot read `ok` on b10x under any flag combination (native: strict, strict `--cwd`, strict
`--cwd --scratch-cwd`, `--hermetic --audit` with and without a copy); Claude and Codex unaffected
(`adapter_class: "harness"`); the constructor change has no external callers (`publish = false`); the
golden changed only `cwd`; a harness record with a `cwd` key is ignored, never half-read; the a6 exit stays
1 (H11). Left as observations: the vector reads the first non-empty line rather than the `started` line,
and a nested `cwd` is not caught; neither reaches anything recorded.

## Findings

```findings
- file: website/docs/reference/cli.md
  line: 38
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the --scratch-cwd flag-table row says strict passes on the pair for every kind, and a b10x declared copy under strict now exits 3
- file: CHANGELOG.md
  line: 58
  category: contract-drift
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the a30 behaviour-change entry names only strict runs, and metaharness run b10x --hermetic --audit also moved from exit 0 to 3
- file: docs/design/metaharness-protocol-v0.1.md
  line: 2461
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a30 (and seam.rs:162) say no released loop writes a started cwd, but harness 0.15.1, tagged on origin before a30 was written, writes a top-level cwd
- file: crates/metaharness/src/builder.rs
  line: 674
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the a6 comment says the floor's H7 reads gap on this kind, and after a30 it reads unk
- file: crates/metaharness/src/audit.rs
  line: 612
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: on a b10x a6 run the H7 row prints unk "the loop did not report its working directory" although the attestation records H7 unavailable for the operator's tree
```

Agent cost: 211,003 tokens, 105 tool uses, 940 s.
