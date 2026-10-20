---
format: aep.planning-md/3
id: story:metaharness-tools-adopts-tool-contract
kind: story
status: implemented
title: metaharness-tools resolves the tool-contract names harness maps
summary: resolve_verb, subjects_of_verb and Server::call accept the contract names b10x-harness-catalogue maps (fs.read, fs.list, fs.glob, code.write, code.edit) and resolve them as their harness entries; no tools dependency.
owner: metaharness
tags:
- pins
refs:
- provider: atlas
  reference: adr-0095
- provider: atlas
  reference: adr-0097
relations:
- decomposes: epic:layer-pins-follow-their-owners
- depends_on: story:b10x-adapter-on-harness-over-llm
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: inferred
  path: crates/metaharness-tools/src/lib.rs
- confidence: cited
  path: crates/metaharness-tools/src/resolve.rs
- confidence: cited
  path: crates/metaharness-tools/src/server.rs
- confidence: cited
  path: crates/metaharness-tools/src/tests.rs
revision: 44
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T09:05:37Z", actor: "human:timo", revision: 30}
- {from: "proposed", to: "active", at: "2026-10-20T09:05:38Z", actor: "human:timo", revision: 31}
- {from: "active", to: "implemented", at: "2026-10-20T19:39:50Z", actor: "human:timo", revision: 44, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"verification":1,"ess_conformance_coverage_v1":1}}}
---
# Story: `metaharness-tools` resolves the tool-contract names harness maps

## Scheduling

Unit 3 of wave 3 of 2026-10-20, after `story:b10x-adapter-on-harness-over-llm` (unit 1,
`depends_on`), which pins harness 0.15.0 and takes `b10x-harness-catalogue`. Re-scoped after the
approval against harness `main` `947d0e48`: the catalogue there carries the contract-name mapping
(`harness_operation`, `tools_operation`), so this story needs no tools dependency and no longer waits
for tools 0.2.0 (decision D6 of wave 3; the `dependency-blocker:tools-0-2-0-release` edge was
removed).

## Outcome

A caller may name an owned-surface tool by its `tool-contract/1` command name (`fs.read`, `fs.list`,
`fs.glob`, `code.write`, `code.edit`) when it invokes, describes or is resolved by metaharness, and
the run admits and scopes it exactly as it does the harness entry it maps to, so the names a caller
will one day supply (`story:caller-supplied-catalogue-on-every-kind`) can be the organization's
(ADR 0095 point 2). The mapping is harness's (`b10x-harness-catalogue`), not a second table here.

## Context

- tools 0.1.0: tag `a9bf6eb6d32ab0a569586afe3a2dfc5000c26af6` on `046dfb14c36773b557d76de0de92e04ce23a388d`;
  `crates/tool-contract` is `b10x-tool-contract`, Apache-2.0; `contracts/code-editor/v1alpha1/contract.yaml`
  with scenarios and fixtures (read at the tag, 2026-10-20). `b10x-tool-fs`, `-code-editor`,
  `-toolchain` are `LicenseRef-B10x-Proprietary` and are not adopted here
  (`decision-blocker:tools-implementation-licence`).
- Today `metaharness-tools` re-exports `Catalogue`, `LocalOperations`, `Operations`, `Verbs` and the
  three verb names from `b10x-harness-tools` at harness `798325f` (`src/lib.rs:47-50`,
  `Cargo.toml:21-22`) and maps a recorded call back to a neutral operation in `src/resolve.rs`.
- Invariant 10's rule applies to tools as to harness: taken by git revision, never by `path`.

## Acceptance

`resolve_verb` and `subjects_of_verb` accept each name `harness_catalogue::harness_operation` maps at
the harness revision unit 1 pins and resolve it to the same neutral `Operation` and subjects as its
harness entry, harness names still accepted, pinned by the existing resolve tests plus one resolve
and one subjects case per mapped name; `Server::call` performs a `tool_invoke` or `tool_describe`
naming a mapped contract name as its harness entry; metaharness adds no tools dependency and no name
table of its own; `task check` exits 0; `CHANGELOG.md` names the mapping's source.

## Out of scope

