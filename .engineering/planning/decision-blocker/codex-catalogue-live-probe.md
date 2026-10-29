---
format: aep.planning-md/3
id: decision-blocker:codex-catalogue-live-probe
kind: decision-blocker
status: cleared
title: Operator decides whether one live Codex probe is spent to finish the caller catalogue on Codex
owner: metaharness
relations:
- blocks: story:caller-supplied-catalogue-on-every-kind
- blocks: story:codex-catalogue-on-app-server
revision: 5
transitions:
- {from: "open", to: "cleared", at: "2026-10-29T10:14:08Z", actor: "human:timo", revision: 5}
---
## The question

`story:caller-supplied-catalogue-on-every-kind` is `active` and delivered except on Codex, where
`metaharness run codex --catalogue` refuses with `catalogue-not-carried`. The Codex arm is
`story:codex-catalogue-on-app-server` (draft). Its acceptance is a live probe at Codex 0.153.4 over the
app-server route, and that probe spends on a real model. A spending run is outside the delegated grant
that wave D193 ran under, so the decision went to the orchestrator. Recorded 2026-10-29.


## Options

| | option | what it does | what it costs |
|---|---|---|---|
| A | approve one live Codex probe and schedule `story:codex-catalogue-on-app-server` in the next wave | the Codex arm is built and driven; the parent story can close on its acceptance | one paid probe run, plus a second Codex launch route to maintain |
| B | close the parent story with Codex refused by name, and archive the Codex story | the store stops showing the story as active | the acceptance is weakened: Codex never carries a caller catalogue, and Loom's per-call governance stays session-level on Codex |
| C | leave both stories as they are | nothing changes | the parent stays `active` with no route to completion |

Recommendation: A.

## What it blocks

`story:caller-supplied-catalogue-on-every-kind` (completion) and `story:codex-catalogue-on-app-server`
(scheduling).

## Decision

A, decided by `b10x-chef-orchestrator` (D195, 2026-10-29): "A, decided by me (D195). ... It is not a
spending decision as long as the probe runs on the existing Codex login, ChatGPT subscription and OAuth,
not a pay-per-token API key: subscription quota is not spending ... If the probe would need a
pay-per-token API key, stop and report instead. Do story:codex-catalogue-on-app-server in this wave if it
fits the 4-unit cap, else first in the next metaharness wave. One live probe, recorded with its output."

The local Codex login is `auth_mode: chatgpt` with no API key, so the probe runs on the subscription.
`story:codex-catalogue-on-app-server` joins wave D193 (`approval-record:wave-d193`).
