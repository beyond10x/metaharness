---
format: aep.planning-md/3
id: story:narrowed-cap-refused-before-persist
kind: story
status: draft
title: A narrowed budget cap that the next resume would refuse is refused when it is set
revision: 1
---
# Story: A narrowed budget cap that the next resume would refuse is refused when it is set

## Scheduling

Draft for wave 8, from the operator's maintainer campaign on release 0.12.0 on 2026-10-21 (finding
F3; the evidence is local to the operator and not in this repository).

## Outcome

`metaharness aep drive resume` never persists budget terms that the next unchanged resume refuses.

## Context

- Observed: a run started with a cap of $1 and an assumed charge of $0.10; a resume narrowing the cap
  to $0.05 exited 0 and persisted it; the next unchanged resume refused the remembered terms as
  invalid. No model was launched.
- The narrowing check is `crates/metaharness-aep/src/drive.rs:1761-1767`: it refuses 0 and a raise,
  and does not compare the new cap with the assumed charge.

## Acceptance

- expected_before: cap $1, assumed charge $0.10, narrowed to $0.05: the narrowing exits 0 and the next
  unchanged resume refuses. After: the narrowing is refused before anything is persisted, naming
  the cap and the assumed charge, and the remembered terms are unchanged.
- A narrowing to a cap at or above the assumed charge persists, and the next unchanged resume accepts it.
- Neither case launches a model.

## Scope

`crates/metaharness-aep/src/drive.rs` (resume budget terms), `crates/metaharness-aep/src/drive/spending.rs`
if the check belongs there, their tests, `CHANGELOG.md` (to be confirmed by the story scoper).
