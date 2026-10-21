---
format: aep.planning-md/3
id: story:ess-pin-0-66-0
kind: story
status: implemented
title: The specification and the gate move to ESS 0.66.0
refs:
- provider: atlas
  reference: adr-0096
relations:
- decomposes: epic:session-format
scope:
- confidence: inferred
  path: .github/workflows/gate.yml
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: README.md
- confidence: inferred
  path: crates/metaharness-aep/src/drive/ess_conformance.rs
- confidence: inferred
  path: spec/conformance.json
- confidence: inferred
  path: spec/coverage.md
- confidence: inferred
  path: spec/ess-inputs.yaml
- confidence: inferred
  path: spec/monetary/ess-inputs.yaml
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T20:30:18Z", actor: "human:timo", revision: 10}
- {from: "proposed", to: "active", at: "2026-10-20T20:30:18Z", actor: "human:timo", revision: 11}
- {from: "active", to: "implemented", at: "2026-10-21T09:47:36Z", actor: "human:timo", revision: 13, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"verification":1,"ess_conformance_coverage_v1":1}}}
---
# Story: the specification and the gate move to ESS 0.66.0

## Scheduling

Filed 2026-10-20 on Timo's request relayed by `b10x-chef-orchestrator` ("we want all to upgrade to most
recent ess ... whenever one of the things you control is being touched and made ready for a release"),
approved as its own unit of wave 4, before the release.

## Outcome

metaharness's specification, its conformance suite and its CI gate run on ESS 0.66.0 (the newest
release on 2026-10-20), so the repository stops pinning 0.56.0.

## Context

- Pins: `requires: ess 0.56.0` in `spec/ess-inputs.yaml:2` and `spec/monetary/ess-inputs.yaml`; CI installs
  `ess-cli` from the 0.56.0 commit (`.github/workflows/gate.yml:67`, rev `84ee8d38`). ESS 0.66.0: tag object
  `55d301e9`, commit `145a1b9d` (tagger `b10x-bot[bot]`); the linux release asset checks against the
  release's `SHA256SUMS` (read 2026-10-20).
- Probe on `main` `6b984c46` with the 0.66.0 binary: `ess specify validate` passes (7 and 2 files) apart from
  the requires line; `ess verify conform synthesize --path spec --suite-format 5` is refused with three
  `ESS-SYNTH-022` occurrences ("leaves a connective's child unwitnessed"):
  `metaharness.session.CloseUnsteeredStream/failed-or-unknown-terminal` and
  `metaharness.spending.CheckFiniteTerms/invalid-finite-terms` (two). ESS's help: cover the child with an
  authored scenario (`ess-scenario/1`) or write the guard so such an input exists. 87 scenarios are
  otherwise selected.

## Acceptance

Both requires lines name 0.66.0 and `--strict-requires` passes with the 0.66.0 binary; CI installs
`ess-cli` at the 0.66.0 tag's commit; each unwitnessed child is covered by an authored scenario where the
guard is right, and a guard changes only where the specification is wrong (each such change named);
`spec/conformance.json` is regenerated with 0.66.0 and runs green on the Rust conformance target, its
scenario count 87 plus the authored ones, the count gate in `ess_conformance.rs` moved by that number;
`task check` exits 0; one `CHANGELOG.md` line names the move.

## Out of scope

- New specification content beyond what the move needs.

## Scope

- inferred: `spec/ess-inputs.yaml`, `spec/monetary/ess-inputs.yaml`, `.github/workflows/gate.yml`,
  `spec/domains/` (the two outcomes' guards or authored scenarios), `spec/conformance.json`,
  `crates/metaharness-aep/src/drive/ess_conformance.rs`, `spec/coverage.md`, `README.md:81` (names the ESS
  version), `CHANGELOG.md`.
## Delivered (wave 4 of 2026-10-21, unit 1)

Commit `49d98f4`, merged `5eacbd4`; two `aep:adversary` passes (`review-result:adr0096-w4-ess-pin-adversary-1`,
`-2`), the last correction verified by the coordinator. `ESS_VERSION` 0.66.0; the suite regenerated with
`--suite-format 5 --strict-requires`. Two guards changed because the specification was wrong, not to get
past the synthesizer: the three `ESS-SYNTH-022` refusals stayed with authored scenarios admitted through
`ess-inputs.yaml` (ESS issues 619 and 620, fixes planned for ESS 0.67.0). The two scenarios that asserted
an outcome on an absent `is_error` moved to Rust tests beside `reason_from_record`; the guard says the
specification does not decide such records (`story:session-close-decides-absent-fields`, after ESS 0.67.0).
Carried: pass 2's F2, the ignored acceptance case of its story.
