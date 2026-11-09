---
format: aep.planning-md/3
id: story:claude-task-text-mentions
kind: story
status: active
title: An @ mention in a Claude task reads no file past the seams
refs:
- provider: atlas
  reference: adr-0098
relations:
- decomposes: epic:loom-agent-work-through-metaharness
- informed_by: review-result:adr0096-w4-context-files-adversary-1
- decomposes: epic:github-zero-2026-11-09
- depends_on: story:h11-not-ok-on-unprobed-memory
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/metaharness-claude/src/launch.rs
- confidence: cited
  path: crates/metaharness-claude/src/lib.rs
- confidence: inferred
  path: crates/metaharness-claude/src/vectors.rs
- confidence: cited
  path: crates/metaharness-claude/tests/adapter.rs
- confidence: cited
  path: crates/metaharness-claude/tests/native_fixture.rs
- confidence: inferred
  path: crates/metaharness/src/builder.rs
- confidence: inferred
  path: crates/metaharness/src/refusal.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: cited
  path: website/docs/harnesses/claude.md
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T22:08:16Z", actor: "human:timo", revision: 13}
- {from: "proposed", to: "active", at: "2026-11-09T22:08:16Z", actor: "human:timo", revision: 14}
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

Derived 2026-11-09 by `aep:story-scoper` at `bff6a66`; confidence medium. It replaces the inferred
list of 2026-10-21; every path of that list still exists.

- cited: `crates/metaharness-claude/src/lib.rs:357-430` (`MentionCanary`, `MENTION_CANARIES`,
  `mention_canary`).
- cited: `crates/metaharness-claude/src/launch.rs:938-946` and `:1345` (the task becomes the `-p`
  argument in `build_args`, after `with_agent_execution_context`).
- cited: `crates/metaharness-claude/tests/native_fixture.rs:1321-1455`,
  `crates/metaharness-claude/tests/adapter.rs:131-160`.
- cited: `docs/design/metaharness-protocol-v0.1.md` (a new amendment after a75; it supersedes a35's
  "this amendment does not move it" and closes a51 § 7's open item),
  `website/docs/harnesses/claude.md:29-55`, `CHANGELOG.md`.
- inferred: `crates/metaharness/src/builder.rs` (`check_spec` at `:2695`),
  `crates/metaharness/src/refusal.rs`, `crates/metaharness-claude/src/vectors.rs`; they move only
  with some of the three acceptance options.
- Shares `lib.rs` and the design document with `story:claude-memory-canary-deeper-ancestors` and
  `story:h11-not-ok-on-unprobed-memory`, so it runs after both.
- Not established: whether `Command::MessageInject` is a second user-turn route on Claude; the
  mention grammar on 2.1.309 (measured on the canary's routes only); whether a vendor flag disables
  the expansion.

Native lane, read 2026-11-09: it needs `METAHARNESS_NATIVE_NETNS=loopback-only`, an explicit pinned
binary in `METAHARNESS_NATIVE_CLAUDE`, the candidate driver and an evidence root
(`native_fixture.rs:378`, `:481`, `:536`, `:484`). Pins are 2.1.304, 2.1.305 and 2.1.309. On this
machine the scoper found 2.1.305 only (digest matches the research page); the versions directory
holds 2.1.317 to 2.1.321.
