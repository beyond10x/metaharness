---
format: aep.planning-md/3
id: release-plan:release-0-24-0
kind: release-plan
status: draft
title: 'Publish Metaharness 0.24.0: a caller''s catalogue on Codex, ESS 0.76.0'
summary: codex app-server catalogue route, review corrections, ESS 0.76.0
relations:
- supersedes: release-plan:release-0-23-1
revision: 1
---
## Why this release

Wave D193 (`approval-record:wave-d193`):

- `story:codex-catalogue-on-app-server`: `metaharness run codex --catalogue` carries a caller's catalogue
  over `codex app-server` (amendment a66). It closes `story:caller-supplied-catalogue-on-every-kind`.
- `task:ess-pin-0-76-0`: ESS 0.76.0 is the specification toolchain.
- `story:independent-documentation`: re-checked, found done and closed, with no code.

It supersedes `release-plan:release-0-23-1`, which was cut before D195 added the Codex story and was
never tagged.

## Version

`0.24.0`. `Refusal` gains two variants and `LaunchContext` and `LaunchPlan` gain fields, so both break
an exhaustive match and a struct literal. A Codex catalogue run starts where it used to be refused, and a
`custom_tool_call` now carries its script. Loom's session executor takes `metaharness` and
`metaharness-b10x` at `0.23.0`. Built and tested against this tree through an uncommitted path patch, it
needs no change.

## Procedure

As in `release-plan:release-0-23-1`: the release commit and the full gate on `integration/d193`; PR 71
through the bot; after CI, the bot fast-forwards `main` and pushes the annotated tag `0.24.0`; artifacts
are checked and scanned, the bot publishes the release, then `verify-release.sh metaharness 0.24.0
<announcement>`.
