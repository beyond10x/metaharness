---
format: aep.planning-md/3
id: story:claude-seam-fails-closed
kind: story
status: implemented
title: The Claude tool-call seam fails closed
refs:
- provider: atlas
  reference: adr-0096
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/metaharness-claude/src/hook.rs
- confidence: cited
  path: crates/metaharness-claude/src/launch.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T15:00:10Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-21T15:00:10Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-21T18:43:46Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"ess_conformance_coverage_v1":1}}}
---
# Story: the Claude tool-call seam fails closed

## Scheduling

Wave 6 unit 1 (`approval-record:adr-0096-wave-6`), filed 2026-10-21 from the wave-5 unit 2 adversary
review. Approved by the operator ("A", relayed by `b10x-chef-orchestrator` on 2026-10-21) as a security
fix, with the amendment included.

## Outcome

When the Claude adapter's tool-call seam produces no decision, the call is refused, never allowed.

## Context

- The decision and the vendor readings per pinned version are amendment a41 of
  `docs/design/metaharness-protocol-v0.1.md`.
- Security-fix rule until the fix is released: pushed text outside that amendment and the code and tests
  carries one CHANGELOG line and no further detail.

## Acceptance

- Each failure case named in a41 has a test that shows the call is refused.
- The CHANGELOG carries one line under `### Security`: "Security: the Claude tool-call seam fails closed."
- No spawn vector moves except by what a41 names.

## Delivered

Commit `cacb923`, merged `5ddebbd` into `integration/adr-0096-wave-6` (`approval-record:adr-0096-wave-6`,
unit 1). Mechanism: amendment a41. Security review: confirmed, no change needed (one `aep:security-reviewer`
pass). Unit gate 91 test binaries, 1054 passed, 0 failed, 48 ignored, conformance 103/103.

Whole gate at the wave-6 head: task check, one step at a time, on `integration/adr-0096-wave-6` at
`5ddebbd`: 6 steps exit 0, 96 test binaries, 1128 passed, 0 failed, 49 ignored, conformance 103/103 on ESS
0.67.0.

Further detail, and the review record, wait until release 0.12.0 is out.
