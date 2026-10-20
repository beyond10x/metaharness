---
format: aep.planning-md/3
id: story:governance-unit-stated
kind: story
status: active
title: AGENTS.md states the unit of governance for agent work
summary: 'One paragraph in AGENTS.md: until the ADR 0098 gaps close, a caller governs the session and its proposed change, not every tool call.'
owner: metaharness
tags:
- governance
refs:
- provider: atlas
  reference: adr-0098
relations:
- decomposes: epic:loom-agent-work-through-metaharness
scope:
- confidence: cited
  path: AGENTS.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T02:14:54Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-20T02:14:54Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
---
# Story: AGENTS.md states the unit of governance for agent work

## Scheduling

Draft. Not in the wave of 2026-10-20; eligible for any later wave, including as a coordinator-owned
documentation change. Added on the scope critic's finding of 2026-10-20 that the epic promised this
statement and nothing claimed it.

## Outcome

A reader of AGENTS.md learns that, until the six ADR 0098 gaps close, a caller of `metaharness run`
governs an agent session as a whole (the session and its proposed change), not per tool call on every
kind, so nobody builds on per-call governance this repository does not deliver.

## Context

- Atlas ADR 0098 Consequences: "Until they close, the unit of governance for agent work is the
  session and its proposed change."
- `grep -n 'unit of governance' AGENTS.md README.md docs -r` finds nothing on 2026-10-20 (scope
  critic, round 1).
- AGENTS.md § What this repository owns is where the binary's surface is stated; § Safety envelope
  holds the per-call decision rule ("Which tools the harness may call is decided per call, by the
  embedder, through the protocol"), which is about the decision seam and says nothing about who
  performs the effect.

## Acceptance

AGENTS.md § What this repository owns contains one paragraph naming ADR 0098, the six gaps by their
story ids, and the sentence that the unit of governance for agent work is the session and its
proposed change until they close; `task check` exits 0 (the docs check in `metaharness-docs` reads
`website/`, not AGENTS.md, so the gate is unaffected and is run for the record).

## Scope

- cited: `AGENTS.md`. Shared with `story:call-seam-for-harness` (invariant 9 and § Live-evaluating)
  and `story:aep-drive-disposition` (a citation in this same section, § What this repository owns);
  both carry a `depends_on` edge to this story, so this paragraph lands first.
