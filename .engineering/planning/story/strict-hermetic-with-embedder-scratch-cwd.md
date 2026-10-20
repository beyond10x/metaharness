---
format: aep.planning-md/3
id: story:strict-hermetic-with-embedder-scratch-cwd
kind: story
status: active
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
  path: CHANGELOG.md
- confidence: cited
  path: crates/metaharness-aep/src/drive/ess_conformance.rs
- confidence: inferred
  path: crates/metaharness-claude/fixtures/c1
- confidence: cited
  path: crates/metaharness-claude/src/launch.rs
- confidence: cited
  path: crates/metaharness-claude/src/vectors.rs
- confidence: inferred
  path: crates/metaharness-codex/fixtures/c1
- confidence: cited
  path: crates/metaharness-codex/src/launch.rs
- confidence: cited
  path: crates/metaharness-codex/src/vectors.rs
- confidence: cited
  path: crates/metaharness-protocol/src/hermetic.rs
- confidence: inferred
  path: crates/metaharness-protocol/src/lib.rs
- confidence: cited
  path: crates/metaharness-protocol/src/spec.rs
- confidence: cited
  path: crates/metaharness/src/audit.rs
- confidence: cited
  path: crates/metaharness/src/builder.rs
- confidence: inferred
  path: crates/metaharness/src/scripted.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: cited
  path: spec/conformance.json
- confidence: cited
  path: spec/domains/workspace.yaml
- confidence: inferred
  path: website/docs/hermetic.md
- confidence: cited
  path: website/docs/reference/cli.md
revision: 43
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T05:04:34Z", actor: "human:timo", revision: 42}
- {from: "proposed", to: "active", at: "2026-10-20T05:04:34Z", actor: "human:timo", revision: 43}
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
amendment), where `<dir>` is a directory the embedder created, exits 0 with a run whose
`session.started.hermetic` carries H7 in a new state `declared` (reason: the embedder declared the
directory as its scratch copy; metaharness did not create it; `--add-dir` never passed) and H11
measured by the ancestor walk over `<dir>`: a memory file above `<dir>` still fails strict, memory
files inside it are listed as the copy's. Today the same invocation exits 1 after the run, because
`attested()` maps H11's `unavailable` row to a gap (`crates/metaharness/src/audit.rs:704-714`); there
is no pre-spawn refusal (`check_spec`, `builder.rs:1554-1625`, has no hermetic clause; both vendor
`plan_launch` cwd gates run only when `spec.cwd` is none). A vector per kind (Claude Code, Codex)
reading `plan.attestation` the way `observe_mode_vector` does pins the `declared` row, and `--cwd`
without the declaration keeps today's exit 1, pinned by the existing unit tests
(`metaharness-claude/src/launch.rs:2672-2713`, `metaharness-codex/src/launch.rs:1864-1882,2002-2011`).

## Wire hazard, read by the scoper

The new state reaches every consumer through serde on `HermeticAttestation`, which sits inside
`session.started` on every committed stream. Without `#[serde(default)]` the recorded streams fail to
parse (`ess_conformance.rs:614-626`, `crates/metaharness-cli/tests/project.rs`); without
`skip_serializing_if` at least seven committed expectations change bytes (claude `fixtures/c2/*.expected.jsonl`
and `fixtures/golden/transcript.expected.jsonl`, codex `fixtures/golden/rollout.expected.jsonl`,
b10x `fixtures/golden/loop.expected.jsonl`, `evals/aep/runs/*.events.jsonl` and `*.trace-ir.json`,
the last pinned byte-for-byte by the trace domain's `golden_stable`). The amendment chooses the
optional, skipped-when-absent shape unless it has a reason not to.

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

Derived 2026-10-20 by `aep:story-scoper` against the tree at `205125c`; cited unless marked inferred.

- **Primary surface:** `crates/metaharness-protocol/src/hermetic.rs` (`HermeticAttestation` 210-240,
  `none()` 250-259, `claims()` 263-265, `UnavailableControl` 196-203 as the model for a `declared`
  control) and `crates/metaharness/src/audit.rs` (`hermetic_floor` H7 607-630, H11 681, `attested()`
  692-720).
- **The flag:** `crates/metaharness-protocol/src/spec.rs` (`RunSpec` is the clap derive, new field
  beside `cwd` 379-394, `RunSpec::new` 570, `with_agent_execution_context` 602-606). Nothing lands
  in `crates/metaharness-cli/src/lib.rs`: `RunArgs` flattens `RunSpec` and `anti_drift.rs:28-44`
  derives the flag set from it.
- **The builder:** `crates/metaharness/src/builder.rs` (`with_cwd` 200-209, `check_spec`
  1554-1625, `resolve_cwd_named` 1499-1518, `start_b10x` attestation 603-648).
- **Vendor launches:** `crates/metaharness-claude/src/launch.rs` (cwd gate 641-656, `attest` a6
  block 1511-1544, tests 2672-2713); `crates/metaharness-codex/src/launch.rs` (cwd gate 602-617,
  `sandbox_mode` 1171-1194, `attest_cwd` 1341-1383, tests 1864-2011).
- **Vectors:** `crates/metaharness-claude/src/vectors.rs` and `crates/metaharness-codex/src/vectors.rs`
  (`observed_plan` records no attestation; a code vector reading `plan.attestation` like
  `observe_mode_vector` pins it without rewriting the c1 fixtures); `fixtures/c1` of both adapters
  only on the fixture route (inferred).
- **Conformance:** `spec/domains/workspace.yaml` (home of `WorkspaceSelection` and the `--cwd` note
  59-63), `crates/metaharness-aep/src/drive/ess_conformance.rs` (new fn modelled on
  `process_write_plan` 37-102, one `execute` arm in the workspace block 1288-1294, the count literal
  1594-1598, the obligation list 1603-1623), `spec/conformance.json` regenerated.
- **Documents:** `docs/design/metaharness-protocol-v0.1.md` (next free amendment, a27 at dispatch
  if nothing else lands first; § 8.1 rows 1218 and 1222; the a6/a6.1 header 26-46),
  `website/docs/reference/cli.md` (`--cwd` row 37 and § 124-145, line 134 states the rule this story
  changes); inferred: `website/docs/hermetic.md`, `CHANGELOG.md`, `crates/metaharness/src/scripted.rs`
  (Done when 4), `crates/metaharness-protocol/src/lib.rs:68-71` (re-export).
- **Shared with `story:project-reads-vendor-transcripts`:** `ess_conformance.rs` lines 1594-1623
  (count literal and obligation list; the arms are 35 lines apart), `spec/conformance.json`
  (regenerated whole), the design doc's amendment tail, `CHANGELOG.md`'s count line. Sequential,
  not parallel: the second unit re-runs the synthesize command over the merged `spec/`.
- Shared with the gap chain: `story:caller-supplied-catalogue-on-every-kind` and
  `story:confinement-of-vendor-kinds` touch `spec.rs`, `hermetic.rs` and the launch files; both
  carry a `depends_on` edge to this story, which lands first.

## Ambiguities

- `requires-stakeholder-input` — whether strict may ever pass on a declaration (a flag) rather than
  on an observation. Default: a `declared` state for H7 only, accepted by strict only together with
  a measured H11 that found nothing above `<dir>`; everything else stays measured. Decides: the
  design amendment's reviewer.
