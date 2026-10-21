---
format: aep.planning-md/3
id: story:claude-task-text-mentions
kind: story
status: draft
title: An @ mention in a Claude task reads no file past the seams
refs:
- provider: atlas
  reference: adr-0098
relations:
- decomposes: epic:loom-agent-work-through-metaharness
- informed_by: review-result:adr0096-w4-context-files-adversary-1
revision: 1
---
# Story: an `@` mention in a Claude task reads no file past the seams

## Scheduling

Draft, filed 2026-10-21 on `b10x-chef-orchestrator`'s first condition for wave 4 unit 4 (context files). The
exposure predates that unit; not in wave 4.

## Outcome

A task handed to `metaharness run claude` cannot make Claude Code read a file that no seam sees and no record
names: an `@` mention in the task text either reaches the model as text or is refused before spawn, by a rule
pinned per Claude version.

## Context

- Measured in wave 4 unit 4 (credential-free, native stub, loopback-only namespace, no model), task text whose
  second line is `@marker.txt`:

  | route | 2.1.304 | 2.1.305 |
  |---|---|---|
  | `-p` argument | expanded | expanded |
  | stdin text, with `-p` | expanded | expanded |
  | stdin, `--input-format stream-json` user message | expanded | expanded |
  | `--append-system-prompt` | text, byte-exact | text, byte-exact |
  | `--system-prompt` | text, byte-exact | text, byte-exact |

- Wave 4 unit 4's adversary pass 1 measured the expansion grammar inside prompt text: any `@` token at line
  start or after whitespace expands (code fences, `@../f`, `@/abs`, `@~/f`, directories, untracked files,
  symlinks out of the tree, under every hermetic mode), with no `tool.requested` and no record; an `@` after a
  letter, digit or most punctuation does not.
- No user-turn route stays unexpanded, and moving the task into a system prompt would stop it being the user
  turn and give it system authority (amendment a35). The context-files block uses `--append-system-prompt`;
  the task does not.
- A per-version canary (`MENTION_CANARIES`, unit 4) already pins "expands in `-p`, not in
  `--append-system-prompt`".

## Acceptance

The design amendment chooses and the code implements one of: refuse before spawn a task whose text contains
an expandable mention by the measured grammar (with the Python-decorator and code-snippet false positives
named), neutralise such mentions in the task with the change recorded in the opening record, or a vendor flag
or setting that disables the expansion if one exists at the pin (measured); the choice is pinned by the
per-version canary and a native-fixture case on both binaries; `task check` exits 0.

## Scope

- inferred: `crates/metaharness-claude/src/launch.rs`, `crates/metaharness-claude/src/lib.rs`
  (`MENTION_CANARIES`), `crates/metaharness-claude/tests/native_fixture.rs`, `crates/metaharness/src/builder.rs`,
  `docs/design/metaharness-protocol-v0.1.md`, `website/docs/harnesses/claude.md`, `CHANGELOG.md`.
