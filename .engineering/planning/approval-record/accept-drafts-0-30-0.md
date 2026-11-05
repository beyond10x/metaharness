---
format: aep.planning-md/3
id: approval-record:accept-drafts-0-30-0
kind: approval-record
status: approved
title: Three stories accepted for the 0.30.0 wave
summary: H11 unprobed memory (verdict route), eval cgroup root by prefix, substrate-host 0.20 confinement-only
tags:
- non-interactive
relations:
- decides: story:h11-not-ok-on-unprobed-memory
- decides: story:eval-cgroup-root-check-by-prefix
- decides: story:substrate-host-0-20-confinement-only
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-11-05T11:09:21Z", actor: "human:timo", revision: 2}
---
## Three stories accepted for the 0.30.0 wave (2026-11-05)

`b10x-chef-orchestrator` accepted these under the delegated grant. It ran non-interactively, as a sub-agent
of the orchestrator.

Accepted:
- `story:h11-not-ok-on-unprobed-memory`, by the verdict route: a memory file found in a surface the canary
  never probed makes H11 `unk`, naming the file and the surface. Re-measuring the canary is not part of this.
- `story:eval-cgroup-root-check-by-prefix`
- `story:substrate-host-0-20-confinement-only`, filed with this record.

All three move to active. This record accepts stories. It does not approve a wave; `approval-record:wave-0-30-0`
does.
