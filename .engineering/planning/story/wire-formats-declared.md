---
format: aep.planning-md/3
id: story:wire-formats-declared
kind: story
status: archived
title: metaharness.event/1 and metaharness.command/1 are declared as ESS types
refs:
- provider: atlas
  reference: adr-0096
relations:
- decomposes: epic:session-format
revision: 2
transitions:
- {from: "draft", to: "archived", at: "2026-10-20T08:52:35Z", actor: "human:timo", revision: 2}
---
# Story: metaharness.event/1 and metaharness.command/1 are declared as ESS types

## Scheduling

Draft, filed 2026-10-20 on a relay from session `b10x-chef-orchestrator` (ESS gap survey, 10:50).
Proposed after wave 3 of 2026-10-20, not before.

## Outcome

The two wire formats every embedder speaks have a typed home in the specification, so a consumer
can check its reader against the declaration, where today the domain uses the format names only as
literals.

## Context

- The format tags are constants: `EVENT_FORMAT` and `COMMAND_FORMAT`
  (`crates/metaharness-protocol/src/framing.rs:19,22`, at `3d697011`).
- The types: `Event` (`crates/metaharness-protocol/src/event.rs:333`), `Decision` and `Command`
  (`crates/metaharness-protocol/src/command.rs:17,149`).
- `spec/domains/trace.yaml` names `metaharness.event/1` as a source literal (lines 76, 307, 311,
  343-345) and declares the twenty event names as `WireEventKind` (line 19 onward), but no field
  of an event or a command.
- The relay names five consumers (harness-cli, swarm, aep, agentplugins, a control plane); not
  verified by this session.
- The relay also notes that the conformance report claims only a "selected executable boundary
  projection; not full-system or current-vendor qualification" (`ess_conformance.rs:1821`).

## Acceptance

`spec/` declares the `metaharness.event/1` line and the `metaharness.command/1` line as types whose
fields match `Event` and `Command`, `ess specify validate --path spec --strict-requires` exits 0,
and a conformance scenario per format parses a committed stream line through the declaration,
where today neither format has a declared field.

## Done when

1. The domain (a new `metaharness.wire` domain or the trace domain; the implementor names which and
   why) declares both formats with every field cited to the protocol crate.
2. Scenarios read committed lines (`evals/aep/runs/*.events.jsonl`, the adapters' golden streams);
   the count gate moves by that number.
3. The consumer list is checked and each consumer's reader is named in the story body.

## Out of scope

- Changing either wire format.
- The trace-ir/1 node payloads (`story:trace-ir-node-payloads-declared`).

## Scope

- inferred: `spec/domains/` (new or `trace.yaml`), `spec/system.yaml`, `spec/conformance.json`
  (regenerated), `crates/metaharness-aep/src/drive/ess_conformance.rs`, `spec/coverage.md`,
  `CHANGELOG.md`.
