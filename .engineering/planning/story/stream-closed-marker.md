---
format: aep.planning-md/3
id: story:stream-closed-marker
kind: story
status: implemented
title: A run's stream ends with a closing event that says it is complete
summary: A terminal stream.closed event with count and reason, in the attestation and the IR, so aep trace check can decide negative rows instead of leaving them unknown.
owner: metaharness
tags:
- bench
- protocol
relations:
- decomposes: epic:runs-side-by-side
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-03T03:48:59Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"test_result":1}}, imported: true}
- {from: "proposed", to: "active", at: "2026-09-03T03:48:59Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1}}, imported: true}
- {from: "active", to: "implemented", at: "2026-09-03T03:48:59Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}, imported: true}
---
# Story: A run's stream ends with a closing event that says it is complete

## Outcome

A checker reading a stream this driver wrote can tell a finished run from a truncated one from the file alone, so an expectation about something that never happened can be decided rather than left unknown.

## Context

On 2026-09-03 eight live eval cases ended "undecided" in `aep trace check` because every negative row (`nothing-was-moved`, `no-store-command-was-run`, `nothing-was-written-to-tmp` …) is `unk`: the checker cannot distinguish an absence from a hole. This driver owns the stream and knows when it ended; the protocol document is the place to say so (AGENTS.md § 8: design before code).

## Acceptance

- The protocol emits a terminal `stream.closed` event (name per the design amendment) carrying the event count and the reason the run ended (`completed`, `budget`, `killed`, `error`); it is the last line of every stream this driver writes.
- The hermetic attestation records the same completeness fact.
- `metaharness project` maps the event into `trace-ir/1`; conformance vectors record it; the two committed `evals/aep/runs` streams are regenerated with it.
- A truncated stream (no closing event) is recognised by `audit` and named as such.

## Out of Scope

Deciding rows; that is aep `story:absent-rows-decide-on-a-closed-stream`, which reads this marker.

## Ambiguities

- `inferable` — the event vocabulary and the amendment record: `docs/design/metaharness-protocol-v0.1.md`.

## Open Questions

None.
