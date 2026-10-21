---
format: aep.planning-md/3
id: story:h11-claims-match-what-was-measured
kind: story
status: draft
title: Claude's H11 claims match what was measured
summary: An ok H11 on Claude names its source and every unprobed surface, and a pin move without a re-measured canary fails a check.
owner: metaharness
refs:
- provider: atlas
  reference: adr-0096
relations:
- decomposes: epic:session-format
- informed_by: review-result:adr0096-w5-independent-review
revision: 1
---
# Story: Claude's H11 claims match what was measured

## Scheduling

Draft, filed 2026-10-21 at the close of wave 5 from `review-result:adr0096-w5-independent-review` (s2-1,
s3-1, s4-1, s2-2, s4-5, s2-4, s4-4, s4-2).

## Outcome

An `ok` H11 on Claude says exactly what was measured and where it came from, and the canary that backs it is
re-measured whenever a pin moves.

## Context

- Since `story:claude-h11-memory-walk-shapes` (amendment a36), Claude's H11 `ok` is computed from the launch
  attestation and the version-keyed `MEMORY_CANARIES` table, not from the run's record
  (`crates/metaharness/src/audit.rs:1072-1124`, `crates/metaharness-claude/src/lib.rs:371-389`). AGENTS.md
  invariant 3 asserts hermeticity from the run's own record and is not amended for a36 (s2-1, s3-1, s4-1).
- The `ok` text is wider than the probe: `MEMORY_NOT_PROBED` (`lib.rs:353-356`) names only managed memory,
  while user memory in the config home (s2-2), ancestors above the cwd's parent (s2-4, s4-5) and
  `.claude/rules` below depth one (s4-4) were not probed either.
- The canary re-probe a36 requires at each pin move is `#[ignore]`
  (`crates/metaharness-claude/tests/native_fixture.rs:1052`), and the gate-time pin test
  (`tests/adapter.rs:131-161`) accepts any self-consistent row (s4-2).

## Acceptance

- expected_before: today an `ok` H11 on Claude reads as this run's measurement and names only managed memory
  as unprobed, and a pin move with a copied canary row passes `task check`. After: the `ok` detail says the
  value is the canary row for that version and names every unprobed surface (user memory, ancestors above
  the parent, rules below depth one, managed memory); invariant 3 records the a36 exception or the record
  carries the evidence; and a pin move without a re-measured canary row fails a named check.

## Scope

Inferred: `crates/metaharness/src/audit.rs`, `crates/metaharness-claude/src/lib.rs`,
`crates/metaharness-claude/tests/adapter.rs`, `crates/metaharness-claude/tests/native_fixture.rs`,
`crates/metaharness/src/builder.rs` (the `ok` sentence test), `AGENTS.md` (invariant 3),
`docs/design/metaharness-protocol-v0.1.md` (a36).
