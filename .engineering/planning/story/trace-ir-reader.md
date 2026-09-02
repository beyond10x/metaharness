---
format: aep.planning-md/1
id: story:trace-ir-reader
kind: story
status: implemented
title: metaharness project reads an event stream into trace-ir/1
summary: 'The reader the README says is missing: byte-stable projection, every event kind mapped or listed as unk, judged by aep trace check.'
owner: metaharness
tags:
- trace
relations:
- decomposes: epic:runs-side-by-side
revision: 4
---
# Story: `metaharness project` reads an event stream into `trace-ir/1`

## Outcome

A recorded run becomes a content-addressed `trace-ir/1` document that `aep trace check` and the viewer both read, so one recording serves judging and reading.

## Context

`README.md` § Status: "`project` | refuses with exit 2 — `trace-ir/1` has no reader yet". The IR is defined on the `aep` side (`crates/trace-domain/src/ir.rs`); `aep trace inspect` already reports events, tool traffic and per-step timings from a transcript. This story gives the driver's own event stream the same destination.

## Acceptance

- `metaharness project <events.jsonl>` writes a `trace-ir/1` document; the same input yields the same bytes (no clock, no network).
- Every event kind the protocol emits maps to an IR node or is listed in the document as `unk` with its kind, never dropped silently.
- `aep trace check` over the projected document judges the two recorded `evals/aep` runs with no `unk` rows for the kinds the driver emits.
- The design document is amended where the mapping is a decision.

## Out of Scope

Projecting a foreign transcript (`audit` over one still refuses); a Claude Code JSONL that did not pass through this driver is not this story.

## Ambiguities

- `inferable` — the IR shape: `aep/crates/trace-domain/src/ir.rs` at `a054945`.

## Open Questions

None.
