---
format: aep.planning-md/3
id: story:codex-terminal-failure
kind: story
status: implemented
title: Codex native terminal failures survive normalization and CLI exit
owner: metaharness
refs:
- provider: github
  reference: beyond10x/metaharness#11
relations:
- decomposes: epic:github-issue-repair
scope:
- confidence: cited
  path: crates/metaharness-cli/src/lib.rs
- confidence: cited
  path: crates/metaharness-codex
- confidence: cited
  path: crates/metaharness/src/audit.rs
- confidence: cited
  path: crates/metaharness/src/run.rs
- confidence: cited
  path: crates/metaharness/src/spawn_codex.rs
- confidence: cited
  path: crates/metaharness/tests/stream_closed.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T23:46:32Z", actor: "human:timo", revision: 7}
- {from: "proposed", to: "active", at: "2026-10-02T23:46:32Z", actor: "human:timo", revision: 8}
- {from: "active", to: "implemented", at: "2026-10-03T10:49:21Z", actor: "human:timo", revision: 16, decided_on: {"recorded":{"test_result":3}}, executor: "agent:codex-metaharness", correlation: "issue-repair-wave"}
---
## Context

GitHub #11 reports Codex 0.153.4 task_complete with a non-null error being normalized as session.ended with absent error fields, stream.closed completed and exit 0. The pinned 0.145.0 version remains distinct from the observed installed version. Source: crates/metaharness-codex/src/rollout.rs:135 (finish) and :298 (task_complete); crates/metaharness/src/run.rs:1140 (reason_from_record). Existing protocol types own terminal semantics; this fix introduces no new entity.

## Acceptance

Synthetic tests and a bounded native unsupported-model probe demonstrate that explicit Codex terminal failure, including failure after preliminary text, yields normalized failure and nonzero CLI exit, actual success remains successful, and incomplete terminal evidence never claims success, without publishing private transcripts or claiming untested vendor compatibility.

## Scope

Cited: crates/metaharness-codex/src/rollout.rs and adapter tests, crates/metaharness/src/run.rs terminal classification, crates/metaharness/src/spawn_codex.rs process outcome, crates/metaharness-cli/src/lib.rs exit mapping, docs/design/metaharness-protocol-v0.1.md terminal semantics. Changes must stay at the owning adapter/core seam; no downstream vendor codec.

## Sequence and evidence

Write and run minimal synthetic regression before implementation. Record red output, then amend the binding design before behavior changes. Incorporate the relevant ESS terminal contract and real-target scenarios under story:ess-specification-and-hardening. Record green regression, native probe vendor version and sanitized conclusions, independent review and full task check. Keep new compatibility pins out until issue #15's evidence is complete.

## Reproduction handoff

Test-only worker wt-13a8621a208a at base 13a8378697fec1462b1014b04e5eb556bcba77cd added five adapter cases in crates/metaharness-codex/src/bridge.rs and seven core cases in crates/metaharness/tests/stream_closed.rs. Command: CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 cargo test -p metaharness-codex -p metaharness --no-fail-fast codex_terminal_regression -- --nocapture. Exit 101: adapter 2 passed/3 failed, core 2 passed/5 failed. Formatting passed. No implementation edit or live probe occurred.

Evidence is retained outside Git in the coordinator's metaharness-issue-repair/codex-terminal scratch directory: report.md, red.log, baseline.log, first-red.log and regression.patch. No private vendor transcript was used. The worker lease is released. Root remains owner of integration and pending implementation.

Source-scope correction: crates/metaharness/src/audit.rs also needs review because AuditReport::exit ignores terminal failure. Legacy successful golden records omit error but include last_agent_message; positive success evidence must be distinguished from a bare task_complete. Decide that in the binding design before the fix.

## Integration result and remaining acceptance

The adapter/core/CLI fix and adversarial synthetic boundaries are integrated and green in the final 744-test gate. ESS terminal cases also pass and unknown price remains absent. The required native unsupported-model probe is still pending under dependency-blocker:adapter-probe-budget; no live evidence or current vendor qualification is inferred from synthetic tests. Keep this story active until that acceptance is satisfied.

## Native acceptance result

Credential-free native Codex0.153.4 observations now satisfy this story's bounded native acceptance alongside the existing synthetic and ESS regressions. Four explicitly selected Rust tests exercise eight native processes against an owned loopback provider, with the actual version and paginated history asserted from the retained native record. Production normalization preserves authoritative final text, turn-scoped model selection, synthetic-provider usage without invented money, command success/failure and patch outcomes; a denied command has no marker effect and remains outcome-unknown when no completion exists. The full Metaharness binary preserves terminal failure and native process exit1 while returning its own failure verdict3; success returns0. Broader adapter qualification belongs to issue15 and remains open.

Source and reproducible invocation: docs/research/2026-10-03-native-codex-fixture.md and crates/metaharness-codex/tests/native_fixture.rs. Private logs are retained under the coordinator's metaharness-issue-repair/native-fixture cache: native-reviewed.log (four passed), cleanup-test.log, and task-check-approved.log (exit0;762 passed/17 ignored). Independent read-only fixture review found two subprocess-boundary gaps, then verified their fixes; it did not independently execute native runs. No paid model, private transcript commit or pin change is claimed.
