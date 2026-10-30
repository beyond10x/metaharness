---
format: aep.planning-md/3
id: story:trace-ir-normalized-tools
kind: story
status: implemented
title: What a tool call ran is its normalized operations, never its raw command
summary: normalized tool per operation; summary counts programs, not carriers
relations:
- decomposes: epic:session-format
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T03:15:18Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-30T03:15:19Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-30T03:23:00Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"ess_conformance_coverage_v1":1}}}
---
## Why

Operator, 2026-10-30: trace-ir normalizes every harness's sessions so that what a session did can
be understood, including which tools it ran, and a shell call must not be left raw. After
`story:trace-ir-semantic-operations`, a summary still named `Bash` and `exec_command` as the tool.

## What

Each semantic operation names its normalized `tool`; the summary lists the tools each call ran and
counts tools and semantic kinds over the document, read off the operations and never off the raw
input (amendment a69).

## Acceptance

The conformance scenarios of `metaharness.trace.ReadNormalizedTools` pass against the production
projection and summary:

- `metaharness.trace.ReadNormalizedTools/outcome/shell-programs`
- `metaharness.trace.ReadNormalizedTools/outcome/wrapped-shell`
- `metaharness.trace.ReadNormalizedTools/outcome/codex-exec`
- `metaharness.trace.ReadNormalizedTools/outcome/native-call`
- `metaharness.trace.ReadNormalizedTools/outcome/unreadable-shell`

and the five `metaharness.trace.SummarizeRecord` outcomes carry `tools` and `operation_kinds`.
