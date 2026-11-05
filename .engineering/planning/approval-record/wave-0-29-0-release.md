---
format: aep.planning-md/3
id: approval-record:wave-0-29-0-release
kind: approval-record
status: approved
title: Release 0.29.0 on green PR CI approved by the orchestrator
summary: Gate, bot PR, merge, tag and release without asking again; baseline read first; drafts stay drafts
tags:
- non-interactive
relations:
- decides: release-plan:release-0-29-0
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-11-05T05:36:39Z", actor: "human:timo", revision: 2}
---
## Release 0.29.0 on green PR CI

`b10x-chef-orchestrator` approved this under the delegated grant. Run non-interactively, as a sub-agent of
the orchestrator.

Approved: after the U2 adversary pass closes, run the staged gate, open the bot PR, then on green PR CI
merge, tag `0.29.0` and release without asking again. The wave lead first checks the PR's checks itself.

Conditions:
- Before the push, the repository's Gates baseline is read against `origin/main`. When it was read it was
  `531f1dd`, 303 commits behind `origin/main`, and the scan was estimated at about 227 MiB of the 256 MiB
  limit. The orchestrator was told.
- A refusal from any delivery tool is a stop.
- The from-finding drafts `story:h11-not-ok-on-unprobed-memory` and `story:eval-cgroup-root-check-by-prefix`
  stay drafts.
