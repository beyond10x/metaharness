---
format: aep.planning-md/3
id: epic:selector-strategy-comparison
kind: epic
status: draft
title: Compare Loom selection strategies on governed cases
summary: Selection accuracy, fallback rate, latency, token cost and task success per strategy, from Loom telemetry.
refs:
- provider: atlas
  reference: epic:ga-metaharness-comparison
revision: 1
---
## Outcome

Metaharness compares Loom's action-selection strategies on the same governed cases and reports the
five measurements the Governed Autonomy build pack asks for (Atlas
`docs/design/governed-autonomy/projects/loom-TASKS.md` lines 15-19): selection accuracy, fallback
rate, latency, token/cost reduction and downstream task success — plus unauthorized-action attempts,
which must stay 0. Refines Atlas `epic:ga-metaharness-comparison`.

## Acceptance

A comparison report over one scenario set lists, per strategy (reasoning-model selector; Laya
selector with fallback), the five measurements and the unauthorized-attempt count, read from Loom's
`SelectionRecord`s and session records (beyond10x/loom story:selection-telemetry).

## Source

Atlas `epic:ga-metaharness-comparison`; build pack ROADMAP Phase 8.
