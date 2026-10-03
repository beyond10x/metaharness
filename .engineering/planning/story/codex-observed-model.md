---
format: aep.planning-md/3
id: story:codex-observed-model
kind: story
status: implemented
title: Preserve observed Codex model identity
refs:
- provider: github
  reference: beyond10x/metaharness#19
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
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T09:13:46Z", actor: "human:timo", revision: 3, executor: "agent:codex-metaharness", correlation: "issue-repair-wave"}
- {from: "proposed", to: "active", at: "2026-10-03T09:13:46Z", actor: "human:timo", revision: 4, executor: "agent:codex-metaharness", correlation: "issue-repair-wave"}
- {from: "active", to: "implemented", at: "2026-10-03T10:49:21Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":2,"review_outcome":4,"ess_conformance_coverage_v1":1}}, executor: "agent:codex-metaharness", correlation: "issue-repair-wave"}
---
## Context

GitHub #19 was found in the final open-issue refresh after the initial #10–16 wave. Its source review refers to 63fe8528 and installed Codex 0.153.4 outside pin 0.145.0. The user requested all open issues on one integration branch. Typed homes are drafted and ESS-validated in spec/domains/observations.yaml in managed unit wt-47e4aa5539fd before this story.

## Acceptance

Named production-target scenarios observed-model, model-change and model-missing preserve observed identity and its turn scope without requested-model substitution or invented cost. A narrow native observation is recorded or the remaining source limitation is explicit.

## Scope and sequence

Codex rollout reader, protocol observation and adapter tests; docs/design; spec. Parse actual selected-version model-bearing records; broader compatibility remains #15.

Serialize #17 then #18–20 because protocol, rollout and conformance ownership overlap. Amend the binding design before implementation. Rust only for committed executable fixtures. Keep frame/1 unchanged. Offline production tests do not qualify a current vendor binary. One PR later, no publishing now.

## Offline implementation and evidence boundary

The combined #18–20 unit implements amendment a22 against official Codex rust-v0.153.4 source at 042fb41b7c813ac7999105e886b2b7aa715b5081. docs/research/2026-10-03-codex-observations.md records the source mappings and limits. Ten parser regressions passed after first failing; an additional adversarial failure exposed public success remaining after reused-call invalidation, now corrected. This is coordinator review because worker quota prevented an independent reviewer. Native acceptance remains open; no vendor pin advances.

ESS scenario names: ReadFinalAnswer/outcome/{authoritative,unavailable}; ReadObservedModel/outcome/{observed-selection,unreported}; ReadToolOutcome/outcome/{command-success,command-failure,failure-without-code,success-without-code,unverified}, under metaharness.observations. The production target reads normalized events from the real adapter reader. Separate Rust tests cover malformed and contradictory metadata, wrong family/turn, missing final text and model changes. A green synthetic suite does not replace native observations.

## Native acceptance result

Credential-free native Codex0.153.4 observations now satisfy this story's bounded native acceptance alongside the existing synthetic and ESS regressions. Four explicitly selected Rust tests exercise eight native processes against an owned loopback provider, with the actual version and paginated history asserted from the retained native record. Production normalization preserves authoritative final text, turn-scoped model selection, synthetic-provider usage without invented money, command success/failure and patch outcomes; a denied command has no marker effect and remains outcome-unknown when no completion exists. The full Metaharness binary preserves terminal failure and native process exit1 while returning its own failure verdict3; success returns0. Broader adapter qualification belongs to issue15 and remains open.

Source and reproducible invocation: docs/research/2026-10-03-native-codex-fixture.md and crates/metaharness-codex/tests/native_fixture.rs. Private logs are retained under the coordinator's metaharness-issue-repair/native-fixture cache: native-reviewed.log (four passed), cleanup-test.log, and task-check-approved.log (exit0;762 passed/17 ignored). Independent read-only fixture review found two subprocess-boundary gaps, then verified their fixes; it did not independently execute native runs. No paid model, private transcript commit or pin change is claimed.
