---
format: aep.planning-md/1
id: story:llm-route-bindings
kind: story
status: draft
title: Vendor adapters consume resolved shared LLM bindings
relations:
- decomposes: epic:llm-adoption
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: crates/metaharness-b10x
- confidence: inferred
  path: crates/metaharness-claude
- confidence: inferred
  path: crates/metaharness-cli
- confidence: inferred
  path: crates/metaharness-codex
- confidence: inferred
  path: docs/design/model-adapter-v0.1.md
revision: 2
---
## Context

Evidence: docs/design/model-adapter-v0.1.md and crates/metaharness-protocol/src/spec.rs. Translate a shared resolved route into each vendor adapter launch binding. Preserve metaharness-protocol dependency invariants; integrate outside the sealed protocol crate and keep all vendor flags in their owning adapters. Do not let binding keys override hermetic environment, isolation, tool authority or owned-process semantics.

## Acceptance

Pinned vendor-launch fixtures consume the same LLM route while rejecting reserved isolation overrides and preserving the existing protocol dependency boundary.

## Verification

Retain exact release/contract identities, baseline and candidate results, and the repository gate. No paid provider call or deployment occurs in the ordinary gate.

## Scope

- inferred: `crates/metaharness-claude` — adoption surface.
- inferred: `crates/metaharness-codex` — adoption surface.
- inferred: `crates/metaharness-b10x` — adoption surface.
- inferred: `crates/metaharness-cli` — adoption surface.
- inferred: `Cargo.toml` — adoption surface.
- inferred: `Cargo.lock` — adoption surface.
- inferred: `docs/design/model-adapter-v0.1.md` — adoption surface.

## External prerequisites

- depends_on:llm/story:foundation-qualified

AEP 0.55.0 currently refuses cross-member targets while creating mutation locator evidence (kind contains disallowed character /). These are explicit references, not admitted graph edges. The local llm-foundation-release blocker gates adoption until the exact upstream release evidence exists.
