---
format: aep.planning-md/3
id: story:codex-final-answer
kind: story
status: active
title: Expose authoritative Codex final answer
refs:
- provider: github
  reference: beyond10x/metaharness#18
relations:
- decomposes: epic:github-issue-repair
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/metaharness-aep/src/drive/ess_conformance.rs
- confidence: cited
  path: crates/metaharness-b10x
- confidence: cited
  path: crates/metaharness-b10x/src/seam.rs
- confidence: cited
  path: crates/metaharness-claude
- confidence: cited
  path: crates/metaharness-claude/src/transcript.rs
- confidence: cited
  path: crates/metaharness-codex
- confidence: cited
  path: crates/metaharness-protocol
- confidence: cited
  path: crates/metaharness/src/scripted.rs
- confidence: cited
  path: crates/metaharness/tests/audit.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: cited
  path: docs/research/2026-10-03-adapter-compatibility.md
- confidence: cited
  path: docs/research/2026-10-03-codex-observations.md
- confidence: cited
  path: evals/aep/runs
- confidence: cited
  path: spec
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T09:13:46Z", actor: "human:timo", revision: 3, executor: "agent:codex-metaharness", correlation: "issue-repair-wave"}
- {from: "proposed", to: "active", at: "2026-10-03T09:13:46Z", actor: "human:timo", revision: 4, executor: "agent:codex-metaharness", correlation: "issue-repair-wave"}
---
## Context

GitHub #18 was found in the final open-issue refresh after the initial #10–16 wave. Its source review refers to 63fe8528 and installed Codex 0.153.4 outside pin 0.145.0. The user requested all open issues on one integration branch. Typed homes are drafted and ESS-validated in spec/domains/observations.yaml in managed unit wt-47e4aa5539fd before this story.

## Acceptance

Named production-target scenarios commentary-before-final, duplicate-final-representations, missing-final-answer and terminal-error-after-text preserve only authoritative final-answer evidence without promoting arbitrary Text. Selected-version source and native observations are distinguished, unknowns remain absent, and current pins require independent live qualification.

## Scope and sequence

Codex rollout reader and adapter tests, protocol terminal payload, docs/design, spec. Native observation joins the bounded probe budget.

Serialize #17 then #18–20 because protocol, rollout and conformance ownership overlap. Amend the binding design before implementation. Rust only for committed executable fixtures. Keep frame/1 unchanged. Offline production tests do not qualify a current vendor binary. One PR later, no publishing now.

## Offline implementation and evidence boundary

The combined #18–20 unit implements amendment a22 against official Codex rust-v0.153.4 source at 042fb41b7c813ac7999105e886b2b7aa715b5081. docs/research/2026-10-03-codex-observations.md records the source mappings and limits. Ten parser regressions passed after first failing; an additional adversarial failure exposed public success remaining after reused-call invalidation, now corrected. This is coordinator review because worker quota prevented an independent reviewer. Native acceptance remains open; no vendor pin advances.

ESS scenario names: ReadFinalAnswer/outcome/{authoritative,unavailable}; ReadObservedModel/outcome/{observed-selection,unreported}; ReadToolOutcome/outcome/{command-success,command-failure,failure-without-code,success-without-code,unverified}, under metaharness.observations. The production target reads normalized events from the real adapter reader. Separate Rust tests cover malformed and contradictory metadata, wrong family/turn, missing final text and model changes. A green synthetic suite does not replace native observations.
