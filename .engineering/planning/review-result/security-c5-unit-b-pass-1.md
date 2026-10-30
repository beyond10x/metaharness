---
format: aep.planning-md/3
id: review-result:security-c5-unit-b-pass-1
kind: review-result
status: active
title: Security review pass 1 on wave 2026-10-30-c5 unit B
summary: named scratch root custody, 4 red cases
relations:
- reviews: story:scratch-root-stable-by-declared-name
revision: 1
---
unit: B, story:scratch-root-stable-by-declared-name, 33bee37 with cases 9c55c33 on unit/c5-ub
verdict: NEEDS-CHANGE
cases: executed 11→17, red 4
origin: introduced 7 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 (wave scratch unit-b-sec/, now empty)
needs-coordinator: no

```findings
[
  {"file": "crates/metaharness/src/builder.rs", "line": 1988, "category": "concurrency", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The flock holds the lock file's inode, not its path: if the lock file is unlinked while a run holds it, a second run under the name creates a new lock, takes it, and removes the live first run's root (red: a_held_name_stays_held_after_its_lock_file_is_unlinked)."},
  {"file": "crates/metaharness/src/builder.rs", "line": 2037, "category": "integrity", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "The lock is opened O_RDONLY without O_NONBLOCK, so a FIFO at the lock path makes the start wait in open() for a writer instead of refusing it as not a regular file; only a same-user FIFO or a non-sticky shared TMPDIR reaches this (red: a_fifo_at_the_lock_path_is_refused_without_waiting_for_a_writer)."},
  {"file": "crates/metaharness/src/builder.rs", "line": 2007, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "A leftover of this user's that holds a directory without owner write permission is never replaced: the drop's remove_dir_all fails silently, and every later run under the name is refused with Permission denied (red: a_leftover_holding_a_read_only_directory_is_replaced)."},
  {"file": "crates/metaharness/src/builder.rs", "line": 1952, "category": "contract-drift", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "Under a relative TMPDIR the named root stays relative while tempfile makes an unnamed root absolute, which breaks a72 section 1 (red: a_named_root_is_absolute_as_an_unnamed_one_is)."},
  {"file": "crates/metaharness/src/run.rs", "line": 253, "category": "judgement", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "Run has no Drop: process drops before scratch, and dropping SpawnedProcess neither kills nor waits the child, so an abandoned run releases the name while its child still writes under a root path the next run under the name reuses."},
  {"file": "crates/metaharness/src/builder.rs", "line": 2004, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Any same-uid directory at <tmp>/metaharness-<NAME> counts as a leftover and is removed recursively, with no marker, while metaharness-docs tests make <tmp>/metaharness-docs-<pid>-<label> in the same namespace."},
  {"file": "docs/design/metaharness-protocol-v0.1.md", "line": 5972, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Section 4's safety argument assumes a sticky TMPDIR and nothing checks it, and the predictable name lets another local user squat metaharness-<NAME> or its lock; a per-user 0700 parent checked for its owner would close both."}
]
```

Diff: test-only, `crates/metaharness/tests/scratch_name_custody.rs` (9c55c33). Sound: no link
followed at the root, the lock or inside a leftover; 0700 root and 0600 lock under umask 0; name
validation; lock lifetime and O_CLOEXEC; a sticky /tmp with protected_* lets another user block a
name but not redirect it.
