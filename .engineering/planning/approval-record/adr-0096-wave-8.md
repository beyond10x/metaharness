---
format: aep.planning-md/3
id: approval-record:adr-0096-wave-8
kind: approval-record
status: approved
title: 'Wave 8 of 2026-10-22: field-report and trial stories, after the 0.13.0 release'
relations:
- decides: story:codex-command-outcomes-reach-tool-result
- decides: story:narrowed-cap-refused-before-persist
- decides: story:exit-status-separates-task-control-audit
- decides: story:mcp-arguments-checked-against-declared-types
- decides: story:first-user-documents-agree-with-cli
- decides: story:b10x-api-key-from-keychain
revision: 5
transitions:
- {from: "draft", to: "approved", at: "2026-10-21T22:46:16Z", actor: "human:timo", revision: 2}
---
## Status

Approved 2026-10-22 by `b10x-chef-orchestrator` under the owner's standing grant for class-C items:
wave 8 in this repository, its commits, push, pull request and the bot fast-forward of `main`. Not a
release. Skill: aep:implementing 0.23.2, wave mode.

## Order

1. Wave 7 closes and release 0.13.0 completes first (`approval-record:adr-0096-wave-7`).
2. Before wave 8 starts, the per-repository retirement drafts that
   `story:retire-trace-formats-beside-trace-ir` routes (harness `harness events`, takt, loom) go to the
   orchestrator as story text, so it hands them to those repositories' sessions.
3. Then wave 8: each draft below is scoped (`aep:story-scoper`), moved to accepted, and implemented in
   its own managed tree.

## Units

| story | source |
|---|---|
| `story:codex-command-outcomes-reach-tool-result` | field report 2026-10-21, F1 |
| `story:narrowed-cap-refused-before-persist` | field report 2026-10-21, F3 |
| `story:exit-status-separates-task-control-audit` | field report 2026-10-21, F6 |
| `story:mcp-arguments-checked-against-declared-types` | user trial of 0.12.0 |
| `story:first-user-documents-agree-with-cli` | user trial of 0.12.0; field report F5 folded in |
| `story:b10x-api-key-from-keychain` | orchestrator, 2026-10-22, for a takt trial; replaces `story:b10x-api-key-from-file` (archived), operator decision: the key is named by a secret name, `--api-key-secret` |

Commits, review passes and the merge are recorded here as they happen.
