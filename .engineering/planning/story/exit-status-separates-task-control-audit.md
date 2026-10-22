---
format: aep.planning-md/3
id: story:exit-status-separates-task-control-audit
kind: story
status: draft
title: The run's exit status documents the task, control and audit verdicts separately
revision: 1
---
# Story: The run's exit status documents the task, control and audit verdicts separately

## Scheduling

Draft for wave 8, from the operator's maintainer campaign on release 0.12.0 on 2026-10-21 (finding
F6; the evidence is local to the operator and not in this repository).

## Outcome

A caller can tell from documented output, without reading vendor records, whether the harness child
exited 0, whether every control held, and whether the hermetic audit was conclusive.

## Context

- Observed: audited real sessions exited 3 with child exit 0 and correct task effects, because rows
  H1a, H4 and H5 read unknown. Automation that treats every nonzero exit as task failure reads these
  as failed tasks.
- Unknown rows stay unknown (invariant 3); this story documents and exposes the verdicts, and does not
  turn configured isolation into a pass.

## Acceptance

- expected_before: the CLI reference names exit 3 without saying that the child may have exited 0.
  After: the CLI reference and the audit output name the child exit, the control verdict and the
  audit verdict as separate fields, and a test holds the page to the output.
- An audit with an unknown row still exits nonzero.

## Scope

`website/docs/reference/cli.md`, the audit summary in `crates/metaharness/src/audit.rs` or its
renderer, their tests, `CHANGELOG.md` (to be confirmed by the story scoper).
