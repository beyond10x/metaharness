---
format: aep.planning-md/3
id: story:ess-pin-0-86-0
kind: story
status: active
title: Specify against ESS 0.86.0
scope:
- confidence: cited
  path: .github/workflows/gate.yml
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/metaharness-aep/src/drive/ess_conformance.rs
- confidence: cited
  path: crates/metaharness-aep/tests/adversary_ess_pin_0_66_0.rs
- confidence: cited
  path: crates/metaharness-aep/tests/adversary_session_close_absent_fields.rs
- confidence: cited
  path: spec/conformance.json
- confidence: cited
  path: spec/coverage.md
- confidence: cited
  path: spec/ess-inputs.yaml
- confidence: cited
  path: spec/monetary/ess-inputs.yaml
- confidence: cited
  path: website/docs/quickstart.md
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-11-05T00:31:35Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-05T00:31:35Z", actor: "human:timo", revision: 3}
---
## Context

The specification declares `requires: ess 0.77.0` (`spec/ess-inputs.yaml`, `spec/monetary/ess-inputs.yaml`),
and CI installs `ess-cli` at `f692aabbb0fcd321eca0a9eef86e12b25a7601f4` (`.github/workflows/gate.yml:63`). The
newest ESS release is 0.86.0 (`8d26c81f74f4374316d5ee565c2da43dd81a16a8`). The workspace rule moves a
repository's ESS pin to the newest release when a wave touches the repository anyway.

With a newer `ess` on PATH, two `metaharness-aep` lib cases fail today because `--strict-requires` refuses a
newer release than the pin (`drive::ess_conformance::ess_suite_matches_the_current_specification`,
`drive::ess_conformance::the_reference_model_closes_every_terminal_record_shape_as_production_does`).

ESS 0.86.0 validates both manifests without `--strict-requires` (2026-11-05: `metaharness v1 — 7 file(s),
6 scenario(s), valid`; `metaharness v1 — 2 file(s), valid`).

## Outcome

The repository is specified against ESS 0.86.0: both manifests require it, CI installs it, and the
conformance suite and every test that runs `ess` pass with 0.86.0 on PATH.

## Acceptance

1. `ess specify validate --path spec --strict-requires` and the same for `spec/monetary` succeed with ESS
   0.86.0.
2. `cargo test --locked -p metaharness-aep --lib` passes with ESS 0.86.0 first on PATH, including the two
   `ess_conformance` cases above.
3. `.github/workflows/gate.yml` installs `ess-cli` at `8d26c81f74f4374316d5ee565c2da43dd81a16a8`.

## Out of scope

- ESS features that 0.86.0 adds beyond what the specification already uses.
