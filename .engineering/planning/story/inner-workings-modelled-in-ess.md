---
format: aep.planning-md/3
id: story:inner-workings-modelled-in-ess
kind: story
status: draft
title: 'metaharness''s inner workings are modelled in ESS: trace-ir payloads, event and command wire, conformance past the boundary projection'
refs:
- provider: atlas
  reference: adr-0096
- provider: atlas
  reference: adr-0101
relations:
- decomposes: epic:session-format
- supersedes: story:trace-ir-node-payloads-declared
- supersedes: story:wire-formats-declared
revision: 1
---
# Story: metaharness's inner workings are modelled in ESS

## Scheduling

Draft, filed 2026-10-20 on Timo's request relayed by session `b10x-chef-orchestrator` (about 11:00):
model metaharness's inner workings in ESS with the `ess:specifying` skill. Supersedes the two drafts
filed from the same session's ESS gap survey (10:50), `story:trace-ir-node-payloads-declared` and
`story:wire-formats-declared`, which are archived. Proposed in a wave after wave 3 of 2026-10-20.

## Outcome

The trace-ir/1 node payloads, the `metaharness.event/1` stream and the `metaharness.command/1`
channel are declared in the specification as types, entities and commands, not only as names, and
the conformance suite drives the production code through them beyond the boundary projection it
claims today, so a consumer (Atlas ADR 0101 cost attribution, the five embedders of the wire) can
check its reader against a declaration rather than against this repository's Rust.

## Context

Read at `3d697011` (Release 0.10.0) unless noted.

- **Payloads.** `spec/domains/trace.yaml:60-62`: "UNMAPPED: the node's kind payload, the event's own
  fields under the family tag, has the IR's per-family field set (projection.rs
  required_ir_fields) and is not re-declared here." The field set is `required_ir_fields`
  (`crates/metaharness-protocol/src/projection.rs:127`). The consumer-side types are in aep:
  `IR_FORMAT` (`crates/observe/trace-domain/src/ir.rs:45`), `ToolCall` (`:272`), `RunUsage`
  (`:488`), `RunOutcome` (`:512`), at aep `8fb443762`. The relay states ADR 0101 attributes cost
  from these; not verified here.
- **Wire formats.** `EVENT_FORMAT` and `COMMAND_FORMAT` (`crates/metaharness-protocol/src/framing.rs:19,22`);
  `Event` (`event.rs:333`); `Decision` and `Command` (`command.rs:17,149`); `CommandOutcome`
  (`command.rs:130`). `spec/domains/trace.yaml` uses `metaharness.event/1` as a source literal
  (lines 76, 307, 311, 343-345) and declares the twenty event names as `WireEventKind`, but no
  event or command field. The relay names five consumers (harness-cli, swarm, aep, agentplugins, a
  control plane); not verified here.
- **Conformance claim.** The report the target writes says
  `"claim":"selected executable boundary projection; not full-system or current-vendor qualification"`
  (`crates/metaharness-aep/src/drive/ess_conformance.rs:1821`; the relay cited `:1642` on an
  earlier tree). The suite has 87 scenarios (`spec/conformance.json`).

## Acceptance

`spec/` declares (1) the trace-ir/1 usage, outcome and tool-call payloads with the fields
`required_ir_fields` names, the `UNMAPPED` note at `trace.yaml:60` removed for them; (2)
`metaharness.event/1` lines and `metaharness.command/1` lines as entities with their commands
(`Decision`, `Command`, `CommandOutcome`), every field cited to the protocol crate; `ess specify
validate --path spec --strict-requires` exits 0; and the conformance target executes scenarios for
each new declaration against production code, where today none of them has a typed home.

## Done when

1. The `ess:specifying` skill's procedure is followed; every relation the implementor cannot read
   from code carries an `UNMAPPED:` marker named in the report.
2. One conformance scenario per declared payload and per command reads a document or a stream line
   the production code emitted (`evals/aep/runs/*`, the adapters' golden streams); the count gate in
   `ess_conformance.rs` moves by that number, and guard-negation mutation kills every new guard.
3. The report's `claim` string is re-read against what now executes and changed only as far as the
   new scenarios justify; what still is not qualified is named.
4. aep's `ToolCall`, `RunUsage` and `RunOutcome` field names are compared with the declaration and
   each difference is listed in this body (aep is not changed here).
5. The consumer list is checked and each consumer's reader is named in this body.

## Out of scope

- Changing either wire format, or aep's IR types.
- A paid vendor run.

## Scope

- inferred: `spec/domains/trace.yaml`, a new domain under `spec/domains/` for the wire if the
  implementor separates it, `spec/system.yaml`, `spec/coverage.md`, `spec/conformance.json`
  (regenerated with ESS 0.56.0 only), `crates/metaharness-aep/src/drive/ess_conformance.rs`,
  `crates/metaharness-protocol/src/projection.rs` (read only), `docs/design/` amendment tail,
  `CHANGELOG.md`.
- Shares `ess_conformance.rs` (count literal, obligation list) and `spec/conformance.json` with any
  story that adds scenarios; sequential with those.
