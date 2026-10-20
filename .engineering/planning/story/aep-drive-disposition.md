---
format: aep.planning-md/3
id: story:aep-drive-disposition
kind: story
status: draft
title: The decided future of metaharness aep drive is carried out
summary: Keep, move to Loom or retire, as decision-blocker:aep-drive-future records; rewritten to the chosen option when it clears.
owner: metaharness
tags:
- governance
refs:
- provider: atlas
  reference: adr-0098
relations:
- decomposes: epic:loom-agent-work-through-metaharness
- depends_on: story:trace-ir-domain
- depends_on: story:governance-unit-stated
- depends_on: story:project-reads-vendor-transcripts
- depends_on: story:continuity-across-executor-calls
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: README.md
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: crates/metaharness-aep-eval/src/lib.rs
- confidence: inferred
  path: crates/metaharness-aep/Cargo.toml
- confidence: inferred
  path: crates/metaharness-aep/src/drive.rs
- confidence: inferred
  path: crates/metaharness-aep/src/drive/ess_conformance.rs
- confidence: inferred
  path: crates/metaharness-cli/src/lib.rs
- confidence: inferred
  path: docs/ROADMAP.md
revision: 18
---
# Story: the decided future of `metaharness aep drive` is carried out

## Scheduling

Draft. `decision-blocker:aep-drive-future` was cleared on 2026-10-20 with option A (keep; Timo's
answer "a", relayed by session `adr-changes`); this story was rewritten to that option the same day.
Not in the wave of 2026-10-20.

## Outcome

Option A is carried out: `metaharness aep drive` stays the governed-run driver inside this
repository until Loom's session executor runs a case, and the overlap ADR 0098 Consequences names
("`metaharness aep drive` ... overlaps Commission; its future is decided in metaharness's plan") is
closed in this repository's record by a citation of the decision. The B-versus-C question is
re-asked when that executor exists; a new decision-blocker carries it then.

## Context

`crates/metaharness-aep` is 8,087 lines (`wc -l`, 2026-10-20): `drive.rs` 2,710, `drive_tests.rs`
2,829, `drive/ess_conformance.rs` 1,001, `eval.rs` 438, `eval_tests.rs` 609, `drive/spending.rs` 286,
`drive/spending_tests.rs` 196, `lib.rs` 18. It pins eight AEP crates at rev
`6d7a44d3607d2d9a6ffdf0a165993c546c43d0db` (`Cargo.toml:14-20,28`), hosts the ESS conformance target
for every domain in `spec/`, mints `metaharness.frame/1` (AGENTS.md § Safety envelope), and is what
`aep-eval driven|native` drives (`crates/metaharness-aep-eval`). Files naming `metaharness aep drive`
on 2026-10-20: agentplugins 40, atlas 26, aep 16, bench 8, mandate 1, harness 1 (92 outside this
repository), metaharness 11.

## Acceptance

AGENTS.md § What this repository owns carries one sentence that `metaharness aep drive` is kept as
the governed-run driver until Loom's session executor runs a case, naming
`decision-blocker:aep-drive-future` and ADR 0098, where today that section says nothing about the
overlap; `crates/metaharness-aep` is unchanged by this story and `task check` exits 0 with the ESS
conformance count gate in `ess_conformance.rs` equal to the count in that file at this story's
dispatch (written into the Result section).

## Procedure

Option A, keep: nothing moves; AGENTS.md § Live-evaluating is unchanged; the sentence above is
added. The former options B (move to Loom) and C (retire) and their costs stay recorded in the
blocker for the day the question is re-asked.

## Order

Depends on `story:trace-ir-domain` and `story:project-reads-vendor-transcripts` (the count gate and
`ess_conformance.rs` they grow are what the move and retire options re-home) and on
`story:governance-unit-stated` (the same AGENTS.md section, § What this repository owns). No edge to
`story:call-seam-for-harness`: it edits other sections of AGENTS.md.

## Scope

- inferred: `crates/metaharness-aep/Cargo.toml`, `crates/metaharness-aep/src/drive.rs`,
  `crates/metaharness-aep/src/drive/ess_conformance.rs`, `crates/metaharness-aep-eval/src/lib.rs`,
  `crates/metaharness-cli/src/lib.rs`, `Cargo.toml`, `Cargo.lock`, `AGENTS.md`, `README.md`,
  `Taskfile.yml`, `docs/ROADMAP.md`. Narrowed from the whole crate on the parallel-safety finding
  of 2026-10-20; the move and retire options touch every file of the crate and are serialized
  behind `story:trace-ir-domain` by the edge above.
