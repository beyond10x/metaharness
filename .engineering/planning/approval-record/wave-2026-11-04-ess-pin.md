---
format: aep.planning-md/3
id: approval-record:wave-2026-11-04-ess-pin
kind: approval-record
status: approved
title: 'Wave of 2026-11-04: ESS pin to 0.86.0 and the gate terms for 0.27.0'
summary: The ESS pin moves to 0.86.0 before the gate; gate terms per stage
tags:
- non-interactive
- wave
relations:
- decides: story:ess-pin-0-86-0
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-11-05T00:31:35Z", actor: "human:timo", revision: 2}
---
## ESS pin and gate terms for release 0.27.0

Approved by `b10x-chef-orchestrator` under the delegated grant (class C: a dependency upgrade, including
the ess pin; a release through the repository's own process). Non-interactive: run as a sub-agent of the
orchestrator. It amends `approval-record:wave-2026-11-04`, which said "the ESS pin stays". Quote
(2026-11-05):

> The ess pin moves from 0.77.0 to 0.86.0 in this wave (when a wave touches the repository, the pin moves
> to the newest release). Do it before G1: the pin, plus the ESS crates if the repository uses any. G1 then
> validates with 0.86.0. If 0.86.0 refuses the spec, keep 0.77.0 for this release, put the refusal verbatim
> in a from-finding draft, and tell me.

The repository has no `b10x.toml` and links no ESS crate. The pin is `requires:` in both
`ess-inputs.yaml` files and the `ess-cli` revision CI installs.

Gate terms, as the orchestrator set them: stages G1 to G5 with the peaks and minutes the coordinator
named; each stage starts at peak + 40G free and never under 45G; it stops at peak + 2G or twice its
cargo minutes, counted on cargo time only; two build slots; the cache is discarded between stages; any
stage stops if `/` falls under 40G. One pre-check per stage. G1 starts after U3 correction 1 merges and
the orchestrator sends the go.

| unit | story | files |
|---|---|---|
| U5 | `story:ess-pin-0-86-0` | `spec/ess-inputs.yaml`, `spec/monetary/ess-inputs.yaml`, `.github/workflows/gate.yml`, `spec/conformance.json` if 0.86.0 changes the suite, `spec/coverage.md`, `README.md` (install line), `crates/metaharness-aep/src/drive/ess_conformance.rs` and the tests that name 0.77.0 as the gate prerequisite |
