---
format: aep.planning-md/3
id: story:native-process-termination
kind: story
status: implemented
title: Expose measured native process termination
refs:
- provider: github
  reference: beyond10x/metaharness#17
relations:
- decomposes: epic:github-issue-repair
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/metaharness
- confidence: cited
  path: crates/metaharness-aep/src/drive.rs
- confidence: cited
  path: crates/metaharness-aep/src/drive/ess_conformance.rs
- confidence: cited
  path: crates/metaharness-aep/src/drive_tests.rs
- confidence: cited
  path: crates/metaharness-claude/tests/recorded_runs.rs
- confidence: cited
  path: crates/metaharness-cli
- confidence: cited
  path: crates/metaharness-protocol
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: cited
  path: evals/aep/runs
- confidence: cited
  path: spec
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T08:50:34Z", actor: "human:timo", revision: 3, executor: "agent:codex-metaharness", correlation: "issue-repair-wave"}
- {from: "proposed", to: "active", at: "2026-10-03T08:50:34Z", actor: "human:timo", revision: 4, executor: "agent:codex-metaharness", correlation: "issue-repair-wave"}
- {from: "active", to: "implemented", at: "2026-10-03T09:12:27Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"test_result":1,"ess_conformance_coverage_v1":1}}, executor: "agent:codex-metaharness", correlation: "issue-repair-wave"}
---
## Context

GitHub #17 was found in the final open-issue refresh after the initial #10–16 wave. Its source review refers to 63fe8528 and installed Codex 0.153.4 outside pin 0.145.0. The user requested all open issues on one integration branch. Typed homes are drafted and ESS-validated in spec/domains/observations.yaml in managed unit wt-47e4aa5539fd before this story.

## Acceptance

Named executable scenarios native-exit-zero, native-exit-nonzero-after-terminal, native-signal, native-status-missing and terminal-failure-with-zero-exit expose runner-observed status in normalized output, retain exactly one final closure, preserve cleanup and distinguish transport completion from consumer success. Unknown and signals never become exit0.

## Scope and sequence

crates/metaharness-protocol/src/event.rs; crates/metaharness-protocol/src/stream.rs; core process/run/spawn/spawn_codex/scripted; audit and CLI semantics; Rust executable test fixture; docs/design; spec. No consumer repository change or live model required.

Serialize #17 then #18–20 because protocol, rollout and conformance ownership overlap. Amend the binding design before implementation. Rust only for committed executable fixtures. Keep frame/1 unchanged. Offline production tests do not qualify a current vendor binary. One PR later, no publishing now.

## Contract decision and reproduction

Amendment a21 specifies additive stream.closed.process, with exited/code, signaled/signal or unknown. Native status does not change terminal/steering closure reason. This corrects the initial #11 implementation's conflation of native failure and normalized terminal failure: successful terminal framing can retain transport exit0 while recording native exit7; explicit terminal errors and incomplete terminal evidence still have nonzero CLI results. Consumers must require all three facts. AEP rejects measured native failure even when transport exits0; StepOutcome::Nothing is still no evidence of task success.

A Rust executable fixture using the real SpawnRunner reproduced missing native status (expected exited/code0, observed null), exit101. Both real SpawnRunner and CodexSpawnRunner now run actual zero/nonzero/signal children and fault-injected unavailable wait results. The latter retains try_wait status intact. Old stream records deserialize to unknown; synthetic replay does not invent native status. An additional host regression reproduced hidden failure behind transport exit0, exit101, before the normalized-status guard.

Three generated ESS ReadNativeTermination outcomes extend the suite from 24 to 27. Production subprocess tests cover the producer; ESS covers the normalized reader. Updating additive fields required deliberate regeneration of two synthetic event streams and their trace/view projections; no live transcript changed. Logs: local cache metaharness-issue-repair/termination17. Unit has not yet passed its final gate or integrated.

## Source result

Unit 8adb80526e9364627f09de00f264a8e1d2d4bf09 passed task check: 751 passing tests, 13 existing ignored live tests; both ESS manifests valid, 27 generated scenarios passed without skips/refusals. Real subprocess cases execute both spawn runners; all children are waited/reaped. The host's new regression passes after reproducing red. Final markers remain single and last. Clippy all targets with warnings denied and formatting pass. Source integration is complete; no publication, release or consumer adoption is claimed. Coordinator review fallback checked status retention at try_wait/wait, missing-status defaults, synthetic non-evidence, frame invariance, EOF/steering ordering and the normalized-only host guard. Independent worker review remains unavailable because of quota.
