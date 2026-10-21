---
format: aep.planning-md/3
id: story:claude-seam-fails-closed
kind: story
status: active
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
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T15:00:10Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-21T15:00:10Z", actor: "human:timo", revision: 4}
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
