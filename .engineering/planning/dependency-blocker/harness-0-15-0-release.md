---
format: aep.planning-md/3
id: dependency-blocker:harness-0-15-0-release
kind: dependency-blocker
status: cleared
title: 'harness 0.15.0 (tools-1: b10x-harness-catalogue over the tools crates) is not tagged'
relations:
- blocks: story:b10x-adapter-on-harness-over-llm
withholds: test_result
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-20T10:56:20Z", actor: "human:timo", revision: 3}
---
# harness 0.15.0 (tools-1: b10x-harness-catalogue over the tools crates) is not tagged
## Cleared (2026-10-20)

harness 0.15.0 is tagged: tag object `1589aa1af07ef426758dd83b6f2efddae373b760` (tagger
`b10x-bot[bot]`) on commit `dd5c2a66218a5f2555c4a9fdab6fdfac8515cc6d`, harness `main` (read with
`gh api repos/beyond10x/harness/git/refs/tags/0.15.0` and `git/tags/1589aa1a…`). Relayed by
`b10x-chef-orchestrator`, which checked it: made by the bot; tools pinned at 0.2.0, llm at 0.8.0; the
public llm crate is `b10x-harness-llm`; CHANGELOG `[0.15.0]` names one model-facing change, a `find`
capped by `max_results` now answers `depth_bound_reached: false`.
