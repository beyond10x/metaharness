---
format: aep.planning-md/3
id: story:metaharness-tools-adopts-tool-contract
kind: story
status: draft
title: metaharness-tools adopts b10x-tool-contract from tools 0.2.0
summary: Depend on the Apache-2.0 contract crate at the tools 0.2.0 tag by git rev; resolve_verb and the served catalogue speak the contract's names; the implementation crates follow in story:metaharness-tools-on-tools-crates at the same tag.
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
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: crates/metaharness-tools/Cargo.toml
- confidence: inferred
  path: crates/metaharness-tools/src/lib.rs
- confidence: inferred
  path: crates/metaharness-tools/src/resolve.rs
- confidence: inferred
  path: crates/metaharness-tools/src/server.rs
- confidence: inferred
  path: crates/metaharness-tools/src/tests.rs
revision: 10
---
# Story: `metaharness-tools` adopts `b10x-tool-contract` from tools 0.1.0

## Scheduling

Draft, filed 2026-10-20 when tools 0.1.0 was released; candidate for the wave after wave 2
(`adr-changes`, 2026-10-20). Blocked by `dependency-blocker:tools-0-2-0-release`: at tools 0.1.0 the
contract YAMLs are embedded only by the proprietary implementation crates, so `b10x-tool-contract`
alone carries no command names or schemas (`adr-changes` relaying tools, 2026-10-20); tools runs
`story:contract-documents-without-an-implementation` and releases 0.2.0 after it. This story pins
tools 0.2.0. `b10x-tool-contract` is Apache-2.0, as metaharness is.

## Outcome

The tool catalogue metaharness serves over MCP and resolves back from a run is described by the
`tool-contract/1` profile that `tools` publishes (ADR 0095 point 2), through the `b10x-tool-contract`
crate at the tools 0.1.0 tag, so the names and schemas a caller will one day supply
(`story:caller-supplied-catalogue-on-every-kind`) are the organization's and not this repository's
spelling of them. The implementations stay where they are until the licence decision.

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

`crates/metaharness-tools/Cargo.toml` depends on `b10x-tool-contract` at the git rev of the tools
0.2.0 tag (recorded in `dependency-blocker:tools-0-2-0-release` when it clears) and on no other
tools crate; `resolve_verb`
maps each operation it already recognises onto the contract's command name where the contract
declares one and says `Unknown` where it does not, pinned by the existing resolve tests plus one per
contract command; the catalogue the server publishes carries the contract's names and input schemas
for the commands the local operations implement; `task check` exits 0; `CHANGELOG.md` names the tools
release and the licence boundary.

## Out of scope

- Depending on `b10x-tool-fs`, `b10x-tool-code-editor` or `b10x-tool-toolchain` (blocked by
  `decision-blocker:tools-implementation-licence`; `story:metaharness-tools-on-tools-crates`).
- Removing `b10x-harness-tools` from either `Cargo.toml` (same story).

## Scope

- inferred: `crates/metaharness-tools/Cargo.toml`, `crates/metaharness-tools/src/lib.rs`,
  `crates/metaharness-tools/src/resolve.rs`, `crates/metaharness-tools/src/server.rs`,
  `crates/metaharness-tools/src/tests.rs`, `Cargo.lock`, `CHANGELOG.md`.
- Shares `crates/metaharness-tools/src/lib.rs` and `Cargo.lock` with `story:metaharness-tools-on-tools-crates`
  (blocked) and `crates/metaharness-tools/src/server.rs` with `story:caller-supplied-catalogue-on-every-kind`,
  which depends on this story.
