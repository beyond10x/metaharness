---
format: aep.planning-md/3
id: task:ess-pin-0-76-0
kind: task
status: implemented
title: Move the specification toolchain to ESS 0.76.0
summary: Both manifests require ess 0.76.0, CI installs ess-cli at d5c00e9c, conformance.json byte-identical
owner: metaharness
relations:
- decomposes: story:ess-specification-and-hardening
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-29T09:57:02Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-29T09:57:02Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-29T10:01:20Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1},"asserted":{"test_result":1}}}
---
## Purpose

Wave D193 ride-along: move the specification toolchain to the newest ESS release, 0.76.0 (tag object
`0a149405`, commit `d5c00e9c`), as the workspace rule requires.

## Change

- `spec/ess-inputs.yaml` and `spec/monetary/ess-inputs.yaml` require `ess 0.76.0`.
- `.github/workflows/gate.yml` installs `ess-cli` from `d5c00e9c73e72d792a390091b8442b22702640b0`.
- `README.md`, `spec/coverage.md`, `website/docs/quickstart.md` and the three test messages that name the
  prerequisite say 0.76.0.
- `spec/conformance.json` was regenerated with the released, checksum-verified 0.76.0 Linux binary
  (`ess verify conform synthesize --path spec --scenarios spec --suite-format 5 --strict-requires`):
  120 selected scenarios, 0 refusals, byte-identical to the 0.75.0 output. Both manifests validate with
  `--strict-requires`.

## Acceptance

`task check` on the integration branch with ESS 0.76.0 on `PATH`, every step exit 0.
