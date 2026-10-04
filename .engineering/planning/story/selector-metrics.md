---
format: aep.planning-md/3
id: story:selector-metrics
kind: story
status: draft
title: Compute selector-strategy measurements from Loom telemetry
refs:
- provider: loom
  reference: taskboard:L-009
relations:
- decomposes: epic:selector-strategy-comparison
revision: 1
---
## Outcome

A Metaharness run over a scenario set — each scenario a governed case with a labelled expected
action per selection point — executes Loom once per selection strategy and computes, per strategy:
selection accuracy (chosen = expected), fallback rate (share of selections with `fell_back_to`),
selection latency p50/p95, selection tokens relative to the reasoning-model strategy, downstream
task success (the case reaches its expected outcome), and boundary refusals. Inputs are Loom's
`SelectionRecord`s and session records (beyond10x/loom story:selection-telemetry, `--ref
loom:taskboard:L-009`).

## Acceptance

Named test `selector_comparison_report`: over a fixture scenario set of 3 cases with recorded Loom
sessions for 2 strategies, the report shows the six figures per strategy, equal to values computed
by hand in the fixture's expected file.

## Source

Atlas `docs/design/governed-autonomy/projects/loom-TASKS.md` lines 13-20.
