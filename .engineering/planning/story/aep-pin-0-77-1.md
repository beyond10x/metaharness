---
format: aep.planning-md/3
id: story:aep-pin-0-77-1
kind: story
status: active
title: Link aep 0.79.0 so a run with --approver gets past an operator step
summary: Retargeted from 0.77.1 to aep 0.79.0; resume takes --evidence
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: crates/metaharness-aep/Cargo.toml
- confidence: inferred
  path: crates/metaharness-aep/src/drive.rs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-04T07:33:26Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"approval":1}}}
- {from: "proposed", to: "active", at: "2026-11-04T07:33:26Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"approval":1}}}
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

Retargeted 2026-11-04 to aep 0.79.0, the newest aep release, on the orchestrator's acceptance. aep 0.77.1
shipped the fix as `aep drive resume <run> --evidence <file>` (aep `CHANGELOG.md`: the pause text names
that route; `aep govern evaluate --evidence --state` writes nothing back). metaharness still links
`23294974` (aep 0.77.0) in `crates/metaharness-aep/Cargo.toml`.

- The AEP crates link the aep 0.79.0 tag commit, the lock holds no second aep revision, and `task check`
  exits 0.
- `metaharness aep drive resume <run> --evidence <file>` hands the file's records to the run before its
  first step; a test drives a run with an approver named to an `operator` step, records a granted
  approval by that approver through that flag, resumes, and the run carries on with the answer in the
  cursor.
- `--aep-binary` and the eval's child `aep` must report 0.79.0.

## Out of Scope

- Any change to AEP itself.

## Ambiguities

- The 0.77.1 interface (snapshot written in place, or a new flag) is not known until it ships.

## Open Questions

None.
