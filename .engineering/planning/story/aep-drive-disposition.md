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
revision: 17
---
# Story: the decided future of `metaharness aep drive` is carried out

## Scheduling

Draft. Blocked by `decision-blocker:aep-drive-future`; not in the wave of 2026-10-20. The blocker
holds the options and their costs; this story is rewritten to the chosen option when it clears.

## Outcome

Whichever option the blocker records is carried out, and the overlap ADR 0098 Consequences names
("`metaharness aep drive` ... overlaps Commission; its future is decided in metaharness's plan") is
closed in this repository's record.

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

Until `decision-blocker:aep-drive-future` clears this story has no end state and stays `draft`;
`aep plan artifact blocked` lists it. On clearing, the acceptance is rewritten to the chosen
option's end state from the Procedure below before the story moves to `proposed`, and in every
option `task check` exits 0 with the ESS conformance count gate in `ess_conformance.rs` equal to the
count in that file at this story's dispatch (written into the Result section), and AGENTS.md § What
this repository owns cites the decision and its reason.

## Procedure, per option

- keep: nothing moves; AGENTS.md § Live-evaluating is unchanged; the citation above is added.
- move to Loom: the ESS conformance target is re-homed into another crate of this workspace first,
  with the count gate unchanged; the frame contract's coordinated migration is written as an Atlas
  ADR before the crate leaves; the `Aep` verb is removed from `metaharness-cli`; the 92 external
  files are listed with their owners in the closing record.
- retire: as for move, minus the Atlas ADR for the frame, plus the statement of what replaces
  `aep-eval driven|native` as the O3/O6 measurement (AGENTS.md § Serves) and the AEP change that
  undoes ADR 0047's redirect to this binary.

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
