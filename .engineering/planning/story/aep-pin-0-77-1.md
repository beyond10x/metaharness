---
format: aep.planning-md/3
id: story:aep-pin-0-77-1
kind: story
status: draft
title: Link aep 0.77.1 so a run with --approver gets past an operator step
summary: 'Re-pin when aep 0.77.1 ships: govern evaluate --evidence persists the snapshot'
revision: 2
---
# Story: aep-pin-0-77-1

## Outcome

A run started with `metaharness aep drive run --approver <actor>` gets past an `operator` step once the
named approver records a granted approval, because the linked AEP persists what
`aep govern evaluate --evidence` submits.

## Context

`ess-ui-templates` reported on 2026-10-23 that a run with `--approver agent:ess-ui-templates` stops at
an `operator` step and cannot resume. In aep 0.77.0 (`23294974`, linked by metaharness 0.17.0):

- the pause text (`aep-cli/src/drive.rs:1995`) says to record the approval with
  `aep govern evaluate --evidence <file> --state <run>/snapshot.json`;
- `evaluate` (`aep-cli/src/app.rs:1787`) restores the snapshot and submits, and never writes it back;
- on resume, `settle` (`aep-driver/src/run.rs:1783`) finds no new evidence and stops again
  ("nothing was recorded while the run was stopped").

The defect is AEP's. `b10x-chef-orchestrator`, 2026-10-23 (decision at 19:25Z): when aep 0.77.1
(`govern evaluate --evidence` persists) ships, re-pin metaharness to it. Workaround until then:
start the run without `--approver`, so the guard on the artifact decides.

## Acceptance

- The AEP crates link the aep 0.77.1 tag commit, and `task check` exits 0.
- A test drives a run with an approver named to an `operator` step, records a granted approval by
  that approver through the 0.77.1 evidence route, resumes, and the run carries on with the answer
  in the cursor.
- `--aep-binary` and the eval's child `aep` must report 0.77.1.

## Out of Scope

- Any change to AEP itself.

## Ambiguities

- The 0.77.1 interface (snapshot written in place, or a new flag) is not known until it ships.

## Open Questions

None.
