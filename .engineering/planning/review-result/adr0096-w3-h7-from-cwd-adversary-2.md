---
format: aep.planning-md/3
id: review-result:adr0096-w3-h7-from-cwd-adversary-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:b10x-h7-from-the-loops-cwd: one note, one red case in the new resolver'
relations:
- reviews: story:b10x-h7-from-the-loops-cwd
revision: 1
---
unit: story:b10x-h7-from-the-loops-cwd, pass 2 (the last) — uncommitted tree wt-6d0269524061 after correction round 1 (base d595b42)
verdict: CONFIRMED (one new red case, a note: the new resolver turns a name the kernel refuses into the declared copy; no recorder that exists writes such a name)
cases: executed 869→871, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 7 paths (metaharness-wave-adr0096/w3/u2b-adv2 under the operator's cache directory)
needs-coordinator: none

## Cases added (untracked, `crates/metaharness/tests/adversary_b10x_h7_resolver_pass2.rs`)

| test | asserts | now |
|---|---|---|
| `a_name_through_a_file_does_not_resolve_and_is_not_the_declared_copy` | a record naming `<copy>/README.md/..` (which `canonicalize` refuses with ENOTDIR) under strict `--cwd <copy> --scratch-cwd` reads H7 `gap` and exits non-zero | red |
| `the_resolver_agrees_with_canonicalize_on_ordinary_names` | the floor agrees with `canonicalize` on about 14,000 names (links absolute and relative, link to `/`, `..` after a link, loops, dangling, chains, up-walks), link chains of 36-45, a non-UTF-8 target, a mode-000 directory, a name longer than PATH_MAX | green |

```
H7   ok  gating   record the working directory is ours — the session ran in <scratch>/copy/README.md/.., which was declared, not created: ...
  left: (Ok, "the session ran in .../copy/README.md/.., which was declared, not created: ...")
 right: (Gap, "metaharness planned .../copy and the session ran in .../copy/README.md/..")
test result: FAILED. 1 passed; 1 failed
```

Suite with the cases: 71 binaries, 870 passed, 1 failed (this file), 37 ignored. Clippy exit 0.

## Ledger, pass 1 → pass 2

F1 resolved (pass-1 case green, unedited; 12 spellings pinned); F2 resolved (doc at `audit.rs:389` and on
`FloorInputs::planned_cwd`); F3 resolved (`opening()` pins the exact directory; native 4/4 and 9/9 from the
round's logs, not re-run); F4 resolved (§ 8.1 cell marks a30 superseded). New: F5. Carried 0, new 1,
resolved 4.

## Attacked and not broken

The 40-link bound matches glibc at 36-45; relative targets, `..` after a link, a link to `/`, `/..` and
doubled slashes match `canonicalize`; a relative link collapsing to `/proc` is refused (the check runs on
every resolved prefix); `/proc` as a link is refused before it is followed; `/dev/stdin` reaches `/proc` and
is refused; permission-denied gives `None`; non-UTF-8 and over-long names agree; every failure maps to
`false`, never `ok`. A procfs mounted elsewhere is not detected (stated in a31; this machine has procfs only
at `/proc`).

## Findings

```findings
- file: crates/metaharness/src/audit.rs
  line: 820
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: resolve_without_the_reader applies `..` lexically after a component that is a file, so `<copy>/README.md/..`, which realpath refuses with ENOTDIR, reads H7 ok as the declared copy and a strict b10x run exits 0, contrary to a31's rule that a name that does not resolve is compared as spelled
```

Agent cost: 280,125 tokens, 28 tool uses, 466 s.