- Publishing contract names in the served catalogue (decision D6): the b10x run and the metaharness
  server share one catalogue (`crates/metaharness-tools/src/lib.rs:14-16`) and harness keeps its names
  model-facing (harness `crates/harness-catalogue/src/local.rs:32-33`), so renaming here would show the
  model two vocabularies.
- Publishing the contract schemas: harness's catalogue drops `expected_sha256`
  (`catalogue.rs:476-483`, `operations.rs:205,216`, `local.rs:520,543`); advertising it would promise
  a guard nobody runs. That needs a harness change.
- A dependency on the tools crates or `b10x-tool-contract`.

## Scope

Derived 2026-10-20 by `aep:story-scoper` against metaharness `3d697011` and harness `947d0e48`
(harness main); assumes unit 1 lands first. Cited unless marked inferred.

- **Primary surface:** `crates/metaharness-tools`.
- `crates/metaharness-tools/src/resolve.rs:1-25,29,78-91,105-117`; `src/server.rs:104-123`
  (`Server::call` performs a mapped contract name as its harness entry); `src/tests.rs:262-389` (one
  resolve and one subjects case per mapped name, 10 new); `CHANGELOG.md` `[Unreleased]`.
- **Symbols:** `resolve_verb`, `subjects_of_verb`, `Server::call`;
  `harness_catalogue::{harness_operation, tools_operation, operation_of, subjects_of}` (harness
  `crates/harness-catalogue/src/lib.rs:59,62`, `local.rs:118-143`: `file_read`→`fs.read`,
  `dir_list`→`fs.list`, `find`→`fs.glob`, `file_write`→`code.write`, `file_edit`→`code.edit`).
- inferred: `crates/metaharness-tools/src/lib.rs:12-20,50-52`.
- **Not touched:** `crates/metaharness-tools/Cargo.toml`, `Cargo.lock`, `crates/metaharness/src/run.rs`,
  fixtures.
- **Shared with unit 1 (sequential):** unit 1's path rewrite in `resolve.rs:29,113,135`,
  `server.rs:5`, `tests.rs:114`, `lib.rs`, `CHANGELOG.md`; with
  `story:caller-supplied-catalogue-on-every-kind`, `server.rs`.
- **Safety fact:** resolve and subjects must translate the same names, because `Run::out_of_scope`
  admits a call with no subjects (`crates/metaharness/src/run.rs:1025-1027`); routing both through
  `harness_operation` makes that structural. Contract ids are dotted and entry names are not, so they
  cannot collide (harness `local.rs:118-124`, `catalogue.rs:931-941`).
- **Estimate:** 10 new tests, 0-3 changed; fixtures 0.

## Ambiguities

- D2 (accepted by `b10x-chef-orchestrator`, 2026-10-20): contract name in, existing neutral operation
  out, harness names still accepted.
- D6 (coordinator, 2026-10-20, after the re-scope): the mapping is harness's; no tools dependency; the
  served catalogue keeps harness's names (Out of scope). Reversible: a later story may publish the
  contract names once harness does.
- Optional, not in the Acceptance: a dev-dependency on the tools revision harness pins, to check the
  mapping against the contract file (`tool_fs::CONTRACT_YAML`).
## Delivered (wave 3 of 2026-10-20, unit 3)

Commit `9f4d6a0`, merged `6ec311c`; two `aep:adversary` passes
(`review-result:adr0096-w3-contract-names-adversary-1`, `-2`). Contract names harness maps at 0.15.1
(`fs.read`, `fs.list`, `fs.glob`, `code.write`, `code.edit`) resolve with their subjects as their entry on
metaharness's own surface only (`VerbSurface`); a b10x record keeps them unknown; a contract-named invoke
carrying `expected_sha256` is refused as `precondition-failed` (amendment a32). Pass 2's pre-existing
finding (a b10x record drops the subject the loop reported) is `story:b10x-record-keeps-the-loops-subjects`,
its acceptance case committed and ignored. The end-to-end frame-scope case in `run_loop.rs` is the
implementor's, applied by the coordinator. Measured on the way and routed to harness: a harness-named
`file_write` accepts and ignores `expected_sha256` despite `additionalProperties: false`.
