---
format: aep.planning-md/3
id: dependency-blocker:adapter-probe-budget
kind: dependency-blocker
status: open
title: Current vendor qualification awaits a bounded live-probe budget
relations:
- blocks: story:current-adapter-compatibility
- blocks: story:codex-terminal-failure
- blocks: story:codex-final-answer
- blocks: story:codex-observed-model
- blocks: story:codex-tool-outcomes
withholds: test_result
revision: 2
---
## Pending decision

The approved wave authorizes offline implementation. The separate question about a shared live-probe budget has not been answered; continuation does not imply paid-model authority. No paid requests were launched for this wave.

## Concrete remaining work

Issue #11 still needs its bounded native unsupported-model failure observation. Issue #15 requires current installed Claude 2.1.288, Codex 0.153.4 and b10x 0.13.3 observations covering success, failure, tool decisions, cancellation, usage and declared controls. docs/research/2026-10-03-adapter-compatibility.md gives the complete disposition matrix and probe constraints. Existing pins remain unchanged. Offline source fixes and ESS checks are green but do not close these obligations.

## Next owner and clearing condition

Operator: choose the permitted total live-probe spend, or explicitly select offline-only delivery. Coordinator: record the authorization, run bounded sequential probes in private scratch, preserve unknown costs and actual child versions, add sanitized regressions, and clear this blocker only when its withheld evidence is available. An offline-only choice scopes delivery; it does not turn unobserved behavior into passing evidence.

Issues #18–20 also require native final-answer authority, observed model selection and supported command/patch outcomes. Source-backed mappings and regressions are implemented under amendment a22. #20 additionally has a legacy-retention gap: the default history lacks command status, so a supported additional native observation path must be qualified rather than inferring success from output.
