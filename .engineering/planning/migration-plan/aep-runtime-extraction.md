---
format: aep.planning-md/1
id: migration-plan:aep-runtime-extraction
kind: migration-plan
status: implemented
title: Move concrete AEP execution above the foundation
revision: 4
---
## Decision
The operator approved runtime extraction on 2026-09-09. Metaharness hosts concrete model execution at `metaharness aep drive`, above the neutral AEP foundation (Atlas ADR 0047).

## Repository scope
Move the concrete executor, sealed frame producer, event translation, native hooks and live evaluation into Metaharness. Pin all AEP crates to one exact published revision. Select the planning executable separately; render its actual native mount path and the host's actual continuation command. Preserve authorization, plugin forwarding, spend reservation, compatible legacy launch records and frozen wire contracts.

## Repository acceptance
The complete `task check` passes with the published Git pin and no local Cargo patch. Offline tests cover denial, frame integrity, budgets, plugin forwarding, live-evaluation refusals, native hooks and a legacy paused run whose configuration and spend ledger survive resume without a model launch. Both paused-run outputs name `metaharness aep drive resume`.

## Coordination
AEP publishes the neutral host before this adapter integrates; Agentplugins migrates callers afterward. Atlas's migration-plan:aep-runtime-extraction and task:foundation-composition-evidence own the coordinated catalog updates, final foundation receipt and ER planning validation. This record's implemented status describes the Metaharness source implementation, not completion of Atlas's remaining verification.

No tag, release, deployment or paid evaluation is authorized.
