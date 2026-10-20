---
format: aep.planning-md/3
id: approval-record:adr-0096-wave-3
kind: approval-record
status: draft
title: 'Wave 3 of 2026-10-20: b10x adapter on harness 0.15.0, the loop''s cwd, tools contract and crates'
tags:
- wave
refs:
- provider: atlas
  reference: adr-0097
- provider: atlas
  reference: adr-0098
relations:
- decides: story:b10x-adapter-on-harness-over-llm
- decides: story:b10x-record-carries-the-loops-cwd
- decides: story:metaharness-tools-adopts-tool-contract
- decides: story:metaharness-tools-on-tools-crates
revision: 3
---
## Status

Proposed 2026-10-20 by session `metaharness`; approved the same day by `b10x-chef-orchestrator` under
the owner's standing exception for ADR 0096–0101 work, with one change (below). Skill:
aep:implementing 0.23.2, wave mode. Integration tree `wt-4d5f44e5ec8f`, branch
`integration/adr-0096-wave-3`, base `3d697011` (`main`, Release 0.10.0).

## Units

Sequential: each unit shares files with the next. After the re-scope against harness `main`
`947d0e48` (decisions D5, D6) the wave has three units of code and one story closed on evidence.

| unit | story | starts | blocker |
|---|---|---|---|
| 1 | `story:b10x-adapter-on-harness-over-llm` | the harness 0.15.0 sha | `dependency-blocker:harness-0-15-0-release` |
| 2 | `story:b10x-record-carries-the-loops-cwd` | after unit 1 merges | none of its own |
| 3 | `story:metaharness-tools-adopts-tool-contract` | after unit 1 merges (sequential with unit 2) | none of its own |
| — | `story:metaharness-tools-on-tools-crates` | verified after unit 1 merges | superseded by unit 1 (D5) |

Each unit: `aep:implementor` in its own managed tree, then `aep:adversary` (at most 2 passes), the
coordinator verifies the last correction; the whole gate on the integration branch before close.

## Proposal and approval

The proposal (2026-10-20, about 11:05) asked for: the opening store commit; per unit one commit and
one merge into the integration branch (formatting-only fix commits where a merge needs one); the
closing store commit; push, PR and fast-forward of `main` through the bot after CI; one harness build
in a managed harness tree for the golden re-record and the native fixture. Not asked: a paid run, a
release, registry publishing.

`b10x-chef-orchestrator`, 2026-10-20 (verbatim): "Wave 3 approved under exception 5 (the commits,
merges, closing commit, PR + bot fast-forward and the one harness build you listed; no paid run, no
release), with one change. harness 0.15.0 deletes b10x-harness-tools and b10x-harness-toolchain
(tools-1: the Catalogue/Verbs/search layer moves to the new b10x-harness-catalogue, which also
re-exports tool_toolchain) and is close (PR #30 CI, then the short llm-port wave). So U1 pins harness
0.15.0, not 0.14.1, and D3 keeps that layer through b10x-harness-catalogue, not b10x-harness-tools; I
send the 0.15.0 sha. Until then, open the store commit and scope U1/U3/U4 against harness's
integration branch wave/2026-10-20-tools-1 (58b29933). D1, D2 and D4 accepted; for D4, file a harness
draft "the loop record carries its cwd" through me if 0.15.0 still writes none."

## Decisions

- D1 (accepted): unit 1 lands before the tools stories; the `depends_on` edge was reversed. No
  harness type crosses `metaharness-tools`/`metaharness-b10x`.
- D2 (accepted): unit 3's direction is contract name in, existing neutral operation out, harness
  names still accepted. The former wording would deny every owned-surface act under a frame
  (`run.rs:966-969`).
- D3 (changed by the approval): the verb/catalogue/search layer comes from `b10x-harness-catalogue`
  (harness 0.15.0), not `b10x-harness-tools`.
- D4 (accepted): unit 2 is narrowed to the null shape; `run b10x --hermetic strict --cwd <copy>
  --scratch-cwd` moves exit 0 → 3 (H7 gating), named in the changelog. Harness `947d0e48` still writes
  no working directory (`event.rs:114-201`; converter `"cwd": Value::Null`, `metaharness.rs:358`); the
  harness draft goes through `b10x-chef-orchestrator` once 0.15.0 is tagged and still writes none.
- D5 (coordinator, after the re-scope): `story:metaharness-tools-on-tools-crates` is superseded by
  unit 1. Harness 0.15.0 deletes `b10x-harness-tools`, so unit 1 must take `b10x-harness-catalogue`,
  whose `LocalOperations` runs `fs` and `code-editor` through the tools crates (harness
  `crates/harness-catalogue/src/local.rs:243-257,499-557`); its doc lines moved to unit 1; it closes
  on evidence read after unit 1 merges. The alternative (unit 1 renames the dependency key back to
  `b10x-harness-tools`) keeps a key named after a deleted crate.
- D6 (coordinator, after the re-scope): unit 3 takes the contract-name mapping from
  `b10x-harness-catalogue` (`harness_operation`, `local.rs:118-143`) with no tools dependency and no
  wait for tools 0.2.0; the served catalogue keeps harness's names, because the b10x run and the
  metaharness server share one catalogue and harness keeps its names model-facing. Contract schemas
  are not published: harness drops `expected_sha256` (`catalogue.rs:476-483`).
  `dependency-blocker:tools-0-2-0-release` was cleared without the tag, its body says why.
- Unit 1's crate swap uses the crate's own name (`harness_catalogue::`); manifest comments that name
  the deleted crate are deleted, not rewritten.

## Log

- Scoping (2026-10-20): four `aep:story-scoper` passes against harness 0.14.1 and tools 0.1.0 /
  `wave/w2`; after the approval, units 1, 3 and 4 re-scoped against harness `947d0e48`
  (`wave/2026-10-20-tools-1`, merged to harness `main` at that commit, verified by
  `b10x-chef-orchestrator`). Relayed and corrected: `b10x-harness-catalogue` re-exports only
  `tool_toolchain::ResolvedProvider` (`lib.rs:65`); the whole crate is re-exported by
  `b10x-harness-substrate` (`lib.rs:86`); metaharness uses neither. Harness `947d0e48` pins tools
  0.1.0 (`046dfb14`), reachable from tools `origin/main`.
- Release 0.10.0 (not part of this wave, recorded here because the wave's branch carries the store):
  tag `0.10.0` (object `a659a232`, commit `3d697011`) pushed; the Release workflow (run 39396148010)
  built all four archives and `SHA256SUMS` verified, but `b10x-gates scan-artifact` refused the
  x86_64-linux executable: "artifact breaks 1 private rule(s): line 21914 private-identifiers". The
  match is a tracker-key example compiled in from aep-cli's help text at the aep pin `6d7a44d`. The
  GitHub Release is not published. `b10x-chef-orchestrator` decided option A: aep replaces the sample
  keys (aep PR 130), metaharness moves its aep pin and releases 0.10.1 (approved: one pin commit and
  the release through the repository's process); 0.10.0's GitHub Release stays unpublished.
- Disk (Timo, 2026-10-20, relayed): disk space and accounts are `b10x-chef-orchestrator`'s; the
  coordinator's 25G build floor was removed from its build wrapper; one cargo build at a time stays.
  The integration tree's build cache (5,272 MiB) was discarded on request at about 11:15.
