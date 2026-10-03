---
format: aep.planning-md/3
id: story:uncapped-spending-policy
kind: story
status: draft
title: Represent explicit uncapped USD authority in governed runs
owner: metaharness
refs:
- provider: github
  reference: beyond10x/metaharness#14
relations:
- decomposes: epic:github-issue-repair
- depends_on: story:governed-codex
scope:
- confidence: cited
  path: crates/metaharness-aep/src/drive.rs
- confidence: cited
  path: crates/metaharness-aep/src/drive_tests.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
revision: 2
---
## Context

Current drive.rs:1594 requires a positive finite cap and assumed charge; SpendTerms, SpendBudget and spend.json represent only reservations. The requested new policy is specified first by metaharness.spending.Policy, UncappedTerms and InvocationAdmission in spec/domains/spending.yaml (draft worktree), with exact optional observed monetary types in spec/monetary. Design amendment a11 defines policy semantics before implementation. These are Metaharness host terms, not a new AEP workflow or an unbounded engine iteration policy.

## Acceptance

Where a governed model map without a finite cap previously could only be refused, named real-target scenarios uncapped-explicit-authority, uncapped-missing-authority-refused, uncapped-admission-before-spawn, uncapped-persistence-failure, uncapped-resume-authority, finite-resume-cannot-uncap and unknown-cost-stays-unknown demonstrate a separately selected durable uncapped policy while unchanged finite admission, exhaustion and resume-narrowing scenarios still pass.

## Scope and sequence

crates/metaharness-aep/src/drive.rs and drive_tests.rs own CLI options, host launch fields, admission ledger and tests; CLI anti-drift tests may need explicit scope expansion. Amend docs/design/metaharness-protocol-v0.1.md before changing semantics. Update private design and public CLI guidance as needed only after implementation evidence. Runs after #13 to serialize the shared executor/accounting surface. ESS policy draft already supplies typed homes; update conformance mappings after concrete implementation.

## Policy bounds

Explicit uncapped option plus a nonempty operator authorization reference, with live opt-in, persisted before spawning. No omitted-budget fallback, fabricated giant cap or zero assumed charge. Finite policy continues to reserve positive assumed charges; uncapped policy records durable invocation admission without a fabricated reservation. Observed costs are separate optional values, never inferred from reservations. Resume cannot change mode or authorization, and corruption prevents launch. Existing finite ledgers remain readable. This story does not remove retry, visit, iteration, cancellation, tool or engine limits.

## Verification

Offline Rust CLI/admission/persistence/resume tests precede enabling any live run. Test failed spawn and crash-resume unknown observations, overflow and malformed ledgers. Independent adversary and integrated task check follow; ESS Binary64 generator limitation remains explicit rather than changing runtime money types.
