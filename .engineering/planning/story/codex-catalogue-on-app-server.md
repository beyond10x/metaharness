---
format: aep.planning-md/3
id: story:codex-catalogue-on-app-server
kind: story
status: draft
title: A Codex run carries a caller's catalogue on the app-server route
summary: metaharness run codex --catalogue offers the catalogue's entries and nothing else, over the app-server protocol, instead of refusing.
owner: metaharness
refs:
- provider: atlas
  reference: adr-0098
relations:
- decomposes: epic:loom-agent-work-through-metaharness
- informed_by: review-result:adr0096-w5-catalogue-adversary-1
revision: 1
---
# Story: a Codex run carries a caller's catalogue on the app-server route

## Scheduling

Draft, filed 2026-10-21 at the close of wave 5 from `review-result:adr0096-w5-catalogue-adversary-1`
(finding 5). Implementation needs the orchestrator's word: it adds a second Codex launch route and a live
probe that spends.

## Outcome

`metaharness run codex --catalogue <FILE>` makes a `tool-contract/1` document's entries the session's whole
tool set, as `story:caller-supplied-catalogue-on-every-kind` already does for Claude, instead of refusing
with `catalogue-not-carried`.

## Context

- Today Codex refuses a catalogue by name (`catalogue-not-carried`); a40 § 2
  (`docs/design/metaharness-protocol-v0.1.md:3345-3397`) records why and cites the switches below. None has
  been driven here, and a rollout's record lists no tools.
- Every citation is Codex source at tag `rust-v0.153.4` (commit `3d2ee51`), under `codex-rs/`:
  - `initialize` takes `experimentalApi`: `app-server-protocol/src/protocol/v1.rs:46-49`.
  - `thread/start` takes `environments` and `dynamicTools`, both experimental:
    `app-server-protocol/src/protocol/v2/thread.rs:129-137` (environments, flag at `:135`) and `:138-144`
    (dynamicTools, flag at `:138`).
  - A dynamic tool call reaches the client as `item/tool/call`: `app-server-protocol/src/protocol/common.rs:1721`.
  - `web_search` is a config key: `config/src/config_toml.rs:435`, read at `core/src/tools/spec_plan.rs:1416`.
  - Image generation is a stable feature, on by default: `features/src/lib.rs:1418-1423`, checked at
    `core/src/tools/spec_plan.rs:698-703`.
  - Multi-agent (`Feature::Collab`) is a stable feature, on by default: `features/src/lib.rs:1220-1225`, read
    at `core/src/config/mod.rs:1545`.
- The route: `initialize` with `experimentalApi`; `thread/start` with `environments: []` and `dynamicTools`
  set to the catalogue's entries; config that turns off `web_search`, image generation and multi-agent.
  Unverified: whether built-in shell and file tools are absent with `environments: []` at the pin.

## Acceptance

- expected_before: today `metaharness run codex --catalogue <FILE>` exits 2 with `catalogue-not-carried` and
  no Codex session starts. After: a live probe at Codex 0.153.4 starts a session over the app-server route
  whose model request lists exactly the catalogue's tools (no built-in, no web search, no image generation,
  no multi-agent tool), and a dynamic tool call is served by the catalogue's server.
- The probe's record is cited in the amendment that replaces a40 § 2's refusal.

## Scope

Inferred: `crates/metaharness-codex/src/launch.rs` (a second launch route), `crates/metaharness/src/spawn_codex.rs`
(the runner for the app-server protocol), `crates/metaharness-codex/src/vectors.rs`,
`docs/design/metaharness-protocol-v0.1.md` (amendment after a40).
