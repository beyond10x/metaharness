---
format: aep.planning-md/3
id: story:call-seam-for-harness
kind: story
status: draft
title: A call seam for the in-house harness
summary: metaharness run b10x accepts --decisions frame|ask and puts the caller between harness and its tools per call; observe stays for the eval arm. Amends AGENTS.md invariant 9 first.
owner: metaharness
tags:
- governance
refs:
- provider: atlas
  reference: adr-0098
relations:
- decomposes: epic:loom-agent-work-through-metaharness
- depends_on: story:b10x-adapter-on-harness-over-llm
- depends_on: story:caller-supplied-catalogue-on-every-kind
- depends_on: story:governance-unit-stated
- depends_on: story:confinement-of-vendor-kinds
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: crates/metaharness-b10x/src/launch.rs
- confidence: inferred
  path: crates/metaharness-b10x/src/lib.rs
- confidence: inferred
  path: crates/metaharness-b10x/src/seam.rs
- confidence: inferred
  path: crates/metaharness-b10x/src/vectors.rs
- confidence: inferred
  path: docs/design/metaharness-protocol-v0.1.md
revision: 10
---
# Story: a call seam for the in-house harness

## Scheduling

Draft. Not in the wave of 2026-10-20 (owner's instruction). Gap 5 of Atlas ADR 0098 Consequences.
This story changes AGENTS.md invariant 9 and is a design change (invariant 8): the amendment comes first.

## Outcome

`metaharness run b10x` can put the caller between the in-house harness and its tools, per call, the
way it does for Claude Code and Codex, so Loom governs an in-house session the same way it governs a
vendor one. The observe-only mode stays available for the evaluation arm and keeps asserting that it
decides nothing.

## Context

- The b10x adapter decides nothing by design: `DecisionMode::Observe` only, every `tool.requested`
  with `decision_required: false` and `Seam::None` (`crates/metaharness-b10x/src/lib.rs:1-25`;
  AGENTS.md invariant 9). The reason is the evaluation: a seam on the observed arm "would put the
  driven arm's treatment back on top".
- The loop already consults hook programs before each call: `--hooks <FILE>` is passed when set
  (`crates/metaharness-b10x/src/launch.rs:558-560`); `RunSpec.hooks` is "the only content-level
  refusal this arm has" (`spec.rs:225-229`). The native walk that wrote `metaharness aep drive hook`
  into that file was removed with harness 0.18.0 (story:aep-eval-native-arm-removed).
- harness is kept and builds on llm (ADR 0097); its hook and approval ports (`HookPort`, `ApprovalPort`
  among the eight ports ADR 0098 Context lists) are the seam a per-call decision can cross.
- `--decisions` defaults to `frame` and b10x refuses it by name today (AGENTS.md § Live-evaluating).

## Acceptance

`metaharness run b10x --decisions frame|ask` is accepted; each harness tool call crosses the seam as
`tool.requested` with `decision_required: true`, a `deny` stops the call and the model is told the
reason, `tool.decided` records `decided_by`; `--decisions observe` behaves as before and its
`session.started` says so; AGENTS.md invariant 9 and `docs/design/metaharness-protocol-v0.1.md` § 8.4
are amended in the same change; a recorded vector against the pinned harness release pins both modes.

## Order

Depends on `story:b10x-adapter-on-harness-over-llm`: the seam targets the hook and approval ports of
the harness release that builds on llm, and both stories edit `crates/metaharness-b10x/src/{lib,seam,
vectors}.rs` and re-record vectors against the pinned release. Depends on
`story:caller-supplied-catalogue-on-every-kind` for `crates/metaharness-b10x/src/launch.rs`, and on
`story:governance-unit-stated`, which writes its AGENTS.md paragraph before this story amends
invariant 9 in the same file.

## Out of scope

- Confining the harness process (`story:confinement-of-vendor-kinds`; harness's own confinement
  is Substrate's, AGENTS.md § Out of scope).

## Scope

- inferred: `crates/metaharness-b10x/src/lib.rs`, `src/seam.rs`, `src/launch.rs`, `src/vectors.rs`,
  `AGENTS.md`, `docs/design/metaharness-protocol-v0.1.md`.

## Ambiguities

- `requires-stakeholder-input` — whether the seam is the loop's `--hooks` program (a child process
  metaharness writes) or a wire the harness exposes. Default: the hooks file, because it exists at
  the pinned release; the wire when harness-over-llm publishes one.
