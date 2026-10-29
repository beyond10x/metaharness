---
format: aep.planning-md/3
id: story:codex-catalogue-on-app-server
kind: story
status: implemented
title: A Codex run carries a caller's catalogue on the app-server route
summary: metaharness run codex --catalogue offers the catalogue's entries and nothing else, over the app-server protocol, instead of refusing.
owner: metaharness
refs:
- provider: atlas
  reference: adr-0098
relations:
- decomposes: epic:loom-agent-work-through-metaharness
- informed_by: review-result:adr0096-w5-catalogue-adversary-1
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-29T10:14:09Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-29T10:14:10Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-29T12:44:25Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"verification":1}}}
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

## Delivered (wave D193, 2026-10-29)

Unit `8ae85af`, merged `f21d117`; review fixes `26208b0`, merged `476b4fa` into `integration/d193`. Amendment
a66 and its "review corrections" section.

- `metaharness run codex --catalogue <FILE>` launches `codex app-server` with `environments: []` and the
  catalogue as `dynamicTools`. Each `item/tool/call` is decided by the frame and served by the catalogue
  server.
- The one live probe ran at Codex 0.160.0 on the ChatGPT-subscription login (D195), recorded in
  `docs/research/2026-10-29-codex-0.160.0-app-server-catalogue.md`. It exited 0, and `fs_read` was served
  and decided by the frame.
- **Deviation from the acceptance, decided by the coordinator.** The model request did not list only
  the catalogue. The model's code mode adds `exec`, `wait` and `request_user_input_async`, and no config
  key removes them; the catalogue was reached from inside `exec`. The security review found, from
  `rust-v0.160.0` source, that none of the three can perform an effect except through a catalogue tool,
  and that each nested call still reaches the frame (`review-result:d193-catalogue-security-1`). The
  route is therefore admitted. `session.started.offered_tools` names the three tools, and H8 states
  what they are. Two other offered tools, the clock tool and the multi-agent tools, are switched off by
  config keys added after the probe. That was checked against a fake model server only.
- Reviews: `review-result:d193-catalogue-adversary-1` (6 red) and `review-result:d193-catalogue-security-1`
  (2 red). All 8 cases are green after the fixes.
- Still open, in `story:codex-catalogue-live-follow-ups` (draft): the two switches on a real login,
  0.153.4 on a login, and the behaviour of `request_user_input_async`.
