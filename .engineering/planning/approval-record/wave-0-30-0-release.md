---
format: aep.planning-md/3
id: approval-record:wave-0-30-0-release
kind: approval-record
status: approved
title: Release 0.30.0 on green PR CI approved by the orchestrator
summary: Gate, bot PR, merge, tag and release without asking again; U1 only; drafts stay drafts
tags:
- non-interactive
relations:
- decides: release-plan:release-0-30-0
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-11-05T11:41:01Z", actor: "human:timo", revision: 2}
---
## Release 0.30.0 on green PR CI

`b10x-chef-orchestrator` approved this under the delegated grant. It ran non-interactively, as a sub-agent of
the orchestrator.

**Approved:** after the U1 adversary pass is green, the wave lead:
1. runs the staged gate G1-G5 on the wave's terms;
2. opens one bot PR with auto-merge;
3. on green PR CI, tags `0.30.0` and releases;
4. verifies the release and hands over the announcement path.

The wave lead checks the PR's checks itself before merging. The CHANGELOG and the announcement name U1's
changes only.

**Conditions:**
- A refusal from any delivery tool is a stop.
- A red check is reported with its job and log lines, with no rerun.
- The from-finding drafts stay drafts.
