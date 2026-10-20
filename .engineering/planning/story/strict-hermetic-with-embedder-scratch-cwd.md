---
format: aep.planning-md/3
id: story:strict-hermetic-with-embedder-scratch-cwd
kind: story
status: draft
title: --hermetic strict admits an embedder-declared scratch working directory
summary: 'takt gap G15: metaharness 0.9.3 refuses HermeticMode::Strict with a named cwd (builder.rs:199-205, a6); Loom''s session executor needs strict over its own scratch copy, attested from the record.'
owner: metaharness
tags:
- governance
refs:
- provider: atlas
  reference: adr-0098
- provider: takt
  reference: G15
relations:
- decomposes: epic:loom-agent-work-through-metaharness
scope:
- confidence: inferred
  path: crates/metaharness-aep/src/drive/ess_conformance.rs
- confidence: inferred
  path: crates/metaharness-claude/fixtures/c1
- confidence: inferred
  path: crates/metaharness-claude/src/launch.rs
- confidence: inferred
  path: crates/metaharness-cli/src/lib.rs
- confidence: inferred
  path: crates/metaharness-codex/fixtures/c1
- confidence: inferred
  path: crates/metaharness-codex/src/launch.rs
- confidence: inferred
  path: crates/metaharness-protocol/src/hermetic.rs
- confidence: inferred
  path: crates/metaharness-protocol/src/spec.rs
- confidence: inferred
  path: crates/metaharness/src/builder.rs
- confidence: inferred
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: inferred
  path: spec/conformance.json
- confidence: inferred
  path: spec/domains/workspace.yaml
revision: 13
---
# Story: `--hermetic strict` admits an embedder-declared scratch working directory

## Scheduling

Draft, filed 2026-10-20 on takt's gap G15 (relayed by session `adr-changes`): it blocks takt's
session-executor crate S2 (Atlas ADR 0098 point 2, ADR 0099 point 3). Proposed as the first unit of
the wave after the trace-ir wave of 2026-10-20.

## Outcome

Loom's session executor runs `metaharness run <kind>` inside a scratch copy of the workspace that
the embedder made, under `--hermetic strict`, and the run is admitted with an attestation that says
what was observed about that directory, instead of being refused as if the directory were the
operator's repository.

## Context

- `with_cwd` is "an operator-named working directory for the child, instead of a scratch one"
  (`crates/metaharness/src/builder.rs:199-205`). Amendment a6 (`docs/design/metaharness-protocol-v0.1.md:26`):
  under such a directory H7 and H11 are attested unavailable, never claimed, so `--hermetic strict`
  refuses the run and `--hermetic` reports the trade by name (`crates/metaharness-claude/src/launch.rs:1511-1528`).
  Amendment a6.1: the adapter widens the vendor sandbox over the named tree
  (`crates/metaharness-protocol/src/spec.rs:388-394`).
- H7, gating: "the working directory is ours — a directory metaharness created; `--add-dir` never
  passed" (design § 8.1, line 1218). H11, gating: "no memory file outside the copied tree is
  discoverable" — the ancestor walk from the cwd (line 1222).
- takt's scoping notes (`scope-s1-s2-notes.md`, S2, 2026-10-20): "metaharness 0.9.3 refuses
  HermeticMode::Strict with an operator-named cwd (builder.rs:200-205; metaharness-claude
  launch.rs:1515-1528, rows H7, H11 unavailable). Needs a metaharness change (Strict with an
  embedder's scratch cwd) or the owner accepting HermeticMode::On." Also: "ScriptedLog records argv
  only; a test needs its own ProcessRunner reading the launch plan's cwd and env, via
  Metaharness::start_with (scripted.rs:64-115, process.rs:42-71)."
- The distinction a6 did not need: an operator's real tree (writes land in the repository; the
  memory files are the operator's) and an embedder's disposable scratch copy (writes are the
  proposed change Commission rechecks, ADR 0098 point 2; memory files are copies). a6 treats both
  as the first. Invariant 3 still holds: a flag cannot make a row `ok`; the row says what the run
  observed and what was declared.

## Acceptance

`metaharness run claude --hermetic strict --cwd <dir> --scratch-cwd` (flag name settled by the
amendment), where `<dir>` is a directory the embedder created, is admitted, and the run's
`session.started.hermetic` carries H7 in a new state `declared` (reason: the embedder declared the
directory as its scratch copy; metaharness did not create it; `--add-dir` never passed) and H11
measured by the ancestor walk over `<dir>`: a memory file above `<dir>` still fails strict, memory
files inside it are listed as the copy's. Today the same invocation exits with the a6 refusal. A
recorded C1 launch vector per kind (Claude Code, Codex) pins the attestation, and `--cwd` without
the declaration keeps the a6 refusal, pinned by the existing vector.

## Done when

1. The design page carries the amendment (next free number at dispatch) defining the `declared`
   attestation state, which rows may take it (H7 only), and what strict does with it.
2. `HermeticAttestation` and the protocol spec carry the declaration; `metaharness-protocol` gains
   no dependency (invariant 1).
3. Both vendor launches honour it; the b10x kind states whether it applies.
4. A scripted `ProcessRunner` test reads the launch plan's cwd (takt's note) so the vector does not
   rest on argv alone.
5. The `metaharness.workspace` or `metaharness.boundaries` domain gains the conformance scenario
   for the admitted and the refused shape, and the count gate moves.

## Out of scope

- Confining the scratch copy (`story:confinement-of-vendor-kinds`).
- Making the scratch copy (Loom's executor does; ADR 0098 point 2).

## Scope

- inferred: `crates/metaharness/src/builder.rs`, `crates/metaharness-protocol/src/spec.rs`,
  `crates/metaharness-protocol/src/hermetic.rs`, `crates/metaharness-claude/src/launch.rs`,
  `crates/metaharness-codex/src/launch.rs`, `crates/metaharness-cli/src/lib.rs`,
  `crates/metaharness-claude/fixtures/c1`, `crates/metaharness-codex/fixtures/c1`,
  `docs/design/metaharness-protocol-v0.1.md`, `spec/domains/workspace.yaml`,
  `spec/conformance.json`, `crates/metaharness-aep/src/drive/ess_conformance.rs`.
- Shared with the gap chain: `story:caller-supplied-catalogue-on-every-kind` and
  `story:confinement-of-vendor-kinds` touch `spec.rs`, `hermetic.rs` and the launch files; both
  carry a `depends_on` edge to this story, which lands first.

## Ambiguities

- `requires-stakeholder-input` — whether strict may ever pass on a declaration (a flag) rather than
  on an observation. Default: a `declared` state for H7 only, accepted by strict only together with
  a measured H11 that found nothing above `<dir>`; everything else stays measured. Decides: the
  design amendment's reviewer.
