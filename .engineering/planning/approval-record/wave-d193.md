---
format: aep.planning-md/3
id: approval-record:wave-d193
kind: approval-record
status: draft
title: 'Wave D193: ESS 0.76.0, story re-checks, Codex catalogue on the app-server route'
tags:
- wave
relations:
- decides: task:ess-pin-0-76-0
- decides: story:codex-catalogue-on-app-server
revision: 2
---
## Wave D193

Approved by `b10x-chef-orchestrator` under the delegated grant (class C: a wave, PR, merge and release
through the repository's bot route; a dependency upgrade).

- `task:ess-pin-0-76-0`: ESS 0.76.0 ride-along.
- `story:caller-supplied-catalogue-on-every-kind`: re-checked; blocked on its Codex arm.
- `story:independent-documentation`: re-checked; found done, moved to implemented.
- `story:codex-catalogue-on-app-server`: added by D195 (2026-10-29), which closes
  `decision-blocker:codex-catalogue-live-probe` with option A. Quote: "Do story:codex-catalogue-on-app-server
  in this wave if it fits the 4-unit cap ... One live probe, recorded with its output." The probe runs on the
  existing ChatGPT-subscription Codex login, never on a pay-per-token API key.
