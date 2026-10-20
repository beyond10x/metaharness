---
format: aep.planning-md/3
id: story:b10x-h7-from-the-loops-cwd
kind: story
status: implemented
title: The b10x H7 row reads the working directory the loop reports (harness 0.15.1)
refs:
- provider: atlas
  reference: adr-0098
relations:
- decomposes: epic:loom-agent-work-through-metaharness
- depends_on: story:b10x-record-carries-the-loops-cwd
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T17:23:11Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-20T17:23:11Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-20T19:39:50Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"verification":1,"ess_conformance_coverage_v1":1}}}
---
# Story: the b10x H7 row reads the working directory the loop reports (harness 0.15.1)

## Scheduling

Draft, filed 2026-10-20 after harness 0.15.1 shipped the field `story:b10x-record-carries-the-loops-cwd`
asked for (the harness draft routed by `b10x-chef-orchestrator`). Follows that story (unit 2 of wave 3
of 2026-10-20), which sets `cwd` to `null` and H7 to `unk` on 0.15.0. `b10x-chef-orchestrator`
(2026-10-20): "Your follow-up to story:b10x-record-carries-the-loops-cwd (H7 from the record) can pin
0.15.1 in your next unit."

## Outcome

On the b10x kind, `session.started.cwd` is the working directory the loop itself reported, and the
floor's H7 row is decided by comparing that record with the directory metaharness planned, so H7 can
read `ok` from an observation rather than from metaharness's own plan; where the loop reports none,
H7 stays `unk` (the previous story's branch).

## Context

- harness 0.15.1: tag object `a0a2dfdaf5b2702db5c77412c28f98b95f0c8794` (tagger `b10x-bot[bot]`) on
  commit `f0870131e58df3e1119860d0a255f46589026d62`, harness main (read with `gh api`, 2026-10-20).
  Relayed by `b10x-chef-orchestrator`, which verified it: the `started` record carries `cwd`, absent
  when the path is not UTF-8 and under `--substrate <socket>`; the converter writes
  `session.started.cwd` (harness `5dccbd24`, merge `9312d84d`); llm pinned at 0.8.1.
- Unit 1 of wave 3 pinned harness 0.15.0 (`dd5c2a66`) and re-recorded the b10x golden; unit 2 makes
  the seam write `null` and the floor read `unk` with the reason "the loop did not report its working
  directory".
- Under `--hermetic strict --cwd <copy> --scratch-cwd` a b10x run exits 3 after unit 2 (H7 gating);
  with the record carrying the directory, a27's `declared` semantics apply to the observed value.

## Acceptance

Every harness git pin and `PINNED_VERSIONS`/`HARNESS_REVISION` name 0.15.1 (`f0870131`), `Cargo.lock`
one harness and one tools revision; the b10x golden is re-recorded from a 0.15.1 binary with harness's
emulator (no paid run) and its `started` line carries `cwd`; the seam sets `session.started.cwd` from
the record's `cwd` when present and `null` when absent; H7 reads `ok` only when the recorded directory
equals the one metaharness created, `declared` under `--scratch-cwd` when it equals the declared copy,
`gap` when it differs, `unk` when absent; a vector per shape (present and equal, present and different,
absent) over recorded or derived records; the strict declared-copy b10x run's exit is pinned and named
in the changelog; the research page and the design doc name the change.

## Out of scope

- A paid run.
- Other harness 0.15.1 changes beyond what the adapter reads (listed in the research page).

## Scope

- inferred: `crates/metaharness-b10x/{Cargo.toml,src/lib.rs,src/seam.rs,src/vectors.rs}`,
  `crates/metaharness-tools/Cargo.toml`, `Cargo.lock`, `crates/metaharness-b10x/fixtures/golden/`,
  `crates/metaharness/fixtures/golden/contract-result-b10x.json`, `crates/metaharness/src/audit.rs`,
  `crates/metaharness/tests/adversary_scratch_cwd.rs`, `crates/metaharness-b10x/tests/native_fixture.rs`,
  `docs/research/`, `docs/design/metaharness-protocol-v0.1.md`, `website/docs/hermetic.md`,
  `CHANGELOG.md`. Shares all of these with unit 2; sequential after it.
## Delivered (wave 3 of 2026-10-20, unit 2b)

Commit `2b03d47`, merged `34c427b`; two `aep:adversary` passes
(`review-result:adr0096-w3-h7-from-cwd-adversary-1`, `-2`) and a coordinator-verified last correction.
Harness 0.15.1 pinned; golden re-recorded inside a private mount namespace; the seam reads the record's
`cwd`; H7 on b10x reads `ok`/`declared`/`gap`/`unk` from it (amendment a31). Directory comparison on every
kind: byte-equal, or both absolute and resolved to the same directory by a realpath-style walk that
refuses `/proc` and `/dev/fd` prefixes and ENOTDIR shapes and never resolves a relative name. A clean
strict or `--audit` b10x run exits 0 again; native lanes 9/9 and 4/4 on 0.15.1. Edited outside the
inferred scope: `adversary_project_from.rs`, `tests/audit.rs`, `website/docs/harnesses/b10x.md`,
`spec/coverage.md`, the CLI reference.
