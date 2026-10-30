---
format: aep.planning-md/3
id: release-plan:release-0-26-0
kind: release-plan
status: draft
title: 'Publish Metaharness 0.26.0: the prompt cache holds between sessions'
summary: trial finding C5, wave 2026-10-30-c5
relations:
- supersedes: release-plan:release-0-25-0
revision: 1
---
## Why this release

Wave `wave/2026-10-30-c5` (approval D290), trial finding C5: the provider's prompt cache did not
hold between two sessions of one case.

- `story:claude-child-runs-without-auto-memory`: a Claude child runs with auto-memory off, so its
  system prompt no longer names the per-run config directory (amendment a70).
- `story:claude-prompt-cache-ttl-declared`: a caller declares the prompt-cache class with
  `--prompt-cache-ttl`, and usage splits cache writes by class (amendment a71).
- `story:scratch-root-stable-by-declared-name`: a caller names the scratch root with
  `--scratch-name`, so derived paths are stable across sessions (amendment a72).

## Version

`0.26.0`. `RunSpec`, `Usage` and `Refusal` gain fields and variants, which breaks a struct literal
and an exhaustive match; the stream stays `metaharness.event/1` under D3.

## Procedure

As in `release-plan:release-0-24-1`: the release commit and the gate on `wave/2026-10-30-c5`; one
PR through the bot; after CI, the bot fast-forwards `main` and pushes the annotated tag `0.26.0`;
artifacts are checked and scanned, then the bot publishes the release.
