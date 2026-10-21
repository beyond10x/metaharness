---
format: aep.planning-md/3
id: story:caller-supplied-catalogue-on-every-kind
kind: story
status: active
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
- depends_on: story:codex-catalogue-on-app-server
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
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T12:00:26Z", actor: "human:timo", revision: 14, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "proposed", to: "active", at: "2026-10-21T12:00:26Z", actor: "human:timo", revision: 15, decided_on: {"recorded":{"review_outcome":4}}}
---
# Story: a caller supplies the tool catalogue on every kind

## Scheduling

Wave 5 unit 4 (`approval-record:adr-0096-wave-5`), gap 1 of Atlas ADR 0098 Consequences; its dependencies
(`story:metaharness-tools-on-tools-crates`, `story:metaharness-tools-adopts-tool-contract`,
`story:trace-ir-domain`, `story:strict-hermetic-with-embedder-scratch-cwd`) are implemented. Decided by the
coordinator (D2): the per-kind vectors are synthetic; a live Claude or Codex launch with a catalogue stays
labelled unverified, because it needs a paid run (no paid run in this wave). This unit is the trigger for
release 0.12.0, which `b10x-chef-orchestrator` approves or holds separately on the release PR.

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

## Delivered

**Delivered in part; this story stays `active` because the Codex arm is open.** The Codex arm is
`story:codex-catalogue-on-app-server` (draft), on which this story now depends.

Commit `1bdc5b2`, merged `875fc71` into `integration/adr-0096-wave-5`; two `aep:adversary` passes
(`review-result:adr0096-w5-catalogue-adversary-1`, NEEDS-CHANGE; `review-result:adr0096-w5-catalogue-adversary-2`,
NEEDS-CHANGE, no blocker), two fix rounds, the second also taking four findings of the independent review
(`review-result:adr0096-w5-independent-review`: s7-1, s7-2, s9-1, s10-5); fix round 2 gate 94 test
binaries, 1112 passed, 0 failed, 49 ignored, conformance 103/103. Amendment a40.

- **Claude: carried.** `run claude --catalogue <FILE>` and `mcp-serve --catalogue <FILE>` make a
  `tool-contract/1` document's entries the run's whole tool set in place of the three verbs; the launch
  removes the built-in tools and every ambient MCP server; `session.started.offered_tools` lists the
  catalogue's names. The vector is synthetic (decision D2); a live launch with a catalogue is unverified.
- **b10x: refused by name** (`catalogue-not-carried`), which the story allows for this kind.
- **Codex: refused by name** (`catalogue-not-carried`). a40 § 2 cites, at `rust-v0.153.4`, the switches
  and the app-server fields a carrying launch would use; none has been driven here, and a rollout's record
  lists no tools. The acceptance is not met for Codex.
- Refusals before the spawn: `catalogue-needs-owned-surface`, `catalogue-with-allow-program` (decided by
  the coordinator after pass 1: an ignored flag is refused, a27), `catalogue-unreadable` (not a regular
  file, past 4 MiB), `catalogue-malformed`, and `catalogue-copy-differs` after the scratch write.
- The scratch copy escapes the 37 scalar values the YAML reader refuses or folds as a line break and is
  read back before the spawn; one leading byte order mark is stripped. The server checks each call's
  arguments against the entry's declared input fields and performs nothing on a mismatch.
- Adversary cases kept in `crates/metaharness-tools/tests/adversary_w5u4.rs` and
  `crates/metaharness/tests/adversary_w5u4.rs`. Vector `c1-caller-catalogue` moves checked 29 → 30,
  `c1-caller-catalogue-refusal` 22 → 23; every other vector fixture is byte-identical.

Whole gate at the wave-6 head: task check, one step at a time, on `integration/adr-0096-wave-6` at
`5ddebbd`: 6 steps exit 0, 96 test binaries, 1128 passed, 0 failed, 49 ignored, conformance 103/103 on ESS
0.67.0.

Deviations and what stays open: argument value types are not checked (a string where a number belongs
reaches the harness verb, which refuses it); a FIFO swapped in between the stat and the open can still
block the open. Both, with the review's s8-3, s8-4 and s7-5, are `story:catalogue-road-leftovers`
(draft). One document is one tool-contract family, so a session cannot get two families
(`story:several-tool-contract-families-per-session`, draft, unscheduled).
