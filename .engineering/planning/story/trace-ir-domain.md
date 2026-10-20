---
format: aep.planning-md/3
id: story:trace-ir-domain
kind: story
status: implemented
title: trace-ir/1 is the ESS domain metaharness.trace and its conformance runs in the gate
summary: spec/domains/trace.yaml declares the session format; generated scenarios execute against the production projection and the three production readers; the suite count gate moves from 45.
owner: metaharness
tags:
- spec
- trace
refs:
- provider: atlas
  reference: adr-0096
relations:
- decomposes: epic:session-format
- informed_by: story:trace-ir-reader
- informed_by: story:ess-specification-and-hardening
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: README.md
- confidence: inferred
  path: crates/metaharness-aep/Cargo.toml
- confidence: cited
  path: crates/metaharness-aep/src/drive/ess_conformance.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: cited
  path: docs/design/runs-side-by-side-v0.1.md
- confidence: cited
  path: spec/conformance.json
- confidence: cited
  path: spec/coverage.md
- confidence: cited
  path: spec/domains/trace.yaml
- confidence: cited
  path: spec/ess-inputs.yaml
- confidence: cited
  path: spec/system.yaml
revision: 22
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T02:14:53Z", actor: "human:timo", revision: 18, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "proposed", to: "active", at: "2026-10-20T02:14:54Z", actor: "human:timo", revision: 19, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "active", to: "implemented", at: "2026-10-20T04:28:51Z", actor: "human:timo", revision: 22, decided_on: {"recorded":{"test_result":1,"review_outcome":7,"verification":1,"ess_conformance_coverage_v1":1}}}
---
# Story: `trace-ir/1` is the ESS domain `metaharness.trace`, and its conformance runs in the gate

## Outcome

The session format this repository owns (Atlas ADR 0096 point 7) is declared in `spec/` as the
domain `metaharness.trace`, and `task check` executes generated scenarios for it against the
production projection and the production readers, the way the five existing domains are checked.

## Context

- Document shape, from `crates/metaharness-protocol/src/trace_ir.rs`: `TraceIrDocument { format:
  "trace-ir/1", transcript_digest, adapter { name: "metaharness/project", written_against:
  ["metaharness.event/1"] }, events: [TraceIrEvent { index, source_line, timestamp, timestamp_ms,
  kind }], requests: [AssistantRequest], metaharness: MetaharnessBlock { source, run, events_total,
  families, unk_kinds, stream_complete, closed, vendor_transcript, withheld, available_operations,
  hermetic, census } }`. Constants: `UNK_FAMILY = "unk"`, `UNK_REASON = "no trace-ir/1 family"`,
  `CLOSED_FAMILY = "stream_closed"`. Entry point: `project_document(lines, bytes)`; `render()`.
- Families, from `crates/metaharness-protocol/src/projection.rs`: ten `IrFamily` variants
  (`session_start`, `run_outcome`, `assistant_text`, `assistant_thinking`, `thinking_estimate`,
  `synthetic_injection`, `tool_call`, `tool_result`, `rate_limit`, `opaque`) and nine
  `CONTROL_PLANE_EVENTS`. The wire has twenty event names (`framing.rs:28`, `EVENT_NAMES`).
- The complete mapping is the twenty-row table in `docs/design/runs-side-by-side-v0.1.md` § 1.2:
  twelve kinds land in a family (`usage` folds into `run_outcome`), eight are written as `unk`
  nodes (`step.entered`, `step.left`, `turn.started`, `turn.ended`, `tool.decided`,
  `command.result`, `warning`, `auth.expired`), and `stream.closed` is the terminal `stream_closed`
  node (amendment a17).
- Properties already decided and tested in code: every event is a node (node count = line count);
  `unk` is not `opaque`; `transcript_digest` is over the stream's own bytes with
  `metaharness.source = "metaharness.event/1"` and the vendor's reference beside it as
  `vendor_transcript`; `stream_complete` is verified, not copied (marker present, last, and counts
  exactly the nodes before it); no clock, no network, same input twice is the same bytes (P1, P3,
  a15, a17).
- Specification baseline: `spec/system.yaml` (five domains, `ess/15`), `spec/ess-inputs.yaml`
  (`requires: ess 0.56.0`), `spec/conformance.json` (`ess-conformance/35`, 45 generated scenarios),
  `spec/coverage.md`. Target: `crates/metaharness-aep/src/drive/ess_conformance.rs` — `execute`
  dispatches on command name (lines 575–660), the count gate is `assert_eq!(scenarios.len(), 45)`
  at line 874, the required-command list follows at 880–895, and
  `ess_suite_matches_the_current_specification` regenerates the suite with
  `ess verify conform synthesize --path spec --suite-format 5 --strict-requires` (lines 973–996).
  CI installs ESS from commit `84ee8d38eb69e3a4507de801fdcb248232ed6e6e` (`.github/workflows/gate.yml:66-67`).
