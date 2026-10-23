---
format: aep.planning-md/3
id: story:codex-0-160-0-token-cost
kind: story
status: implemented
title: Decide the Codex 0.160.0 pin from its token cost
summary: 'takt trial 2026-10-23: Codex 0.160.0 ran 0/2 and 0/2 at cap 85088, about +475 tokens a run over 0.153.4 with the model fixed; keep, fix or drop the pin'
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-23T18:23:46Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"approval":1,"verification":1}}}
- {from: "proposed", to: "active", at: "2026-10-23T18:23:46Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"approval":1,"verification":1}}}
- {from: "active", to: "implemented", at: "2026-10-23T18:26:15Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"approval":1,"verification":1}}}
---
## Problem

Reported by `b10x-chef-orchestrator` on 2026-10-23, from takt trial cycle 2026-10-23-54b8dd83 (takt 0.10.1,
metaharness 0.15.0, scenario 1, token cap 85088, the compared baseline's saved maximum):

| set | Codex | model | runs | pass | tokens |
|---|---|---|---|---|---|
| x101 | 0.153.4 | gpt-6-astra | 5 | 4/5 | 84292 / 84989 / 85120 (min/median/max) |
| x102 | 0.160.0 | gpt-6.1-sol (0.160.0's default) | 2 (early stop) | 0/2 | 85944, 85794 |
| x103 | 0.160.0 | gpt-6-astra (`--model`) | 2 (early stop) | 0/2 | 85462, 85468 |

With the model held fixed, 0.160.0 spends about +475 tokens a run over 0.153.4 (x103 against the x101 median),
per step about +200 (tests.write), +170 (implementation.edit) and +90 (change.handover). Every x102 and x103
transcript records `cli_version 0.160.0`. On this scenario the increase puts every run over the cap.

Not yet known: where the extra tokens come from (instructions, tool schemas, the reasoning budget, or the model
catalogue), and whether they are constant per call or grow with turns.

## Decision this story produces

One of:

- A. Keep the 0.160.0 pin unchanged, and record that consumers who cap tokens must raise the cap for 0.160.0.
- B. Keep the pin, and remove the extra tokens through a launch argument or config key, if a credential-free
  probe finds one that metaharness controls.
- C. Drop 0.160.0 from `metaharness_codex::PINNED_VERSIONS` (a breaking array-length change) until the cost is
  understood.

## Acceptance

- A credential-free, loopback-only probe compares the request bodies 0.153.4 and 0.160.0 send for the same frame
  and model, and attributes the difference by section (instructions, tools, other) in a dated research page.
- The decision (A, B or C) is recorded in the design document as an amendment, citing that page.
- No paid run: the trial numbers above are the cost evidence, and the probe needs no provider.
