---
format: aep.planning-md/3
id: story:confinement-of-vendor-kinds
kind: story
status: draft
title: Confinement of vendor kinds
summary: A Claude Code or Codex session run for Loom is bounded from outside the vendor's permission system and the record says what was bounded; metaharness attests, Loom confines over Substrate.
owner: metaharness
tags:
- governance
refs:
- provider: atlas
  reference: adr-0098
relations:
- decomposes: epic:loom-agent-work-through-metaharness
- depends_on: story:continuity-across-executor-calls
scope:
- confidence: inferred
  path: crates/metaharness-claude/src/launch.rs
- confidence: inferred
  path: crates/metaharness-codex/src/launch.rs
- confidence: inferred
  path: crates/metaharness-protocol/src/hermetic.rs
- confidence: inferred
  path: crates/metaharness/src/run.rs
- confidence: inferred
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: inferred
  path: spec/domains/workspace.yaml
revision: 9
---
# Story: confinement of vendor kinds

## Scheduling

Draft. Not in the wave of 2026-10-20 (owner's instruction). Gap 6 of Atlas ADR 0098 Consequences.

## Outcome

A Claude Code or Codex session run for Loom cannot touch what Commission did not grant: the
filesystem and process surface of the session is bounded from outside the vendor's own permission
system, and the run's record says what was bounded (invariant 3: from the record, never from
configuration).

## Context

- "The vendor harnesses bring their own tools and their own containment story"
  (`crates/metaharness-protocol/src/spec.rs:425`); ADR 0098 Context: "vendor harnesses get no
  confinement beyond their own, and the sandbox-inversion design was rejected on 2026-09-01".
- The rejection stands in `docs/ROADMAP.md` § 4: no Substrate dependency, no production outer-envelope
  provider, no vendor-adapter routing through one; the neutral `ProcessEnvelope` contract (0.4.0)
  remains as evidence vocabulary with absence-as-unknown tests.
- ADR 0099 point 4: "Confinement of takt's test commands and harness children comes through Loom
  over Substrate". Read with the rejection, the route is: Loom confines the `metaharness run` child
  through Substrate, and metaharness attests what its run observed about that confinement.
- What exists: declared process-write subtrees and their observation
  (`story:native-process-write-subtrees`, `spec/domains/workspace.yaml`, `DeclareProcessWrites`),
  for the b10x kind.

## Acceptance

Under a scripted `ProcessEnvelope` (the 0.4.0 contract `docs/ROADMAP.md` § 4 kept), a recorded
`metaharness run claude|codex` replay that attempts one write outside the declared surface produces
a `session.started.hermetic` block with that path under a new `denied` field, and `--hermetic strict`
exits non-zero on it; today the attestation has no such field and the run exits 0.

## Done when

1. `HermeticAttestation` gains `write_surface` (what the envelope declared writable) and `denied`
   (paths the run's own record shows refused), both `null` when no envelope was handed over
   (absence is not a property, invariant 3); the design page is amended first.
2. The recorded vector above is committed per kind (Claude Code, Codex) and pins both fields.
3. `--hermetic strict` exits non-zero when `denied` is non-empty for a path the frame admitted.

The live confinement that produces a real envelope is Loom's over Substrate (ADR 0099 point 4) and
is not part of this story's vector.

## Order

Depends on `story:continuity-across-executor-calls`: both change `crates/metaharness/src/run.rs`,
both `launch.rs` files and the protocol design; the scratch home's lifetime that story settles is
what an envelope has to cover.

## Out of scope

- Implementing confinement in metaharness (rejected 2026-09-01). This story observes and attests.
- Harness's own confinement (Substrate's boundary, AGENTS.md § Out of scope).

## Scope

- inferred: `crates/metaharness-protocol/src/hermetic.rs`, `crates/metaharness/src/run.rs`,
  `crates/metaharness-claude/src/launch.rs`, `crates/metaharness-codex/src/launch.rs`,
  `spec/domains/workspace.yaml`, `docs/design/metaharness-protocol-v0.1.md` § 8.3.

## Ambiguities

- `requires-stakeholder-input` — whether the operator keeps the 2026-09-01 rejection (metaharness
  attests only; Loom confines over Substrate) or reverses it. Default: keep it. Decides: operator.
