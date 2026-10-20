---
format: aep.planning-md/3
id: story:b10x-record-carries-the-loops-cwd
kind: story
status: active
title: The b10x opening record's working directory comes from the loop, not from metaharness's echo
summary: On b10x session.started.cwd is metaharness's own plan echoed back (seam.rs:163), so H7 is decided by the plan; read it from the loop's record and say unk where the loop reports none.
owner: metaharness
tags:
- governance
refs:
- provider: atlas
  reference: adr-0098
relations:
- decomposes: epic:loom-agent-work-through-metaharness
- depends_on: story:b10x-adapter-on-harness-over-llm
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: inferred
  path: crates/metaharness-b10x/fixtures/golden/README.md
- confidence: cited
  path: crates/metaharness-b10x/fixtures/golden/loop.expected.jsonl
- confidence: cited
  path: crates/metaharness-b10x/src/seam.rs
- confidence: cited
  path: crates/metaharness-b10x/src/vectors.rs
- confidence: inferred
  path: crates/metaharness-b10x/tests/native_fixture.rs
- confidence: cited
  path: crates/metaharness/src/audit.rs
- confidence: cited
  path: crates/metaharness/tests/adversary_scratch_cwd.rs
- confidence: inferred
  path: crates/metaharness/tests/audit.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: cited
  path: website/docs/hermetic.md
revision: 29
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T09:05:36Z", actor: "human:timo", revision: 28}
- {from: "proposed", to: "active", at: "2026-10-20T09:05:37Z", actor: "human:timo", revision: 29}
---
# Story: the b10x opening record's working directory comes from the loop, not from metaharness's echo

## Scheduling

Draft, filed 2026-10-20 from adversary pass 1 on `story:strict-hermetic-with-embedder-scratch-cwd`
(finding 2, `pre-existing`). Proposed as unit 2 of wave 3 of 2026-10-20, after
`story:b10x-adapter-on-harness-over-llm` (unit 1), whose re-recorded 0.14.1 golden it reads.

## Outcome

On the b10x kind, `session.started.cwd` is what the loop itself reported, so the floor's H7 row
(`the working directory is ours`, design § 8.1) is decided by the record and not by metaharness's
own plan echoed back; where the loop reports no working directory, H7 reads `unk` (nobody found
out), never `ok` (invariant 3).

## Context

- `crates/metaharness-b10x/src/seam.rs:163` fills the opening record's `cwd` from `observe_launch`,
  metaharness's own launch plan; the loop's `started` line names no working directory (read
  2026-10-20 at `972a580`, adversary pass 1). So `record cwd == planned cwd` is a tautology on b10x,
  and the floor's H7 arm (`crates/metaharness/src/audit.rs:626-634`) reads `ok` from the plan
  under `--cwd` and, since a27, under `--cwd --scratch-cwd`.
- The b10x golden `crates/metaharness-b10x/fixtures/golden/loop.expected.jsonl` opens with
  `"cwd":"/scratch/run/work"`, the echoed value; the recorded loop record `loop.jsonl` is from
  `b10x-harness 0.9.1`; harness 0.14.0 (tag `58cc4623`) may or may not report the working directory
  in its `--json` record: unverified (invariant 4).
