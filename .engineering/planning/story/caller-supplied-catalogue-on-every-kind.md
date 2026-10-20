---
format: aep.planning-md/3
id: story:caller-supplied-catalogue-on-every-kind
kind: story
status: draft
title: A caller supplies the tool catalogue on every kind
summary: Loom hands metaharness run the catalogue a session may see; the model is offered that and nothing else, on Claude Code, Codex and harness, attested in session.started.
owner: metaharness
tags:
- governance
refs:
- provider: atlas
  reference: adr-0098
relations:
- decomposes: epic:loom-agent-work-through-metaharness
- depends_on: story:trace-ir-domain
- depends_on: story:strict-hermetic-with-embedder-scratch-cwd
- depends_on: story:metaharness-tools-adopts-tool-contract
scope:
- confidence: inferred
  path: crates/metaharness-b10x/src/launch.rs
- confidence: inferred
  path: crates/metaharness-claude/src/launch.rs
- confidence: inferred
  path: crates/metaharness-cli/src/lib.rs
- confidence: inferred
  path: crates/metaharness-codex/src/launch.rs
- confidence: inferred
  path: crates/metaharness-protocol/src/spec.rs
- confidence: inferred
  path: crates/metaharness-tools/src/lib.rs
- confidence: inferred
  path: crates/metaharness-tools/src/server.rs
- confidence: inferred
  path: crates/metaharness/src/run.rs
- confidence: inferred
  path: docs/design/metaharness-protocol-v0.1.md
revision: 12
---
# Story: a caller supplies the tool catalogue on every kind

## Scheduling

Draft. Not in the wave of 2026-10-20 (owner's instruction). Gap 1 of Atlas ADR 0098 Consequences.

## Outcome

Loom's session executor hands `metaharness run <kind>` the catalogue the session may see (tool names,
input schemas, descriptions, in the neutral operation vocabulary), and the model is offered that
catalogue and nothing else, on Claude Code, Codex and the in-house harness alike. `session.started.
offered_tools` records what was offered, so the claim is read from the run's record (invariant 3).

## Context

- Today the owned surface is fixed: `metaharness run --tool-surface owned` serves
  `metaharness_tools::Catalogue` built over `LocalOperations` (`crates/metaharness-cli/src/lib.rs:300`),
  i.e. the file, search and bounded `run` operations metaharness implements itself
  (`crates/metaharness-tools/src/lib.rs:1-45`). The only caller-side choice is `--allow-program`
  (`spec.rs:258-262`).
- The three verbs (`tool_search`, `describe`, `invoke`) are already the same on every harness
  (`metaharness-tools/src/lib.rs`, "Three verbs, not six tools"); a caller-supplied catalogue sits
  behind them, so the vendor-facing surface need not change.
- `Kind::B10x` publishes its own toolset ("its published toolset already is its policy",
  `spec.rs:24-30,498`); `run.rs:893` treats `Owned` and `B10x` alike for subject resolution.
- Unverified at this draft: whether Codex is launched with the owned surface today (the CLI doc at
  `lib.rs:67` names only the `--tools ""` launch, which is Claude Code's flag). The story establishes
  it per kind and labels what a kind cannot do as a refusal by name (design § 6.1).

## Acceptance

`metaharness run claude --catalogue <file>` and `metaharness run codex --catalogue <file>` each
launch a session whose `session.started.offered_tools` equals the catalogue's names and whose
`tool.requested` events name only catalogue entries, pinned by a recorded vector per kind (adapter
contract CT-4, `docs/design/adapter-contract-v0.1.md`); only `b10x` may refuse by name before
spawn, because its published toolset is its policy, and if Codex cannot be launched with the owned
surface that is a finding in the report, not a pass.

## Order

Depends on `story:metaharness-tools-on-tools-crates`: the catalogue a caller supplies is in the
`tool-contract/1` format `tools` publishes (ADR 0095 point 2), and both stories change
`crates/metaharness-tools/src/lib.rs` and the server construction at `crates/metaharness-cli/src/lib.rs:300`.
No interim local format is built: the edge behind the `tools` release is the chosen cost.
`story:decision-returns-caller-produced-result`, `story:continuity-across-executor-calls` and
`story:call-seam-for-harness` follow this story on `run.rs`, `spec.rs`, the launch files and the
protocol design.

## Out of scope

- Performing the call on the caller's behalf (`story:decision-returns-caller-produced-result`).
- Changing the catalogue mid-session (`story:catalogue-changes-within-a-session`).
- The tool contracts themselves: `tools` owns them (ADR 0095); this story carries names and schemas.

## Scope

- inferred: `crates/metaharness-protocol/src/spec.rs` (a catalogue field on `RunSpec`),
  `crates/metaharness-tools/src/server.rs` and `src/lib.rs`, `crates/metaharness-cli/src/lib.rs`,
  `crates/metaharness/src/run.rs`, `crates/metaharness-claude/src/launch.rs`,
  `crates/metaharness-codex/src/launch.rs`, `crates/metaharness-b10x/src/launch.rs`,
  `docs/design/metaharness-protocol-v0.1.md` (amendment).

## Ambiguities

- `inferable` — the catalogue file format is the `tool-contract/1` profile from `tools` (ADR 0095
  point 2); settled by the edge above.
