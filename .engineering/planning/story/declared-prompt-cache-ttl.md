---
format: aep.planning-md/1
id: story:declared-prompt-cache-ttl
kind: story
status: active
title: Declare per-run prompt cache lifetime
relations:
- decomposes: epic:runs-side-by-side
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/metaharness-claude/src/launch.rs
- confidence: cited
  path: crates/metaharness-cli/tests/anti_drift.rs
- confidence: cited
  path: crates/metaharness-codex/src/launch.rs
- confidence: cited
  path: crates/metaharness-protocol/src/lib.rs
- confidence: cited
  path: crates/metaharness-protocol/src/spec.rs
- confidence: cited
  path: crates/metaharness/src/builder.rs
- confidence: cited
  path: crates/metaharness/tests/run_loop.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: cited
  path: website/docs/reference/cli.mdx
- confidence: cited
  path: website/docs/reference/library.mdx
revision: 14
---
## Requirements

Advance O3 by exposing an explicit per-run prompt cache lifetime through the library and CLI, so a governed caller can select five-minute caching without changing model, complete context, reasoning effort, budgets or admission controls. A retained cold-session cost profile already exceeds its reservation before model output; low effort alone cannot address that input charge. Static inspection of installed Claude 2.1.263 establishes a promptCacheTtl setting accepting 5m and 1h. This is binary evidence, not a completed live cache-policy measurement and not a claim about the adapter's unchanged 2.1.259 global pin.

Introduce an optional closed PromptCacheTtl duration with serde/Clap spellings 5m and 1h, default None, omitted on serialization when absent. Expose the same declaration through SessionBuilder. Only the Claude adapter maps the declaration to promptCacheTtl in its generated per-run settings. Continue scrubbing ambient TTL variables; do not edit operator settings, inherit arbitrary environment, add a generic settings passthrough, move the vendor pin or change any frame/event/contract-result bytes. Shared startup and direct public Codex planning must refuse the setting for unsupported kinds rather than ignore it. Amend the existing design before implementation.

## Acceptance

Offline red-before/green-after tests establish CLI, SDK and serde parity for both durations; malformed durations/shapes and unsupported harnesses refuse before launch. Omission preserves the prior serialized RunSpec and launch settings. Removing only the new settings member from an explicit launch reproduces the prior plan: identical prompt/context, model, budget, turns, hooks, permissions, credentials, argv and environment. Conflicting ambient TTL values never enter the child. The complete task check and changed website reference build pass, with exact source and binary identity recorded before the AEP consumer or private live proof relies on the capability.

## Scope

Derived 2026-09-06 by story-scoper from the existing tree. Placement is cited; proposed option mechanism is inferred.

- crates/metaharness-protocol/src/spec.rs:167 — cited: RunSpec, closed duration, serde/Clap, defaults and inline tests.
- crates/metaharness-protocol/src/lib.rs:79 — cited: public option exports.
- crates/metaharness/src/builder.rs:175 — cited: SDK setter and shared unsupported-kind guard at 1619.
- crates/metaharness/tests/run_loop.rs:288 — cited: exhaustive RunSpec fixture and SDK/from_spec parity.
- crates/metaharness-claude/src/launch.rs:630 — cited: pure launch plan, generated settings at 1444, environment guards and inline tests.
- crates/metaharness-codex/src/launch.rs:772 — cited: direct planner unsupported-option refusal and inline tests.
- crates/metaharness-cli/tests/anti_drift.rs:32 — cited: actual CLI/RunSpec option parity and parsing.
- docs/design/metaharness-protocol-v0.1.md:1194 — cited: amend hermetic and two-face option contract before implementation; label evidence scope.
- website/docs/reference/cli.mdx:51 — cited: explicit per-run flag, supported kind and omission.
- website/docs/reference/library.mdx:33 — cited: matching SDK setter.
- CHANGELOG.md — cited: Unreleased record.

## Compatibility and delivery

No external exact RunSpec byte pin was found among inspected provider/consumer surfaces; external exhaustive Rust literals need the new field on rebuild. Preserve absent-field serialization and all cross-repository sealed bytes. Do not add public conformance vectors: contract-result golden counts/provider identities and AEP fixtures would then require coordinated migration. Ordinary launch, SDK and CLI tests exercise this option without that expansion. The existing global vendor pin and historical captures remain intact. Provider order is metaharness, AEP, then engine declaration. The activation owner separately proves actual installed vendor settings and successful extraction within its original cumulative allowance; this code story authorizes no model call, installation into global paths, publication or history rewrite.
