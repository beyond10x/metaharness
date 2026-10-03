---
format: aep.planning-md/3
id: story:current-adapter-compatibility
kind: story
status: active
title: Verify installed adapter releases before advancing pins
owner: metaharness
refs:
- provider: github
  reference: beyond10x/metaharness#15
relations:
- decomposes: epic:github-issue-repair
- depends_on: story:governed-codex
- depends_on: story:scripted-b10x-run-does-not-need-the-binary
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.lock
- confidence: inferred
  path: crates/metaharness-b10x
- confidence: inferred
  path: crates/metaharness-claude
- confidence: cited
  path: crates/metaharness-codex
- confidence: inferred
  path: docs/research
- confidence: cited
  path: docs/research/2026-10-03-adapter-compatibility.md
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T10:10:31Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-03T10:10:31Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"review_outcome":1}}}
---
## Context

Installed banners are Claude 2.1.288, Codex 0.153.4 and b10x-harness 0.13.3; current pins are older. Issue #15 calls existing success observations insufficient for general compatibility. Each version claim must be supported by the corresponding observed surface; retained private transcripts never enter Git.

## Acceptance

For each installed release explicitly named here—Claude 2.1.288, Codex 0.153.4 and b10x-harness 0.13.3—the compatibility-success, compatibility-terminal-failure, compatibility-tool-decision, compatibility-cancellation, compatibility-model-usage and compatibility-declared-controls matrix records a disposition backed by sanitized observed evidence and synthetic regression vectors, keeps untested claims explicitly unverified, and advances a pin only when its required evidence is complete; an empty proposed-pin set does not satisfy this acceptance.

## Scope and sequence

Adapter crates metaharness-claude, metaharness-codex and metaharness-b10x own claims and synthetic vectors. Core spawn vector tests may change after scope confirmation. docs/research holds sanitized method, binary versions and measured conclusions; CHANGELOG records any pin change, Cargo.lock only if a deliberate source revision is required. Do not change pins to suppress drift warnings. Native b10x evidence does not establish governed execution. Runs after #11/#13 and scripted launch repairs to avoid concurrent adapter/core edits.

## Verification and resources

Offline suites first. Short live probes run sequentially in private scratch with a shared explicit operator spending limit, no private repository task, and no unbounded model loop. Budget requested while offline work proceeds; a missing answer does not authorize spending. Record each command, version, time bound, cost if observed and exact unsupported scope. If a required surface cannot be exercised, retain its older pin/unverified label and record the unresolved issue honestly.

## Offline inventory result

Read-only banners still report Claude 2.1.288, Codex 0.153.4 and b10x-harness 0.13.3. docs/research/2026-10-03-adapter-compatibility.md records all six required surfaces for each version, distinguishes offline observations from native evidence, and leaves every incomplete claim unverified. No pins changed. dependency-blocker:adapter-probe-budget names the missing authorization; this story is not implemented.

## Credential-free native fixture work

The release goal authorizes continuing this story. Native vendor binaries can be exercised against an owned loopback fixture provider without operator credentials or a paid API request. Start with actual Codex 0.153.4, a fresh HOME/CODEX_HOME, CredentialSource::None, explicit loopback model_endpoint and the production launch/rollout reader. Assert no credential copies, no authorization header and no hosted endpoint before treating the result as a credential-free observation. Bound the child, server and retained files. These tests are explicitly opt-in and never launch a vendor binary in task check.

The fixture can establish native record format, final-answer/model mapping and failure behavior. Its synthetic responses and usage do not establish hosted model quality, provider/subscription authentication, observed money or general compatibility. Those limits must remain explicit; the paid-probe budget question remains pending for hosted qualification. Rust only for committed executable fixtures. This unit is serialized after #18–20 and release-preparation source.

## Native Codex fixture result

The actual installed Codex0.153.4, with paginated history confirmed by session_meta, passed four opt-in Rust tests exercising eight native processes. The production adapter/hook/rollout path preserves final text, selected model, synthetic-provider token usage, provider refusal, correlated command exits0/7, hook denial without its marker side effect, and patch success without an invented exit code. Full metaharness run codex cases preserve successful/refused terminals and final stream closure, native exits0/1 and CLI verdict exits0/3. All requests target an owned loopback server with no operator credentials or authorization headers; no paid request occurred. Native invocation and private logs are described in docs/research/2026-10-03-native-codex-fixture.md.

Two initial probe assumptions were corrected from observed records, not by changing production behavior: denied commands lack a structured completion and remain unknown; CLI verdict3 differs from native exit1. An independent read-only review found and then verified fixes for fixture child cleanup on panic and an unbounded redundant version subprocess. The native suite passed again. task check exited0 with762 passed/17 ignored, strict ESS validation and the production conformance suite green. The additional ordinary test proves direct-child cleanup after a forced panic; descendant-group cleanup was not independently exercised.

This completes the bounded Codex observations for issues11 and18–20. It does not complete this story: cancellation, remaining declared controls, the wider AEP path, Claude2.1.288, b10x0.13.3 and hosted-provider qualification remain unverified. No pins advance. Budget authorization remains unanswered for any paid probes.
