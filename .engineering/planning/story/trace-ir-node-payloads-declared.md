---
format: aep.planning-md/3
id: story:trace-ir-node-payloads-declared
kind: story
status: archived
title: The trace-ir/1 node payloads are declared in the trace domain
refs:
- provider: atlas
  reference: adr-0101
relations:
- decomposes: epic:session-format
revision: 2
transitions:
- {from: "draft", to: "archived", at: "2026-10-20T08:52:35Z", actor: "human:timo", revision: 2}
---
# Story: the trace-ir/1 node payloads are declared in the trace domain

## Scheduling

Draft, filed 2026-10-20 on a relay from session `b10x-chef-orchestrator` (ESS gap survey, 10:50).
Proposed after wave 3 of 2026-10-20, not before.

## Outcome

A consumer that attributes cost or outcome from a `trace-ir/1` document (Atlas ADR 0101) reads the
payload fields of each node family from the ESS domain `metaharness.trace`, where today the domain
declares the node envelope and leaves the payload `UNMAPPED`.

## Context

- `spec/domains/trace.yaml:60-62` (at `3d697011`): "UNMAPPED: the node's kind payload, the event's
  own fields under the family tag, has the IR's per-family field set (projection.rs
  required_ir_fields) and is not re-declared here."
- The per-family field set is `required_ir_fields` (`crates/metaharness-protocol/src/projection.rs:127`).
- The relay names the consumer-side types in aep: `RunUsage` and `RunOutcome` at
  `crates/observe/trace-domain/src/ir.rs:488,512` (aep `8fb443762`, read 2026-10-20), and a
  `ToolCall` payload (location not verified by this session).
- The relay's claim that ADR 0101 attributes cost from these payloads is not verified here.

## Acceptance

`spec/domains/trace.yaml` declares the usage, outcome and tool-call payloads as types with the
fields `required_ir_fields` names for their families, the `UNMAPPED` note at line 60 is removed for
those families, and `ess specify validate --path spec --strict-requires` exits 0, where today the
payload has no typed home.

## Done when

1. One type per payload family the IR writes, each field cited to `projection.rs` or `trace_ir.rs`.
2. A conformance scenario per declared payload reads the field set off a document the production
   writer emitted; the count gate in `ess_conformance.rs` moves by that number.
3. aep's `RunUsage`/`RunOutcome` field names are compared with the declaration and every
   difference is listed in the story body (aep is not changed here).

## Out of scope

- Changing aep's IR types.
- The `metaharness.event/1` and `metaharness.command/1` wire types (`story:wire-formats-declared`).

## Scope

- inferred: `spec/domains/trace.yaml`, `spec/conformance.json` (regenerated),
  `crates/metaharness-aep/src/drive/ess_conformance.rs`, `crates/metaharness-protocol/src/projection.rs`
  (read only), `CHANGELOG.md`.
