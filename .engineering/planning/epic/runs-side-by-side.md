---
format: aep.planning-md/1
id: epic:runs-side-by-side
kind: epic
status: implemented
title: Two runs of one task, from any two stacks, read beside each other
summary: A trace-ir/1 reader behind project, a static two-column viewer, and a --plugin option that installs a named marketplace plugin into the scratch home and attests it.
owner: metaharness
tags:
- bench
- trace
revision: 4
---
# Epic: Two runs of one task, from any two stacks, read beside each other

## Outcome

A person comparing two agent runs — Claude Code against Codex, or one spec-driven plugin against another on the same harness — opens one page and reads both event streams aligned: the steps, the tool calls, the refusals, the cost, and where they diverged. Both streams came from this driver, so they are in one format regardless of what ran inside.

## Why Now

`beyond10x/bench` (created 2026-09-02) will hold benchmarks whose first subject is `bdfinst/agentic-dev-team` against `agentplugins`, and later harness against harness. The facts machinery exists (`aep eval run` drives `metaharness` as a tool; `aep eval matrix` counts). What does not exist is the reading surface: `metaharness project` refuses with exit 2 because `trace-ir/1` has no reader yet (`README.md` § Status at `cfcdd7a`), there is no viewer, and a third-party marketplace plugin cannot be placed into the hermetic scratch home, so the alternative stack cannot be run under this driver at all.

## Scope

The `trace-ir/1` reader behind `project`; a static viewer that renders one or two streams side by side; and a `--plugin` option that installs a named marketplace plugin into the scratch config home and attests it.

## Out of Scope

- Scoring or ranking the two streams. The viewer aligns and shows; `aep eval matrix` counts; nobody scores.
- Driving a harness the adapter set does not have. Pi and OpenCode are `docs/ROADMAP.md` § 2.
- Hosting. The viewer is static HTML the bench site embeds.

## Risks

- The alignment rule (by state entry, by tool call index, by time) is a design choice that decides what a reader sees. Mitigation: the design page under `docs/design/` decides it before the viewer is written, per `AGENTS.md` § 7.
- A plugin installed into the scratch home weakens the hermetic promise. Mitigation: it is opt-in, named, and the attestation lists it.

## Ambiguities

- `inferable` — the protocol and IR are bound by `docs/design/metaharness-protocol-v0.1.md`; a change amends that document first (`AGENTS.md:49-55`).
- `requires-stakeholder-input` — the alignment rule. Decides: metaharness owner, on the design page. Default: align by workflow state entry when both runs are driven, else by tool-call index.

## Done When

`metaharness project` reads a recorded stream into `trace-ir/1`; the viewer renders the two recorded `evals/aep` runs beside each other; a run with `--plugin dev-team@bfinster` launches with the plugin present and the attestation says so.
