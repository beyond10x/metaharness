---
format: aep.planning-md/3
id: story:b10x-record-carries-the-loops-cwd
kind: story
status: draft
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
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/metaharness-b10x/fixtures/golden/loop.expected.jsonl
- confidence: inferred
  path: crates/metaharness-b10x/src/seam.rs
- confidence: inferred
  path: crates/metaharness-b10x/src/vectors.rs
- confidence: inferred
  path: crates/metaharness/src/audit.rs
- confidence: inferred
  path: crates/metaharness/src/builder.rs
- confidence: inferred
  path: docs/design/metaharness-protocol-v0.1.md
revision: 8
---
# Story: the b10x opening record's working directory comes from the loop, not from metaharness's echo

## Scheduling

Draft, filed 2026-10-20 from adversary pass 1 on `story:strict-hermetic-with-embedder-scratch-cwd`
(finding 2, `pre-existing`): not in wave 2; waits for the budget hold to lift.

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

With a loop record that names a working directory, `session.started.cwd` equals it and H7 reads
`ok` only when it equals the planned directory; with a loop record that names none, `session.started.cwd`
is `null` and H7 reads `unk` with the reason "the loop did not report its working directory";
pinned by a recorded vector per shape against the pinned harness release; `loop.expected.jsonl`
changes accordingly (a deliberate expected-stream change, read in the diff), and the hermetic domain
scenario for b10x reads the new states.

## Out of scope

- Making the loop report its working directory: harness (`b10x-harness --json` record); this story
  reads what the pinned release writes and says `unk` where it writes nothing.

## Scope

- inferred: `crates/metaharness-b10x/src/seam.rs`, `crates/metaharness-b10x/src/vectors.rs`,
  `crates/metaharness-b10x/fixtures/golden/loop.expected.jsonl`, `crates/metaharness/src/audit.rs`,
  `crates/metaharness/src/builder.rs`, `docs/design/metaharness-protocol-v0.1.md`, `CHANGELOG.md`.
- Shares `seam.rs`, `vectors.rs` and the golden with `story:b10x-adapter-on-harness-over-llm`
  (re-records the golden); lands after it.
