---
format: aep.planning-md/3
id: dependency-blocker:harness-llm-release
kind: dependency-blocker
status: open
title: No harness release builds on llm yet
owner: metaharness
refs:
- provider: atlas
  reference: adr-0097
relations:
- blocks: story:b10x-adapter-on-harness-over-llm
withholds: test_result
revision: 1
---
## Upstream requirement

A harness release (beyond10x/harness) that builds on llm 0.7.0 with its four wire and credential
crates removed (Atlas ADR 0097 point 2), tagged and reachable from harness main (this repository's
invariant 10). Recorded 2026-10-20.

## Why it is open

ADR 0097 Context: harness 0.13.3 "has no llm dependency"; the move exists only on branch
`spike/builds-on-llm-0.1.7` (`8989d138`, not merged), to be rebased onto llm 0.7.0. The current pin is
0.13.3 at `798325f03cf5a18df8fadb346d31b314826136ec` (`crates/metaharness-b10x/src/lib.rs:80,87`).

## What it withholds

A green `task check` and a re-recorded golden loop against the new release (`test_result`) for
`story:b10x-adapter-on-harness-over-llm`.

## Clearing condition

Record here the harness tag, its revision, and the llm version it builds on, then
`aep plan artifact move dependency-blocker:harness-llm-release --to cleared`. Cross-store edges are
not admitted by `aep`; the reference stays in prose.
