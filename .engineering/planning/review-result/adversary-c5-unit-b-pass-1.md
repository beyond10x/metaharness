---
format: aep.planning-md/3
id: review-result:adversary-c5-unit-b-pass-1
kind: review-result
status: active
title: Adversary pass 1 on wave 2026-10-30-c5 unit B
summary: named scratch root, 3 red cases
relations:
- reviews: story:scratch-root-stable-by-declared-name
revision: 1
---
unit: B, story:scratch-root-stable-by-declared-name, 33bee37 with cases 51a479f on adv/c5-ub
verdict: NEEDS-CHANGE
cases: executed 7→10, red 3
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 (wave scratch unit-b-adv/run1.log)
needs-coordinator: none

```findings
[
  {"file": "crates/metaharness/src/builder.rs", "line": 2024, "category": "concurrency", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "NamedScratch::drop never unlocks explicitly, so a child forked by any other thread of the embedder keeps the flock and 189-244 of 300 back-to-back sessions under one name are refused scratch-name-held (adversary_scratch_name_pass1.rs:191); a72 point 3's release claim is wrong under fork"},
  {"file": "crates/metaharness/src/builder.rs", "line": 2024, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "dropping a named run removes the root and frees the name while its child still runs (Run has no Drop, the child is not killed), so the child's late write lands in the next run's hook channel under the same name (adversary_scratch_name_pass1.rs:133)"},
  {"file": "crates/metaharness/src/builder.rs", "line": 1992, "category": "boundary", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "a leftover root of this user's holding a read-only directory cannot be removed, so every later run under the name is refused; red at adversary_scratch_name_pass1.rs:234, but no child was shown to create such a directory"},
  {"file": "crates/metaharness/src/builder.rs", "line": 1909, "category": "judgement", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "check_spec accepts a name containing ${CLAUDE_PLUGIN_ROOT}, which only the Claude start refuses, after the lock file and root were created; no caller was shown to use such a name"}
]
```

Diff: test-only, `crates/metaharness/tests/adversary_scratch_name_pass1.rs` (51a479f), merged into
unit/c5-ub as 9f484b7. Could not break: identical child paths across two runs on all three kinds,
removal after a strict-version refusal, removal after a panic, one name check across the spec
document, builder, CLI and check_spec.
