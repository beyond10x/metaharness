---
format: aep.planning-md/3
id: story:narrowed-cap-refused-before-persist
kind: story
status: implemented
title: A narrowed budget cap that the next resume would refuse is refused when it is set
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/metaharness-aep/src/drive.rs
- confidence: cited
  path: crates/metaharness-aep/src/drive/ess_conformance.rs
- confidence: cited
  path: crates/metaharness-aep/src/drive_tests.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: cited
  path: spec/conformance.json
- confidence: cited
  path: spec/domains/spending.yaml
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-22T00:51:00Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-22T00:51:00Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-22T07:46:30Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
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

Derived 2026-10-22 by `story-scoper` at `a34551e` (release 0.13.0); decisions D5–D7 by the coordinator.

- Primary surface: `crates/metaharness-aep`, resume cost terms.
- `crates/metaharness-aep/src/drive.rs:1731-1772` (`resumed_spend_terms_with_live`; the narrowing check at
  `:1760-1772` refuses 0 and a raise and gains the assumed-charge floor). `drive/spending.rs:121` only calls
  it and is unchanged.
- Persist path: `Host::prepare` (`drive.rs:2648`) builds `spend_policy` into `launch_fields` (`:2682`); the
  pinned aep-cli calls `host.prepare(...)?` before it takes the lock and writes the record, so a refusal
  there persists nothing.
- Tests: `crates/metaharness-aep/src/drive_tests.rs:2619-2655`
  (`a_resume_inherits_or_narrows_its_cap_and_never_widens_it`), including cap equal to the charge.
- ESS: a named outcome for a narrowing below the assumed charge in `spec/domains/spending.yaml` (beside
  `CheckFiniteTerms` at `:75-88`), its scenario, the production arm in
  `crates/metaharness-aep/src/drive/ess_conformance.rs` (`CheckFiniteTerms` arm at `:1799`), and the
  regenerated `spec/conformance.json` (ESS 0.67.0).
- Design: `docs/design/metaharness-protocol-v0.1.md:1927-1931` ("a resume may only narrow that cap") gains the
  floor, as amendment a46.
- `CHANGELOG.md` under `[Unreleased]`.

Decisions:

- D5 the ESS amendment is part of this story.
- D7 a narrowing below spend already reserved (`spent_micro_usd`) is not this story: `prepare` has no run
  directory to read the ledger from.
