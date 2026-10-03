---
format: aep.planning-md/3
id: story:governed-codex
kind: story
status: implemented
title: Run Codex steps through the governed adapter seam
owner: metaharness
refs:
- provider: github
  reference: beyond10x/metaharness#13
relations:
- decomposes: epic:github-issue-repair
- depends_on: story:codex-terminal-failure
- depends_on: story:current-aep-runtime
- depends_on: story:managed-workspace-admission
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/metaharness-aep/Cargo.toml
- confidence: cited
  path: crates/metaharness-aep/src/drive.rs
- confidence: cited
  path: crates/metaharness-aep/src/drive_tests.rs
- confidence: cited
  path: crates/metaharness-codex
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T07:30:53Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-03T07:30:54Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-10-03T08:45:34Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"test_result":1,"verification":1}}, executor: "agent:codex-metaharness", correlation: "issue-repair-wave"}
---
## Context

The existing normalized Codex adapter is absent from the concrete AEP host's Harness enum (crates/metaharness-aep/src/drive.rs:1408), so selecting codex cannot invoke it. Design amendment a19 defines the change; the drafted ESS session and invocation types under spec/domains provide the typed home before this story. Existing frame, operation and AEP action types remain authoritative.

## Acceptance

Where a codex step previously returned unsupported-harness NoVerdict, named real-target scenarios governed-codex-success, governed-codex-provider-failure, governed-codex-interruption, governed-codex-engine-denial, governed-codex-unsupported-operation, governed-codex-unknown-cost and governed-codex-resume-identity demonstrate an admitted invocation through the existing ask seam with every supported effectful call adjudicated by the current engine step and unsupported controls explicitly refused.

## Scope and sequence

crates/metaharness-aep/src/drive.rs and drive_tests.rs own selection, launch, engine mapping, transcript and resume tests. Vendor interpretation, if required for a typed action translation, belongs in crates/metaharness-codex, never a second downstream decoder. docs/design/metaharness-protocol-v0.1.md amendment a19 binds behavior. Shared protocol additions require explicit review and scope update; frame/1 bytes remain unchanged. Runs after #10, #11 and #12 because these land on its seams. #14 follows because it shares admission and accounting surfaces. Real supported controls remain version-bounded; #15 owns new pin evidence.

## Verification

Write red tests before implementation, drive production translation and StepAuthorizer with both allow and deny, verify malformed/unsupported requests cannot enter the allow exemption, retain task/state/step/attempt identity on resume, preserve unknown costs, and exercise provider failure and interruption without a paid model. Independent adversary precedes aggregation and full task check. A live supported-model run requires the separate bounded probe budget.

## Conformance mapping and result

Production Rust targets in drive_tests.rs: governed-codex-success maps to governed_codex_is_selected_as_an_adjudicating_harness, codex_resume_options_launch_the_same_ask_seam_and_do_not_load_claude_plugins and the zero-exit branch of governed_codex_terminal_exit_failure_and_interruption_never_complete_a_step; provider-failure and interruption map to its nonzero/signal cases. Engine-denial and unsupported-operation map to codex_decisions_reach_the_engine_and_unsupported_calls_never_gain_an_exemption (actual AEP engine). Resume-identity maps to codex_resume_options_launch_the_same_ask_seam_and_do_not_load_claude_plugins and governed_codex_uses_the_current_frame_coordinates_and_no_skill_exemption. Unknown-cost is checked by the Codex terminal contract and ESS FinishCodexCompletion target (normalized total_cost_usd remains None).

These are offline production-boundary tests, not a real vendor loop. Final integration task check: 744 passed, 13 existing ignored live tests, exit0. Coordinator review record issue-repair-local-boundaries replaces the unavailable independent worker review and says so. New-version native qualification remains story:current-adapter-compatibility. No paid probe was run.
