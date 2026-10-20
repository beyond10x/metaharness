---
format: aep.planning-md/3
id: story:decision-returns-caller-produced-result
kind: story
status: draft
title: A decision returns a caller-produced result
summary: Answering tool.requested with a result lets Commission perform the effect and hand the harness the output; refused by name where a kind cannot deliver it.
owner: metaharness
tags:
- governance
refs:
- provider: atlas
  reference: adr-0098
relations:
- decomposes: epic:loom-agent-work-through-metaharness
- depends_on: story:caller-supplied-catalogue-on-every-kind
scope:
- confidence: inferred
  path: crates/metaharness-protocol/src/command.rs
- confidence: inferred
  path: crates/metaharness-protocol/src/event.rs
- confidence: inferred
  path: crates/metaharness-tools/src/server.rs
- confidence: inferred
  path: crates/metaharness/src/run.rs
- confidence: inferred
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: inferred
  path: spec/domains/boundaries.yaml
revision: 9
---
# Story: a decision returns a caller-produced result

## Scheduling

Draft. Not in the wave of 2026-10-20 (owner's instruction). Gap 2 of Atlas ADR 0098 Consequences.

## Outcome

Answering a `tool.requested` with a result performs nothing in metaharness: the caller (Commission,
through `EffectPort::invoke`, ADR 0082) performs the effect and the harness receives the caller's
result as the tool's output. ADR 0098 point 4 names the absence of this as the reason plugin turns
move to Loom's llm chooser ("metaharness 0.9.3 cannot let its caller perform an effect").

## Context

- `Decision` is `Allow | Deny { reason } | Replace { input } | Abstain`
  (`crates/metaharness-protocol/src/command.rs:14-58`). `Replace` exists because "both vendors' hook
  wires carry an updated input"; neither wire carries a result, so a result cannot cross the hook
  seam on the native tool surface.
- On the owned surface the server *is* the tool: `tools/call` is answered by `metaharness_tools::Server`
  from `LocalOperations` (`crates/metaharness-tools/src/lib.rs`, the method table). A caller-produced
  result is deliverable there by answering `tools/call` with the caller's payload instead of
  performing the operation locally.
- The per-call round trip already exists as `DecisionMode::Ask` ("the embedder decides; the run
  blocks, one round trip per call", `spec.rs:60-61`).

## Acceptance

On Claude Code and on Codex, a run started with `--decisions ask --tool-surface owned --results
caller`: the caller answers a `tool.requested` with `decision: result` and a payload, the wire then
carries `tool.decided { decided_by: "<caller>" }` and a `tool.result` whose `content` is that
payload, and the model's next turn reads it; a recorded vector pins each of the two kinds, and a
kind that cannot deliver it is a finding, not a pass. Only `b10x` refuses by name. The refusals,
each observable: `--results caller` with `--tool-surface native`, or on a kind that cannot
deliver a result, is refused by name before spawn (exit 2, the refusal code in the opening
`warning`); a `decision: result` answer on a run started without `--results caller` is answered with
`command.result { error }` and the call is decided `deny` with that reason in `tool.decided`, never
downgraded to `allow`.

## Order

Depends on `story:caller-supplied-catalogue-on-every-kind`: both change `crates/metaharness-tools/src/server.rs`,
`crates/metaharness/src/run.rs` and the protocol design, and the result is delivered through the
owned surface that story opens to the caller. `story:catalogue-changes-within-a-session` follows this
one on the same files.

## Out of scope

- Who supplies the catalogue (`story:caller-supplied-catalogue-on-every-kind`).
- Commission's own effect invocation and rechecking (Loom, ADR 0082).

## Scope

- inferred: `crates/metaharness-protocol/src/command.rs`, `crates/metaharness-protocol/src/event.rs`,
  `crates/metaharness-tools/src/server.rs`, `crates/metaharness/src/run.rs`,
  `docs/design/metaharness-protocol-v0.1.md` § 6 (amendment), `spec/domains/boundaries.yaml`.

## Ambiguities

- `inferable` — the wire shape of the result (content blocks vs one JSON value): follow the MCP
  `tools/call` result the server already emits ("one JSON text content block").