- Committed inputs the target may read by path: `evals/aep/runs/decomposer-clean.events.jsonl`
  (16 lines, ends with `stream.closed`) and `plan-reviewer-clean.events.jsonl` (14 lines); each
  adapter's golden vendor record and expected stream: `crates/metaharness-claude/fixtures/golden/
  transcript.jsonl` (+ `.expected.jsonl`, 8 events), `crates/metaharness-codex/fixtures/golden/
  rollout.jsonl` (23 events), `crates/metaharness-b10x/fixtures/golden/loop.jsonl` (10 events).
  Each expected stream opens with `session.started`. The readers are `metaharness_claude::
  TranscriptReader` (driven in `crates/metaharness-claude/tests/recorded_runs.rs`; reachable from
  `metaharness-aep` without a manifest change through `metaharness::ClaudeSeams`,
  `crates/metaharness/src/lib.rs:103`, whose `SeamFactory::build` wraps the reader,
  `crates/metaharness-claude/src/bridge.rs:35`), `metaharness_codex::RolloutReader`
  (`src/rollout.rs:48`, exported at `lib.rs:71`) and `metaharness_b10x::B10xSeams` (`src/seam.rs:26,35`;
  driven in `src/vectors.rs`, `golden_loop_vector`). `metaharness-codex` and `metaharness-b10x` are
  already dependencies of `metaharness-aep` (`Cargo.toml:10-11`). Reading the committed
  `evals/aep/runs/*.events.jsonl` in `cargo test` has precedent in `recorded_runs.rs:13-15`
  (invariant 5 is about paid runs).

## Specification

The domain `metaharness.trace`, in `spec/domains/trace.yaml`, declared in `spec/system.yaml` and
`spec/ess-inputs.yaml`, must state:

- Types: the twenty wire event kinds; the twelve node families (ten IR families plus `unk` and
  `stream_closed`); the four source kinds a record is normalized from (`claude-code-stream-json`,
  `codex-rollout`, `harness-json`, `metaharness-event-stream`), used only as inputs of the
  normalize commands; a node (`index`, `source_line`, `timestamp`, `timestamp_ms`, `family`); the
  document header (`format`, `transcript_digest`, adapter name, `metaharness.source`); the census
  (`events_total`, `unk` count, `stream_complete`). The header field `metaharness.source` is
  observed as the constant `metaharness.event/1`, which is all production writes today
  (`trace_ir.rs`, `MetaharnessBlock::source`); whether it ever carries a vendor format name is
  `story:project-reads-vendor-transcripts`'s open question and is marked `UNMAPPED:` in the domain.
- Commands, as conformance operations over production code (never a re-implementation of the
  mapping): classify one event kind into its family, one outcome per family, the eight `unk` kinds
  carrying `reason == "no trace-ir/1 family"` and `stream.closed` carrying its own family; verify
  stream completeness over three marker shapes (absent, last and counting, present but not last or
  miscounting); name the digest, with and without a vendor transcript reference on
  `session.started`; project each of the two recorded runs into an immutable retained record whose
  view exposes node count equals line count, the family census and byte stability across two
  renders; normalize each of the three golden vendor records through its production reader and
  then `project_document`, one outcome per source kind, each observing an opening `session_start`
  node, and the recorded `metaharness.event/1` stream as the fourth source.
- Comments in the domain cite the Rust and design sources above, mark every semantic not read from
  code or design `UNMAPPED:` (ESS refuses guessed semantics), and say that these commands are
  conformance observations, not public CLI verbs.

The design documents are binding and are amended before code (AGENTS.md invariant 8). Where
production output and the design disagree, the scenario records what production does and the
disagreement is a finding in the report, never a fix inside this unit. Drive the Claude reader
through `metaharness::ClaudeSeams` so no manifest changes; if that path and `TranscriptReader::new`
differ in output over the same bytes, that too is a finding.

## Acceptance

On the integration branch, `task check` exits 0 with `spec/conformance.json` carrying at least 22
generated `metaharness.trace.*` scenarios (one per declared outcome: 12 classify, 3 completeness,
2 digest, 1 retained projection plus its view, 4 normalize) that `ess_conformance.rs` executes
against `metaharness_protocol::trace_ir::project_document` and the three production readers, with
zero skipped, unsupported or refused; the story moves `active → implemented` on that run's
`test_result` and the `ess-conformance-report/2` recorded as evidence.

## Done when

Each line is read from a file or a command, not asserted:

1. `spec/system.yaml` and `spec/ess-inputs.yaml` declare `metaharness.trace`; `ess specify validate
   --path spec --strict-requires` (ESS 0.56.0) exits 0.
2. `spec/conformance.json` is the output of `ess verify conform synthesize --path spec
   --suite-format 5 --strict-requires` on ESS 0.56.0 and `ess_suite_matches_the_current_specification`
   passes.
3. The count gate at `ess_conformance.rs:874-878` and the required-command list at lines 883–898
   name the new total and the five new commands (the `execute` match ends at 669 with
   `other => panic!`, so a scenario without an arm fails closed); the total is written into
   `spec/coverage.md` (lines 8, 19, 121 state 45 today), `README.md:81` ("The tests execute 45
   generated conformance scenarios") and this story's Result section.
4. `docs/design/metaharness-protocol-v0.1.md` carries amendment a26: the document form's owning
   specification is `spec/domains/trace.yaml` (ADR 0096 point 7); `docs/design/runs-side-by-side-v0.1.md`
   § 1 points at it. a26 is the next number at the time of writing (a25 is the last today); a later
   story takes the next free number at its own dispatch.
5. `CHANGELOG.md` gains an `## [Unreleased]` heading above `## [0.9.3]` (line 6; none exists
   today) with the entry under it.
6. The b10x golden record `crates/metaharness-b10x/fixtures/golden/loop.jsonl` is read as it stands
   and is not changed by this story. It was captured from `b10x-harness 0.9.1`
   (`crates/metaharness-b10x/fixtures/golden/version.txt:1`), not from the 0.13.3 pin, as the
   implementor found; the domain states 0.9.1 and declares no b10x node count, so
   `story:b10x-adapter-on-harness-over-llm`'s later re-record moves no scenario. That story depends
   on this one so the trace scenarios stay green across the move.

## Hardening evidence required

At least one planted defect per command group, failing by scenario name and restored green: one in
the specification (a family swapped in the classify table), one in production (the `unk` reason
constant changed, or the completeness check made to copy the marker). A guard-negation mutation run
(`ess verify conform mutate --class guard-negate`) over the new domain with its kill count, or the
statement that it was not run and why.

## Out of scope

- Reading a vendor record through `metaharness project` directly (`story:project-reads-vendor-transcripts`).
- Changing any adapter reader, the projection, or `trace_ir.rs`. A mismatch is reported, not fixed.
- Moving the ESS pin off 0.56.0. The gate runs on the pinned compiler.
- Publishing a `trace-ir/1` JSON schema or a reader for third parties (design Q9, half closed).

## Scope

Derived 2026-10-20 by `aep:story-scoper` against the tree. Every line is **cited** (read from the
story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `spec/` and `crates/metaharness-aep/src/drive/ess_conformance.rs` — cited
- **Files (spec):** `spec/domains/trace.yaml` (new); `spec/system.yaml` (lists five domains, lines
  4–9); `spec/ess-inputs.yaml` (`specification:` file list, lines 3–9); `spec/conformance.json`
  (regenerated; `counts.generated: 45`); `spec/coverage.md:8,19,121` (count and suite statements) — cited
- **Files (target):** `crates/metaharness-aep/src/drive/ess_conformance.rs` — `execute` match
  575–669 (`other => panic!` at 668), suite-version list 857–861, count gate 874–878,
  required-command list 883–898, regen test `ess_suite_matches_the_current_specification` 972–995 — cited
- **Files (docs):** `docs/design/metaharness-protocol-v0.1.md` (last amendment `### Amendment a25` at
  line 2106, file ends 2135; a26 appends); `docs/design/runs-side-by-side-v0.1.md` § 1 (line 39;
  header note at 5–8 lists a15–a17); `CHANGELOG.md` (no `## [Unreleased]` heading exists; top entry
  `## [0.9.3]` at line 6, so the heading is added) — cited
- **Symbols called, not changed:** `metaharness::protocol::{project_document, TraceIrDocument,
  EventLine, EventStream, RunId}` (re-export `metaharness/src/lib.rs:69`;
  `metaharness-protocol/src/lib.rs:81–85`); `metaharness_codex::RolloutReader` (`lib.rs:71`,
  `rollout.rs:48`); `metaharness_b10x::B10xSeams::{new, build}` (`seam.rs:26,35`);
  `metaharness::ClaudeSeams` (`metaharness/src/lib.rs:103`), whose `SeamFactory::build` wraps
  `TranscriptReader::new(..).with_seam(..)` (`metaharness-claude/src/bridge.rs:35`) behind
  `HarnessSeam::push_line/finish` (`metaharness-protocol/src/seam.rs:19,22`) — cited
- **Manifests:** `crates/metaharness-aep/Cargo.toml` and `Cargo.lock` — inferred, conditional:
  `metaharness-codex` and `metaharness-b10x` are already `[dependencies]` (Cargo.toml:10–11) and the
  Claude reader is reachable through `metaharness::ClaudeSeams` with no manifest change; both files
  change only if the target names `metaharness_claude::TranscriptReader` directly, and then
  `Cargo.lock` must follow because `.github/workflows/gate.yml:91` runs `cargo build --workspace
  --locked` and `Taskfile.yml:12` runs `cargo run --locked`
- **Also:** `README.md:81` ("The tests execute 45 generated conformance scenarios") — inferred, no
  test guards it, goes stale when the count moves
- **Adapter crates:** no file changes; every reader API used is `pub`, and Out of scope plus Done
  when 6 forbid changing readers or `crates/metaharness-b10x/fixtures/golden/loop.jsonl` — cited
- **Reads only (committed inputs):** `evals/aep/runs/{decomposer-clean,plan-reviewer-clean}.events.jsonl`
  (16, 14 lines); `crates/metaharness-claude/fixtures/golden/transcript.jsonl` (6 lines, expected 8);
  `crates/metaharness-codex/fixtures/golden/rollout.jsonl` (19, expected 23);
  `crates/metaharness-b10x/fixtures/golden/loop.jsonl` (11, expected 10) — cited
- **Confidence:** high — the story cites every file and line and the tree confirms them; the only
  fork (manifests) is an implementor's choice, not an unknown
- **Implementor's confirmation (wave of 2026-10-20), corrections kept visible:** every cited line
  confirmed except `spec/coverage.md:8` (wrong: no count there, it names the suite format; line 19
  updated, line 121 is historical 0.9.0 text left as such); manifests `crates/metaharness-aep/Cargo.toml`
  and `Cargo.lock` confirmed not needed (the Claude reader is reached through
  `metaharness::ClaudeSeams`, `bridge.rs:35-37`; both files untouched); `README.md:81` confirmed and
  updated; adapter crates, protocol crate, core, fixtures and `evals/` confirmed untouched (diff
  against `d7fad09` empty); the golden fixtures' line counts confirmed by `wc -l`; the
  `#[cfg(test)]` mount confirmed. One file was added that the scope did not list:
  `crates/metaharness-aep/tests/adversary_trace_ir_domain.rs` (the adversary's 7 regression cases,
  kept as part of the unit).
- **Would collide with:** any unit touching `spec/`; the `execute` match, count gate or required
  list in `ess_conformance.rs`; the amendment tail of `metaharness-protocol-v0.1.md`;
  `CHANGELOG.md` Unreleased; `runs-side-by-side-v0.1.md` § 1. Known neighbours are sequenced after
  by `depends_on`: `story:project-reads-vendor-transcripts` and `story:b10x-adapter-on-harness-over-llm`.
  A unit re-capturing any golden fixture or regenerating `evals/aep/runs/*.events.jsonl` changes
  this unit's observed census
- **Safety fact:** `ess_conformance.rs` is mounted `#[cfg(test)]` (`crates/metaharness-aep/src/drive.rs:2708–2710`),
  so no shipped binary or library changes; new `execute` arms are additive and the existing 45
  scenarios keep their arms; drift between spec, suite and target fails closed through
  `ess_suite_matches_the_current_specification`, the count gate and `other => panic!` at line 668

## Ambiguities

- `inferable` — the ESS syntax for each declaration: mirror `spec/domains/session.yaml` and
  `observations.yaml` (evidence enums as inputs, outcomes with `when` guards, immutable `Recorded`
  entities with `read_your_writes` views).
- `inferable` — whether ESS 0.56.0 generates one scenario per outcome for a command with twelve
  outcomes: observed on the existing suite (`PollSteering`, five outcomes, five scenarios).

## Open Questions

None that block the unit. The family set itself is owned upstream (`epic:session-format` Out of scope).
