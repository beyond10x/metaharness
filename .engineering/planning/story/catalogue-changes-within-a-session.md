---
format: aep.planning-md/3
id: story:catalogue-changes-within-a-session
kind: story
status: draft
title: The catalogue changes within a session
summary: A frame that changes at a step boundary changes what tool_search answers and what a call may reach, without a new process.
owner: metaharness
tags:
- governance
refs:
- provider: atlas
  reference: adr-0098
relations:
- decomposes: epic:loom-agent-work-through-metaharness
- depends_on: story:caller-supplied-catalogue-on-every-kind
- depends_on: story:decision-returns-caller-produced-result
scope:
- confidence: inferred
  path: crates/metaharness-protocol/src/event.rs
- confidence: inferred
  path: crates/metaharness-tools/src/server.rs
- confidence: inferred
  path: crates/metaharness/src/run.rs
- confidence: inferred
  path: docs/design/metaharness-protocol-v0.1.md
revision: 6
---
# Story: the catalogue changes within a session

## Scheduling

Draft. Not in the wave of 2026-10-20 (owner's instruction). Gap 3 of Atlas ADR 0098 Consequences.

## Outcome

Between two steps of one session the caller changes what the model may call, and the next call sees
the new catalogue: a frontier that moved (Commission's case) narrows or widens the tools without a
new process.

## Context

- The frame is per step already: "the frame is per step, not per session"
  (`docs/design/metaharness-protocol-v0.1.md` § 2.3); `step.entered` / `step.left` are wire events.
- What is per launch is the served tool list: `tools/list` is answered once with the three verbs
  (`crates/metaharness-tools/src/lib.rs`, method table). Because the catalogue sits behind
  `tool_search` and `describe`, swapping it per step changes what those verbs answer without the
  vendor re-listing tools. Inferred, not yet shown by a vector.
- MCP has a `notifications/tools/list_changed` notification; whether Claude Code 2.1.x or Codex
  0.153.x honours it is unverified here and is labelled so (invariant 4).

## Acceptance

A session run with a frame that changes at a step boundary records, in `step.entered`, the catalogue
in force, and a `tool_search` after the boundary returns only the new catalogue; a call to an entry
removed at the boundary is refused and recorded as `tool.decided`; a recorded vector pins it per kind.

## Order

Depends on `story:caller-supplied-catalogue-on-every-kind` (the catalogue) and on
`story:decision-returns-caller-produced-result` (both change `crates/metaharness-tools/src/server.rs`,
`crates/metaharness/src/run.rs`, `crates/metaharness-protocol/src/event.rs` and the protocol design).
`story:continuity-across-executor-calls` follows this story on `run.rs`.

## Out of scope

- Supplying the catalogue in the first place (`story:caller-supplied-catalogue-on-every-kind`).

## Scope

- inferred: `crates/metaharness-tools/src/server.rs`, `crates/metaharness/src/run.rs`,
  `crates/metaharness-protocol/src/event.rs` (`step.entered` payload),
  `docs/design/metaharness-protocol-v0.1.md`.

## Ambiguities

- `requires-stakeholder-input` — whether a widened catalogue is admitted mid-session at all, or only
  narrowing. Default: both, recorded; the governor decides what to send.
