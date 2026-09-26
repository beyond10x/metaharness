---
format: aep.planning-md/1
id: story:llm-route-evidence
kind: story
status: draft
title: Metaharness observes model-route identity and costs truthfully
relations:
- decomposes: epic:llm-adoption
- depends_on: story:llm-route-bindings
scope:
- confidence: inferred
  path: contracts
- confidence: inferred
  path: crates/metaharness-b10x
- confidence: inferred
  path: crates/metaharness-claude
- confidence: inferred
  path: crates/metaharness-codex
- confidence: inferred
  path: crates/metaharness-protocol
- confidence: inferred
  path: docs/design
revision: 2
---
## Context

Evidence: metaharness-protocol owns the sealed execution event/frame contract; the current adapter reports cache creation and exact Harness pins. Add an explicitly versioned projection where LLM route/attempt/cost evidence crosses that seam. Preserve unknown versus zero, intended versus observed model, and consumer compatibility. Do not claim that an external vendor process exposed internal fallback events it never reported.

## Acceptance

Cross-adapter fixtures preserve model, usage and cost provenance or explicit absence without inventing observations or changing frozen frame bytes.

## Verification

Retain exact release/contract identities, baseline and candidate results, and the repository gate. No paid provider call or deployment occurs in the ordinary gate.

## Scope

- inferred: `crates/metaharness-protocol` — adoption surface.
- inferred: `crates/metaharness-b10x` — adoption surface.
- inferred: `crates/metaharness-claude` — adoption surface.
- inferred: `crates/metaharness-codex` — adoption surface.
- inferred: `contracts` — adoption surface.
- inferred: `docs/design` — adoption surface.

## External prerequisites

- depends_on:llm/story:foundation-qualified

AEP 0.55.0 currently refuses cross-member targets while creating mutation locator evidence (kind contains disallowed character /). These are explicit references, not admitted graph edges. The local llm-foundation-release blocker gates adoption until the exact upstream release evidence exists.
