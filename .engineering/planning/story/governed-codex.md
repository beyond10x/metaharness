---
format: aep.planning-md/3
id: story:governed-codex
kind: story
status: draft
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
  path: crates/metaharness-aep/src/drive.rs
- confidence: cited
  path: crates/metaharness-aep/src/drive_tests.rs
- confidence: cited
  path: crates/metaharness-codex
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
revision: 2
---
## Context

The existing normalized Codex adapter is absent from the concrete AEP host's Harness enum (crates/metaharness-aep/src/drive.rs:1408), so selecting codex cannot invoke it. Design amendment a11 defines the change; the drafted ESS session and invocation types under spec/domains provide the typed home before this story. Existing frame, operation and AEP action types remain authoritative.

## Acceptance

Where a codex step previously returned unsupported-harness NoVerdict, named real-target scenarios governed-codex-success, governed-codex-provider-failure, governed-codex-interruption, governed-codex-engine-denial, governed-codex-unsupported-operation, governed-codex-unknown-cost and governed-codex-resume-identity demonstrate an admitted invocation through the existing ask seam with every supported effectful call adjudicated by the current engine step and unsupported controls explicitly refused.

## Scope and sequence

crates/metaharness-aep/src/drive.rs and drive_tests.rs own selection, launch, engine mapping, transcript and resume tests. Vendor interpretation, if required for a typed action translation, belongs in crates/metaharness-codex, never a second downstream decoder. docs/design/metaharness-protocol-v0.1.md amendment a11 binds behavior. Shared protocol additions require explicit review and scope update; frame/1 bytes remain unchanged. Runs after #10, #11 and #12 because these land on its seams. #14 follows because it shares admission and accounting surfaces. Real supported controls remain version-bounded; #15 owns new pin evidence.

## Verification

Write red tests before implementation, drive production translation and StepAuthorizer with both allow and deny, verify malformed/unsupported requests cannot enter the allow exemption, retain task/state/step/attempt identity on resume, preserve unknown costs, and exercise provider failure and interruption without a paid model. Independent adversary precedes aggregation and full task check. A live supported-model run requires the separate bounded probe budget.
