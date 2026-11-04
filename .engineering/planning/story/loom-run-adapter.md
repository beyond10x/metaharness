---
format: aep.planning-md/3
id: story:loom-run-adapter
kind: story
status: draft
title: metaharness run loom observes a b10x-loom run on the event wire
summary: 'Issue 28: a metaharness-loom adapter beside metaharness-b10x; the default-kind change stays out'
tags:
- from-issue
relations:
- decomposes: epic:loom-agent-work-through-metaharness
revision: 1
---
## Scheduling

Draft, filed 2026-11-04 from the idle audit of open GitHub issues:
https://github.com/beyond10x/metaharness/issues/28 (opened 2026-10-05, no store artifact until now).

## Outcome

`metaharness run loom` drives a `b10x-loom run` session and emits `metaharness.event/1` and accepts
the same steering commands as the other kinds, so a consumer of the event stream sees no protocol
change. `metaharness-b10x` stays as it is and selectable.

## Context

- The issue's dependency is met: Loom `story:run-event-stream` is implemented; `b10x-loom run --output
  jsonl` writes the versioned `intake.events.RunEventLine` stream (loom `crates/loom-cli/src/events.rs`,
  read at loom `origin/main` 2026-11-04).
- The issue asks for the Loom adapter to be the native default. Changing which kind is the default
  changes what every caller of `metaharness run` gets; that part is a design decision (invariant 8)
  and is not in this story's acceptance.
- AGENTS.md invariant 2: everything Loom-specific lives in a new `metaharness-loom` crate.
- ADR 0098 points the other way for governance (Loom's session executor runs metaharness sessions);
  this adapter observes a Loom run and is not per-call governance.

## Acceptance

1. A recorded `b10x-loom run --output jsonl` stream from a pinned Loom release replays through
   `metaharness-loom` into a `metaharness.event/1` stream with `session.started`, one `tool.requested`
   and `tool.result` per Loom tool call, and one `session.ended`; a golden expected stream pins it.
2. `metaharness run loom --strict-version` refuses an off-pin `b10x-loom` binary before the spawn.
3. `metaharness run b10x` is unchanged; `task check` exits 0.

## Out of scope

- Making `loom` the default native kind (a design amendment first).
- Steering a Loom run per tool call.

## Scope

- inferred: new `crates/metaharness-loom/`, `crates/metaharness-cli/src/lib.rs` (kind list),
  `crates/metaharness/src/builder.rs`, `Cargo.toml`, `docs/design/metaharness-protocol-v0.1.md`,
  `website/docs/harnesses/`, `CHANGELOG.md`.
