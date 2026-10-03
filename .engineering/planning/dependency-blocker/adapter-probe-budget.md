---
format: aep.planning-md/3
id: dependency-blocker:adapter-probe-budget
kind: dependency-blocker
status: cleared
title: Current vendor qualification awaits a bounded live-probe budget
relations:
- blocks: story:current-adapter-compatibility
withholds: test_result
revision: 6
transitions:
- {from: "open", to: "cleared", at: "2026-10-03T12:02:25Z", actor: "human:timo", revision: 6}
---
## Pending decision

The approved wave authorizes offline implementation. The separate question about a shared live-probe budget has not been answered; continuation does not imply paid-model authority. No paid requests were launched for this wave.

## Concrete remaining work

Issue #11 still needs its bounded native unsupported-model failure observation. Issue #15 requires current installed Claude 2.1.288, Codex 0.153.4 and b10x 0.13.3 observations covering success, failure, tool decisions, cancellation, usage and declared controls. docs/research/2026-10-03-adapter-compatibility.md gives the complete disposition matrix and probe constraints. Existing pins remain unchanged. Offline source fixes and ESS checks are green but do not close these obligations.

## Next owner and clearing condition

Operator: choose the permitted total live-probe spend, or explicitly select offline-only delivery. Coordinator: record the authorization, run bounded sequential probes in private scratch, preserve unknown costs and actual child versions, add sanitized regressions, and clear this blocker only when its withheld evidence is available. An offline-only choice scopes delivery; it does not turn unobserved behavior into passing evidence.

Issues #18–20 also require native final-answer authority, observed model selection and supported command/patch outcomes. Source-backed mappings and regressions are implemented under amendment a22. #20 additionally has a legacy-retention gap: the default history lacks command status, so a supported additional native observation path must be qualified rather than inferring success from output.

## Source correction: persistent Codex exec history

Read-only inspection of official Codex rust-v0.153.4 exec/src/lib.rs at 042fb41b7c813ac7999105e886b2b7aa715b5081 changes the earlier inference: thread_start_params_from_config requests Paginated history when not ephemeral. Metaharness refuses --ephemeral. start_thread falls back to unspecified/default legacy only on the explicit server error that paginated threads require listing support. The enum default is therefore not proof that this selected exec path normally uses legacy history. Existing support for retained CommandExecution items can cover the normal persistent path; native qualification must observe which path was taken. Legacy/resumed/fallback records that omit status remain unknown. No additional history flag or alternate vendor parser is currently justified.

## Current disposition

The credential-free native fixture now supplies the bounded evidence for issues11 and18–20; their blocking edges were removed through the CLI. Actual Codex0.153.4 session metadata confirms paginated history, correlated command outcomes are observed, and denial without a completion remains unknown. The native and full-binary results are recorded in docs/research/2026-10-03-native-codex-fixture.md. No paid request was required for those observations.

Only story:current-adapter-compatibility remains blocked here. Its wider qualification matrix still requires completion; the pending budget answer applies to any hosted requests. Prior paragraphs describe the earlier state and are superseded by this result for issues11 and18–20.

## Cleared by credential-free observations

The required bounded native compatibility matrix is now supplied without any paid or hosted request: Codex6tests, Claude4tests and b10x7tests passed against the immutable0.9.0 candidate. The loopback-only provider/fresh-home boundary is enforced. Codex0.153.4 and b10x0.13.3 pins advance only for observed surfaces; Claude's H1a gap and native process-exec withholding remain unverified. b10x's strict-version prompted scoper observes the selected role text, readonlysurface and realfilebytes against the source/versionpair798325f. Hosted model quality/authentication and money were never required to infer from these fixtures and remain outside the claims. The pending spend answer authorizes no paid requests; it no longer withholds the evidence needed by this story.
