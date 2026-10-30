---
format: aep.planning-md/3
id: review-result:adversary-c5-unit-a-pass-1
kind: review-result
status: active
title: Adversary pass 1 on wave 2026-10-30-c5 unit A
relations:
- reviews: story:claude-child-runs-without-auto-memory
- reviews: story:claude-prompt-cache-ttl-declared
revision: 1
---
unit: c5 unit A, bd35f37..af161b3 on unit/c5-ua (worktree mh-c5-ua)
verdict: green, no defect found; 3 judgement findings (1 NEEDS-CHANGE, 1 CONFIRMED, 1 INFEASIBLE)
cases: executed 352→354, red 0
origin: introduced 2 / pre-existing 0 / undecided 1
wrote-outside-worktree: 2 paths under the wave scratch unit-a-adv/ (probe.log, suite.log)
needs-coordinator: no

```findings
[
 {"file": ".engineering/planning/story/claude-child-runs-without-auto-memory.md", "line": null, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The acceptance names a native test native_two_sessions_in_one_cwd_send_one_system_prompt that does not exist, because the check is a helper called inside native_success_failure_model_usage_and_extension_inventory (crates/metaharness-claude/tests/native_fixture.rs:737, called at :788), so running the story's test by name matches nothing and exits 0."},
 {"file": "crates/metaharness-claude/src/transcript.rs", "line": 1446, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The new test was inserted under the doc comment of a_per_request_usage_event_leaves_the_terminal_only_figures_absent, so the new test carries both doc texts and that older test (:1518) has none."},
 {"file": "crates/metaharness-claude/src/launch.rs", "line": 1930, "category": "judgement", "severity": "note", "verdict": "INFEASIBLE", "origin": "undecided", "message": "H3 now states an outcome (auto-memory off, no per-run memory path in the system prompt) that was read from the binary and not run, and managed-settings env could override it the way a33 handles for H4, but /etc/claude-code does not exist on this machine, so nothing shown here reaches that case."}
]
```

- Diff: none. Two probe tests were green and were deleted uncommitted.
- Probes: a codex or b10x run-spec document with prompt-cache-ttl is refused by check_spec with the flag and kind named, and nothing is spawned. An old Usage record without the new keys reads both as None, and Some(0) round-trips.
- Could not break:
  - The inherited env goes through the INHERITED_KEYS allowlist and env_clear (spawn.rs:276).
  - The scrub exemptions are exact matches, not prefixes.
  - The CLI, the spec document and the builder all go through the same check_spec.
  - The assistant-level, result-level and session-file usage share one cache_class.
  - The goldens changed only on lines whose vendor input has cache_creation.
