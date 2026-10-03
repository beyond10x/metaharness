---
format: aep.planning-md/3
id: story:uncapped-spending-policy
kind: story
status: implemented
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
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/metaharness-aep/src/drive.rs
- confidence: inferred
  path: crates/metaharness-aep/src/drive/spending.rs
- confidence: inferred
  path: crates/metaharness-aep/src/drive/spending_tests.rs
- confidence: cited
  path: crates/metaharness-aep/src/drive_tests.rs
- confidence: cited
  path: crates/metaharness-cli/tests/aep_resume.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T07:47:14Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-03T07:47:14Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-10-03T08:45:34Z", actor: "human:timo", revision: 12, decided_on: {"recorded":{"test_result":1,"verification":1}}, executor: "agent:codex-metaharness", correlation: "issue-repair-wave"}
---
## Context

Current drive.rs:1594 requires a positive finite cap and assumed charge; SpendTerms, SpendBudget and spend.json represent only reservations. The requested new policy is specified first by metaharness.spending.Policy, UncappedTerms and InvocationAdmission in spec/domains/spending.yaml (draft worktree), with exact optional observed monetary types in spec/monetary. Design amendment a19 defines policy semantics before implementation. These are Metaharness host terms, not a new AEP workflow or an unbounded engine iteration policy.

## Acceptance

Where a governed model map without a finite cap previously could only be refused, named real-target scenarios uncapped-explicit-authority, uncapped-missing-authority-refused, uncapped-admission-before-spawn, uncapped-persistence-failure, uncapped-resume-authority, finite-resume-cannot-uncap and unknown-cost-stays-unknown demonstrate a separately selected durable uncapped policy while unchanged finite admission, exhaustion and resume-narrowing scenarios still pass.

## Scope and sequence

crates/metaharness-aep/src/drive.rs and drive_tests.rs own CLI options, host launch fields, admission ledger and tests; CLI anti-drift tests may need explicit scope expansion. Amend docs/design/metaharness-protocol-v0.1.md before changing semantics. Update private design and public CLI guidance as needed only after implementation evidence. Runs after #13 to serialize the shared executor/accounting surface. ESS policy draft already supplies typed homes; update conformance mappings after concrete implementation.

## Policy bounds

Explicit uncapped option plus a nonempty operator authorization reference, with live opt-in, persisted before spawning. No omitted-budget fallback, fabricated giant cap or zero assumed charge. Finite policy continues to reserve positive assumed charges; uncapped policy records durable invocation admission without a fabricated reservation. Observed costs are separate optional values, never inferred from reservations. Resume cannot change mode or authorization, and corruption prevents launch. Existing finite ledgers remain readable. This story does not remove retry, visit, iteration, cancellation, tool or engine limits.

## Verification

Offline Rust CLI/admission/persistence/resume tests precede enabling any live run. Test failed spawn and crash-resume unknown observations, overflow and malformed ledgers. Independent adversary and integrated task check follow; ESS Binary64 generator limitation remains explicit rather than changing runtime money types.

## Implementation ownership

Coordinator implementation proceeds locally after worker quota failure. Isolate new tagged policy and uncapped ledger in drive/spending.rs with spending_tests.rs; drive.rs remains the only spawn authority. Finite aep.drive-spend/1 ledger stays readable. New uncapped ledger records ordered invocation coordinates and optional observed cost independently of admission. No resume mode or authorization change. Unit wt-682171833f78 starts from governed Codex unit 9a5f4646; one build at a time reuses wt-0866b435e898/target to limit disk use.

## Conformance mapping and result

Production targets in drive/spending_tests.rs map uncapped-explicit-authority and uncapped-missing-authority-refused to uncapped_admission_requires_explicit_reference_and_live_opt_in; uncapped-admission-before-spawn and unknown-cost-stays-unknown to invocation_precedes_spawn_and_unknown_cost_survives_crash_resume; uncapped-persistence-failure to failed_persistence_grants_no_authority_and_corrupt_order_cannot_resume; uncapped-resume-authority and finite-resume-cannot-uncap to resume_preserves_mode_and_reference_and_reads_legacy_finite_terms. CLI resume tests exercise persisted launch options and conflict/missing-authority refusals. Existing finite admission, exhaustion and narrowing tests remain green.

A planted reserve-without-persist defect failed the file-reading test, then restoration passed. ESS RetainedAdmission also killed that production defect; it reads disk, not intended admission state. Final integration gate: 744 passed, 13 existing live ignores, exit0. Local review and limits are recorded in review-result:issue-repair-local-boundaries. No model request was made, finite ledgers remain readable, and no invented monetary precision or authorization is used.
