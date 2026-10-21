---
format: aep.planning-md/3
id: story:claude-seam-abstain-is-recorded-as-vendor-decided
kind: story
status: draft
title: A Claude seam abstain is recorded as decided by the vendor
summary: An abstained Claude tool call names the vendor pipeline as its decider, and a41 names abstain as outside the fail-closed claim.
owner: metaharness
refs:
- provider: atlas
  reference: adr-0096
relations:
- informed_by: review-result:adr0096-w6u1-review
- informed_by: story:claude-seam-fails-closed
revision: 1
---
# Story: a Claude seam abstain is recorded as decided by the vendor

## Scheduling

Draft, filed 2026-10-21 after release 0.12.0 from `review-result:adr0096-w6u1-review` (s6-1).

## Outcome

When metaharness abstains on a Claude tool call, the run's record says the vendor's own permission
pipeline decided that call, and amendment a41 states that abstain is outside the fail-closed claim.

## Context

- An abstain is an empty answer file; the hook program exits 0 with no output and the vendor's permission
  pipeline decides the call (`crates/metaharness/src/run.rs:962-978`,
  `crates/metaharness-claude/src/hook.rs:267-283`, at `cacb923`).
- A run with `--decisions frame` and no frame abstains on every call; abstain is by design
  (a41 §§ 5–6, `crates/metaharness/src/seam.rs:189-193`, at `cacb923`).
- Verified on the metaharness side; what the vendor then allows is inferred, not driven.

## Acceptance

- expected_before: today an abstained call is counted as abstained in the census
  (`crates/metaharness/src/run.rs:1166`) and the record does not name the vendor pipeline as its decider;
  a41 states the seam fails closed without naming abstain. After: each abstained call
  on the Claude arm carries a record entry naming the vendor pipeline as the decider; a run that abstains on
  every call emits one warning at start saying so; a41 names abstain as outside the fail-closed claim.

## Scope

Inferred: `crates/metaharness/src/run.rs`, `crates/metaharness-claude/src/hook.rs`,
`docs/design/metaharness-protocol-v0.1.md`, `CHANGELOG.md`.
