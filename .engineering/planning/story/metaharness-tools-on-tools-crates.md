---
format: aep.planning-md/3
id: story:metaharness-tools-on-tools-crates
kind: story
status: draft
title: metaharness-tools builds on the tools crates
summary: The owned tool surface and the b10x catalogue come from tools instead of b10x-harness-tools at harness 798325f.
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
- depends_on: story:trace-ir-domain
- depends_on: story:metaharness-tools-adopts-tool-contract
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: crates/metaharness-b10x/Cargo.toml
- confidence: inferred
  path: crates/metaharness-b10x/src/seam.rs
- confidence: inferred
  path: crates/metaharness-cli/src/lib.rs
- confidence: inferred
  path: crates/metaharness-tools/Cargo.toml
- confidence: inferred
  path: crates/metaharness-tools/src/lib.rs
- confidence: inferred
  path: crates/metaharness-tools/src/resolve.rs
revision: 11
---
# Story: `metaharness-tools` builds on the `tools` crates

## Scheduling

Draft. `dependency-blocker:tools-first-release` cleared on 2026-10-20 (tools 0.1.0, tag `a9bf6eb6`
on `046dfb14`). `decision-blocker:tools-implementation-licence` cleared the same day with option A:
Timo decided (about 10:10, relayed by `adr-changes`) that the implementation crates moved from
harness (`b10x-tool-fs`, `b10x-tool-code-editor`, `b10x-tool-toolchain`) become Apache-2.0 in tools
0.2.0. This story therefore pins tools 0.2.0 and is held by `dependency-blocker:tools-0-2-0-release`
until that tag exists; it goes into wave 3 beside `story:metaharness-tools-adopts-tool-contract`
(`depends_on`), both at the same tag. Not in wave 2.

## Outcome

The owned tool surface metaharness serves over MCP, and the catalogue the b10x adapter derives its
`rendering()` from, come from `tools` (ADR 0095, ADR 0097 point 3), so the one path-containment check
that "must not exist twice" (`crates/metaharness-tools/Cargo.toml:13-17`) lives in the layer that owns
tool contracts.

## Context

- `metaharness-tools` re-exports `Catalogue`, `DESCRIBE_VERB`, `INVOKE_VERB`, `LocalOperations`,
  `Operations`, `SEARCH_VERB`, `Verbs` from `b10x_harness_tools` (`src/lib.rs:47-50`) and resolves a
  recorded call back to a neutral operation (`resolve.rs`: `file.write` and siblings).
- `metaharness-b10x` takes `b10x-harness-tools` so its rendering is derived from the published
  catalogue rather than written twice (`Cargo.toml:14-19`).
- ADR 0095 point 5: harness, metaharness and takt adopt the first toolsets (`fs`, `code-editor`);
  "implements a toolset version" means passing that version's conformance suite (point 3).

## Acceptance

`task check` exits 0 with no `b10x-harness-tools` or `b10x-harness-wire` dependency in
`crates/metaharness-tools/Cargo.toml` or `crates/metaharness-b10x/Cargo.toml`; the three verbs are
served from the `tools` crates at a git revision reachable from `tools` main (invariant 10's rule,
applied to `tools`); `resolve_verb` maps the `tool-contract/1` command names onto the same neutral
operations the recorded vectors already assert; the b10x adapter's `rendering()` still derives from
the catalogue; `CHANGELOG.md` names the `tools` release.

## Out of scope

- Writing the tool contracts (`tools`).
- The b10x adapter's loop pin (`story:b10x-adapter-on-harness-over-llm`).

## Scope

- cited: `crates/metaharness-tools/Cargo.toml`, `crates/metaharness-tools/src/lib.rs`,
  `crates/metaharness-tools/src/resolve.rs`, `crates/metaharness-b10x/Cargo.toml`, `Cargo.lock`,
  `CHANGELOG.md`.
- inferred: `crates/metaharness-b10x/src/seam.rs` (where `rendering()` reads the catalogue),
  `crates/metaharness-cli/src/lib.rs:300` (server construction).

## External prerequisite

`tools` release holding the former `harness-tools` code under `tool-contract/1` (ADR 0097 point 3,
ADR 0095 T1). Cross-store edges are not admitted by `aep` (`/` in the id); the blocker carries the
reference in prose.
