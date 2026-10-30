---
format: aep.planning-md/3
id: release-plan:release-0-25-0
kind: release-plan
status: draft
title: 'Publish Metaharness 0.25.0: semantic operations on every trace-ir tool call'
summary: context-lens semantic extractor, unit 1
relations:
- supersedes: release-plan:release-0-24-1
revision: 1
---
## Why this release

Wave `wave/context-lens-semantic`:

- `story:trace-ir-semantic-operations`: every `tool_call` node of a projected `trace-ir/1`
  document carries `semantic_operations`, extracted by the context-lens (MIT) semantic extractor
  in the new crate `metaharness-semantic`, with `executable`, `wrappers` and `script` added on top
  (amendment a68). `docs/design/context-lens-parity.md` lists the units still open.

## Version

`0.25.0`. `metaharness_protocol::MetaharnessBlock` gains `semantic_rule`, which breaks a struct
literal; the document format stays `trace-ir/1` under D3.

## Procedure

As in `release-plan:release-0-24-1`: the release commit and the full gate on
`wave/context-lens-semantic`; one PR through the bot; after CI, the bot fast-forwards `main` and
pushes the annotated tag `0.25.0`; artifacts are checked and scanned, then the bot publishes the
release.