- The consumer that would act on a false `ok`: Loom's session executor over `metaharness run b10x
  --hermetic strict --cwd <copy> --scratch-cwd` (ADR 0098 point 2).

## Acceptance

At the pinned harness release (0.14.1, whose `started` record names no working directory: harness
`crates/harness-loop/src/event.rs:114-201`), the b10x `session.started.cwd` is `null` and H7 reads
`unk` with the reason "the loop did not report its working directory", pinned by a vector over unit
1's recorded 0.14.1 golden whose line 1 is checked to carry no `cwd` key; `loop.expected.jsonl`
line 1 changes to `"cwd":null` and nothing else (a deliberate expected-stream change, read in the
diff); the adversary test written to go red on this change
(`crates/metaharness/tests/adversary_scratch_cwd.rs:141-183`) is rewritten to the new state; a
strict declared-copy b10x run's exit (3, H7 gating) is pinned and named in the changelog. Where a
later harness release names a working directory, a follow-up story reads it; no field name is
guessed here (`seam.rs:234-236`).

## Out of scope

- Making the loop report its working directory: harness (`b10x-harness --json` record); this story
  reads what the pinned release writes and says `unk` where it writes nothing.
- A conformance scenario: at `3d697011` `DeclareScratchWorkingDirectory` runs Claude only
  (`ess_conformance.rs:50`, `spec/coverage.md:258-259`); the former "hermetic domain scenario for
  b10x" names nothing that exists. A b10x scenario is optional and moves the count gate if added.

## Scope

Derived 2026-10-20 by `aep:story-scoper` against `3d697011`; harness read at `f109612e` (0.14.1);
cited unless marked inferred.

- **Primary surface:** `crates/metaharness-b10x/src/seam.rs:162-163` (`cwd: self.cwd.clone()` becomes
  `None`), the then write-only `cwd` field at `:20-21,26-31,41,53,72`, tests `:549-564` and `:650-694`
  (`:688` asserts `Some("/work")`).
- **Floor:** `crates/metaharness/src/audit.rs:607-613`, H7's `(None, _)` arm; the reason needs a
  `direct_provider` arm as H1a `:421-426` and H1b `:457-461` have.
- **Tests:** `crates/metaharness/tests/adversary_scratch_cwd.rs:141-183` (goes red when this lands,
  `:164-167`), `:119-139` (a6 H7 `gap` becomes `unk` because `:609` runs before the attestation),
  doc `:87-88`; inferred: `crates/metaharness/tests/audit.rs:434-449` or `audit.rs:871-909`.
- **Golden and vectors:** `crates/metaharness-b10x/fixtures/golden/loop.expected.jsonl:1`
  (regenerated by `vectors.rs:258-266`); `crates/metaharness-b10x/src/vectors.rs:152-156`
  (`reader()`'s `Some("/scratch/run/work")` at `:155`); inferred: `fixtures/golden/README.md`, one
  sentence; `tests/native_fixture.rs:500-509`, a `start["cwd"].is_null()` check.
- **Documents:** `website/docs/hermetic.md:71-73` (states the echo); `docs/design/metaharness-protocol-v0.1.md`,
  a new amendment after unit 1's (a27's b10x paragraph `:2268-2276` and § 8.1 H7 row `:1243`
  describe the echo); `CHANGELOG.md` `[Unreleased]`.
- **Not touched:** `crates/metaharness/src/builder.rs` (`:709-713` still hands the plan to
  `observe_launch`), `fixtures/enforcement/*`, `contract-result-b10x.json`,
  `crates/metaharness-cli/tests/adversary_project_from.rs:96-99` (already asserts `null`).
- **Constructor:** keep the third parameter of `B10xSeams::new` (ignored) or remove it (14 call sites
  in 10 files); the implementor decides and names the choice.
- **Shared with unit 1 (sequential):** `seam.rs` (no shared lines; both in the `SessionStarted`
  literal `:148-285`), `vectors.rs:152-156` (same call), `loop.expected.jsonl` line 1,
  `golden/README.md`, `native_fixture.rs` `assert_closed`, the design doc's amendment tail,
  `CHANGELOG.md` `[Unreleased]`.
- **Safety fact:** the record's `cwd` is read only by H7 (`audit.rs:608`) and the trace-ir/1
  `SessionStart` projection (`projection.rs:145`). H7 is gating (`hermetic.rs:161-165`,
  `audit.rs:162-173`), so a clean `run b10x --hermetic strict --cwd <copy> --scratch-cwd` moves from
  exit 0 to exit 3; whether Loom's executor runs b10x under strict is not known here.
- **Estimate:** 3 tests changed, 2-3 new; 1 field on 1 fixture line.
