---
format: aep.planning-md/3
id: story:metaharness-tools-on-tools-crates
kind: story
status: active
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
- depends_on: story:b10x-adapter-on-harness-over-llm
revision: 47
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T09:05:39Z", actor: "human:timo", revision: 33, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-20T09:05:40Z", actor: "human:timo", revision: 34, decided_on: {"recorded":{"review_outcome":2}}}
---
# Story: `metaharness-tools` builds on the `tools` crates

## Scheduling

In wave 3 of 2026-10-20 as unit 4, then superseded by unit 1 (decision D5, after the re-scope against
harness `main` `947d0e48`): harness 0.15.0 deletes `b10x-harness-tools`, so
`story:b10x-adapter-on-harness-over-llm` must replace it with `b10x-harness-catalogue` in both
manifests, and that catalogue's `LocalOperations` runs `fs` and `code-editor` through the tools crates
(harness `crates/harness-catalogue/src/local.rs:243-257,499-557`). No unit is dispatched for this
story; the coordinator verifies its Acceptance on the integration branch after unit 1 merges and
records the result.

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

Closing evidence, read after unit 1 merges: the file operations metaharness serves run in the tools
crates through `b10x-harness-catalogue` (the containment check runs in tools); `git grep -n
'b10x.harness.tools'` outside `CHANGELOG.md` and `.engineering/` returns 0 hits; `Cargo.lock` holds
one harness revision and one tools revision, with `b10x-tool-fs` and `b10x-tool-code-editor` from it;
the containment refusal scenario (`crates/metaharness-aep/src/drive/ess_conformance.rs:513-516`)
passes in the whole gate.

## Out of scope

- Contract names (`story:metaharness-tools-adopts-tool-contract`).
- Contract schemas: harness's catalogue drops `expected_sha256` (`catalogue.rs:476-483`,
  `operations.rs:205,216`); a harness change first.
- A direct tools dependency in metaharness: which tools tag metaharness links is decided by the tools
  tag harness 0.15.0 pins (0.1.0 at `947d0e48`, `Cargo.toml:37-40`); a direct pin would add a second
  tools revision.

## Scope

Derived 2026-10-20 by `aep:story-scoper` against metaharness `3d697011` and harness `947d0e48`.

- **Primary surface:** none after unit 1. The doc lines naming `b10x-harness-tools` moved to unit 1's
  scope.
- **Safety fact:** the containment text in tools (`046dfb14` `tool-fs/src/error.rs:51`) is the text
  `ess_conformance.rs:513-516` expects.

## Ambiguities

- D3 (changed by the approval): the verb/catalogue/search layer comes from `b10x-harness-catalogue`.
- D5 (coordinator, 2026-10-20): this story's code lands in unit 1; it closes on the evidence above.
  The tools crates harness pins at `947d0e48` are tools 0.1.0, `LicenseRef-B10x-Proprietary`
  (tools `crates/tool-fs/Cargo.toml:4`), the licence class of the harness crates metaharness links
  today; they become Apache-2.0 when harness moves to tools 0.2.0.
