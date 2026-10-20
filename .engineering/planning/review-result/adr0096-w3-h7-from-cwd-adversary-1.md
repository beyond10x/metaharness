---
format: aep.planning-md/3
id: review-result:adr0096-w3-h7-from-cwd-adversary-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:b10x-h7-from-the-loops-cwd: 4 notes, one red case'
relations:
- reviews: story:b10x-h7-from-the-loops-cwd
revision: 1
---
unit: story:b10x-h7-from-the-loops-cwd, pass 1 — uncommitted tree wt-6d0269524061 (branch impl/b10x-h7-from-the-loops-cwd, base d595b42)
verdict: CONFIRMED (one red case, a note: the code breaks a31's own rule, but no loop or vendor that exists today writes the triggering value)
cases: executed 867→868, red 1
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 4 paths (metaharness-wave-adr0096/w3/u2b-adv1 under the operator's cache directory)
needs-coordinator: none (cut once by HTTP 429, resumed on the same model)

## Case added (untracked)

`crates/metaharness/tests/adversary_b10x_h7_proc_self_cwd.rs`: the test process `cd`s into a declared copy
and runs `--hermetic strict --cwd . --scratch-cwd` on b10x over the golden record with the record's `cwd`
set to `/proc/self/cwd` (then `/proc/thread-self/cwd`); expects H7 `gap` and a non-zero exit. Red:

```
H7   ok  gating   record the working directory is ours — the session ran in /proc/self/cwd, which was declared, not created: the embedder declared <scratch>/copy as its disposable scratch copy ...
  left: (Ok, "the session ran in /proc/self/cwd, which was declared, not created: ...")
 right: (Gap, "metaharness planned <scratch>/copy and the session ran in /proc/self/cwd")
test result: FAILED. 0 passed; 1 failed
```

Suite with the case: 70 binaries, 867 passed, 1 failed (this case), 37 ignored.

## Attacked and not broken

Pins (four manifest pins and the lock at `f0870131`; three harness crates, one tools tag 0.2.0, no llm
crate; catalogue, wire, substrate and `price.rs` byte-identical between `dd5c2a6` and `f087013`); golden
provenance (`loop.jsonl` and `version.txt` byte-identical to the chroot recording; binary digest matches;
the emulator saw no authorization; no home path in fixtures); canonical comparison (`..`, trailing slash,
doubled slash resolve; unresolvable, relative, empty and NUL names read `gap`); the floor runs while the
scratch is held; changed adversary assertions moved to the new state (one weakened, F3); a6 never `ok`;
`declared` only under `--scratch-cwd`; a missing or non-string `cwd` maps to null; docs match harness's
`[0.15.1]` changelog.

## Findings

```findings
- file: crates/metaharness/src/audit.rs
  line: 741
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: same_directory canonicalizes /proc/self/cwd inside metaharness, so a record naming it reads H7 ok (declared, exit 0) whenever metaharness runs in the planned directory, contrary to a31's rule that nothing is resolved against metaharness's own working directory
- file: crates/metaharness/src/audit.rs
  line: 381
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the public hermetic_floor doc still says rows come from the emitted events alone, while H7 now depends on filesystem state when the function is called
- file: crates/metaharness-b10x/tests/adversary_native_cli_exits.rs
  line: 338
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the opening-record check moved from cwd null to cwd is_string, so the a6 and no-audit native cases no longer check which directory the record names
- file: docs/design/metaharness-protocol-v0.1.md
  line: 1243
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the H7 table cell still states a30's "names no working directory at the pinned release" without marking it superseded, which is false at the 0.15.1 pin
```

Agent cost: 225,058 tokens, 9 tool uses, 104 s on the final resume.
