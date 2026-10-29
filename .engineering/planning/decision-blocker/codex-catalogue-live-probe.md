---
format: aep.planning-md/3
id: decision-blocker:codex-catalogue-live-probe
kind: decision-blocker
status: open
title: Operator decides whether one live Codex probe is spent to finish the caller catalogue on Codex
owner: metaharness
relations:
- blocks: story:caller-supplied-catalogue-on-every-kind
- blocks: story:codex-catalogue-on-app-server
revision: 2
---
## The question

`story:caller-supplied-catalogue-on-every-kind` is `active` and delivered except on Codex, where
`metaharness run codex --catalogue` refuses with `catalogue-not-carried`. The Codex arm is
`story:codex-catalogue-on-app-server` (draft). Its acceptance is a live probe at Codex 0.153.4 over the
app-server route, and that probe spends on a real model. A spending run is outside the delegated grant
that wave D193 ran under, so the decision belongs to the operator. Recorded 2026-10-29.

Decider: operator

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
