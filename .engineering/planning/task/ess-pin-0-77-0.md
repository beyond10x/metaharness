---
format: aep.planning-md/3
id: task:ess-pin-0-77-0
kind: task
status: implemented
title: Move the specification toolchain to ESS 0.77.0
summary: Both manifests require ess 0.77.0, CI installs ess-cli at f692aabb, conformance.json byte-identical
owner: metaharness
relations:
- decomposes: story:ess-specification-and-hardening
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-29T20:41:35Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-29T20:41:35Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-29T20:46:16Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Purpose

Wave D221 ride-along: move the specification toolchain to the newest ESS release, 0.77.0 (tag object
`2eacff07`, commit `f692aabb`), because the wave changes the repository anyway (workspace rule, ESS
version).

## Change

- `spec/ess-inputs.yaml` and `spec/monetary/ess-inputs.yaml` require `ess 0.77.0`.
- `.github/workflows/gate.yml` installs `ess-cli` from `f692aabbb0fcd321eca0a9eef86e12b25a7601f4`.
- `README.md`, `spec/coverage.md`, `website/docs/quickstart.md` and the three test messages that name the
  prerequisite say 0.77.0.
- `spec/conformance.json` was regenerated with the released, checksum-verified 0.77.0 Linux binary
  (`ess verify conform synthesize --path spec --scenarios spec --suite-format 5 --strict-requires`):
  120 selected scenarios, 6 authored sources, 0 refusals, byte-identical to the 0.76.0 output. Both
  manifests validate with `--strict-requires` (exit 0).

## Acceptance

`task check` on the integration branch with ESS 0.77.0 on `PATH`, every step exit 0.
