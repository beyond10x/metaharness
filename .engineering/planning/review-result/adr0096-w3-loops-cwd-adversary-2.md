---
format: aep.planning-md/3
id: review-result:adr0096-w3-loops-cwd-adversary-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:b10x-record-carries-the-loops-cwd: one note, all pass-1 findings resolved'
relations:
- reviews: story:b10x-record-carries-the-loops-cwd
revision: 1
---
unit: story:b10x-record-carries-the-loops-cwd, pass 2 (the last) — uncommitted working tree of wt-e432eccdde80 after correction round 1 (base 0a7b3885)
verdict: CONFIRMED (one note, nothing needing change)
cases: executed 856→857, red 0
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory (metaharness-wave-adr0096/w3/u2-adv2 under the operator's cache directory)
needs-coordinator: none

## Ledger, pass 1 → pass 2

| pass 1 | status | evidence |
|---|---|---|
| F1 `cli.md:38` | resolved | its red case passes; the row is qualified and :163-164 names 0.15.0 |
| F2 `CHANGELOG.md:58` | resolved, leftover N1 | its red case passes; the entry and a30 decision 3 name `--hermetic --audit` and say bare `--hermetic` stays 0 |
| F3 a30 and `seam.rs` | resolved | 0.15.0 writes no `cwd`, 0.15.1 (`f0870131`) writes a top-level one, matching harness `event.rs:202-228` |
| F4 `builder.rs:674` | resolved | the comment says `unk` |
| F5 `audit.rs:612` | resolved by documenting | a30 decision 2 states the a6 case |
| vector read line 1 (not raised) | fixed | the check finds the `started` line; pinned by a new case |

Carried 0, new 1, resolved 5. The pass-1 files are unedited; test fixes changed wording only, never an
assertion.

## Case added

`crates/metaharness/tests/adversary_b10x_loops_cwd_pass2.rs::b10x_audit_without_the_hermetic_flag_exits_3_on_h7_alone`,
green: b10x with `--audit` and no `--hermetic`, over the 0.15.0 golden, with and without a declared
copy, exits 3 with H7 `unk` the only gating row not `ok`.

## Suite

`cargo test --locked --workspace --no-fail-fast`: 68 binaries, 857 passed, 0 failed, 36 ignored, exit 0.
Native lane on a rebuilt CLI with the 0.15.0 binary: 4 passed.

## Findings

```findings
- file: CHANGELOG.md
  line: 60
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: metaharness run b10x --audit without --hermetic also moved from exit 0 to 3, and the CHANGELOG and a30 decision 3 name only the --hermetic --audit form
```

Agent cost: 250,768 tokens, 28 tool uses, 341 s.
