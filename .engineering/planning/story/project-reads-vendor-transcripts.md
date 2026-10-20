---
format: aep.planning-md/3
id: story:project-reads-vendor-transcripts
kind: story
status: implemented
title: metaharness project normalizes a vendor record that did not pass through this driver
summary: A Claude Code stream-json, Codex rollout or harness --json record recorded outside a run becomes a trace-ir/1 document through the production readers.
owner: metaharness
tags:
- trace
refs:
- provider: atlas
  reference: adr-0096
relations:
- decomposes: epic:session-format
- depends_on: story:trace-ir-domain
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: inferred
  path: README.md
- confidence: cited
  path: crates/metaharness-aep/src/drive/ess_conformance.rs
- confidence: cited
  path: crates/metaharness-cli/src/lib.rs
- confidence: cited
  path: crates/metaharness-cli/tests/project.rs
- confidence: inferred
  path: crates/metaharness-protocol/src/trace_ir.rs
- confidence: inferred
  path: crates/metaharness/src/refusal.rs
- confidence: cited
  path: crates/metaharness/src/viewer.rs
- confidence: inferred
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: cited
  path: docs/design/runs-side-by-side-v0.1.md
- confidence: cited
  path: spec/conformance.json
- confidence: inferred
  path: spec/coverage.md
- confidence: cited
  path: spec/domains/trace.yaml
revision: 31
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T05:04:35Z", actor: "human:timo", revision: 28, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "proposed", to: "active", at: "2026-10-20T05:04:35Z", actor: "human:timo", revision: 29, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "active", to: "implemented", at: "2026-10-20T08:35:02Z", actor: "human:timo", revision: 31, decided_on: {"recorded":{"test_result":1,"review_outcome":9,"verification":1,"ess_conformance_coverage_v1":1}}}
---
# Story: `metaharness project` normalizes a vendor record that did not pass through this driver

## Scheduling

Implemented as unit 2 of wave 2 of 2026-10-20; follows `story:trace-ir-domain`.

## Outcome

A Claude Code `stream-json` transcript, a Codex rollout or a `b10x-harness --json` record recorded
outside a `metaharness run` is normalized into a `trace-ir/1` document by `metaharness project`, through
the same production readers a driven run uses, so ADR 0096 point 7 ("Claude Code, Codex and harness
transcripts are normalized into `trace-ir/1`") holds for a transcript metaharness did not produce.

## Context

- `metaharness project <events.jsonl>` reads a `metaharness.event/1` stream and nothing else
  (`docs/design/runs-side-by-side-v0.1.md` § 1.1). `story:trace-ir-reader` put a foreign transcript
  out of scope by name: "a Claude Code JSONL that did not pass through this driver is not this story".
- The pass exists as a test, not a verb: `crates/metaharness-claude/tests/recorded_runs.rs` drives
  `TranscriptReader` over `evals/aep/checks/transcripts/*-clean.jsonl` and commits the derived streams.
  A replay is not a launch: its attestation claims nothing (`HermeticAttestation::none(HermeticMode::Off)`,
  `Seam::None`), and the verb must carry that distinction into the document.
- `story:trace-ir-domain` declares the four source kinds in `spec/domains/trace.yaml`; this story
  gives three of them a command-line entry.
- The Claude Code *session* JSONL under `~/.claude/projects/` is a different format from
  `stream-json`; its adapter is ADR 0088's "session-JSONL adapter" under `epic:session-mining`, not this story.

## Acceptance

`metaharness project --from <claude-code-stream-json|codex-rollout|harness-json> <file>` over each
of the three golden records exits 0 and writes a `trace-ir/1` document whose node census
(`metaharness.families`) equals the census of the adapter's own expected stream for that record,
where today the verb exits 2 on anything but a `metaharness.event/1` stream.

## Done when

1. The verb above exists and the three documents are byte-stable across two runs.
2. An unknown or mis-declared `--from` is refused by name with exit 2.
3. The document's `hermetic` block claims nothing (`HermeticMode::Off`, no impositions), and
   `metaharness.source` carries whatever the Ambiguities answer below settles.
4. `spec/domains/trace.yaml` gains one conformance scenario per vendor source for this verb, and the
   count gate in `ess_conformance.rs` moves by that number.

## Out of scope

- The session-JSONL adapter (ADR 0088; `epic:session-mining`).
- Redaction. A document projected from an operator's real transcript is evidence and may contain
  anything the vendor wrote (AGENTS.md § Safety envelope); this verb neither redacts nor commits.

## Scope

- inferred: `crates/metaharness-cli/src/lib.rs` (the `project` verb's arguments),
  `crates/metaharness/src/viewer.rs` or the module that owns `project`, `spec/domains/trace.yaml`,
  `crates/metaharness-aep/src/drive/ess_conformance.rs`, `docs/design/runs-side-by-side-v0.1.md`.
- Shared file, split by symbol: in `crates/metaharness-cli/src/lib.rs` this story owns the `project`
  verb's definition and nothing else; `story:caller-supplied-catalogue-on-every-kind`,
  `story:continuity-across-executor-calls` and `story:aep-drive-disposition` own the `run` and `aep`
  verbs there, and `story:metaharness-tools-on-tools-crates` owns the owned-surface server
  construction at line 300. Not in one wave with any of them unless the brief declares the split by
  line range.

## Ambiguities

- `requires-stakeholder-input` — whether `metaharness.source` for a replayed vendor record should be
  the vendor format name or stay `metaharness.event/1` with the vendor named in `vendor_transcript`.
  Default: the vendor format name, because P3 says the digest names the file the document was made from.
  **Settled in the default's favour by amendment a28 (wave 2 of 2026-10-20).**

## Implementor's confirmation (wave 2 of 2026-10-20), corrections kept visible

Every scoped file confirmed and edited except as noted: `crates/metaharness/src/viewer.rs` owns
`project_file` and gained the reader-backed entry, `VendorSource` and the replay; `trace_ir.rs`
needed `TraceIrDocument::project_from(lines, bytes, source, source_lines)` (the source was a
constant inside `project_document`); `refusal.rs` gained `Refusal::ProjectionSource`; `README.md:69,81`
confirmed; `spec/coverage.md:21` and the obligation row confirmed. Edited outside the list: the
re-export line of `crates/metaharness/src/lib.rs` (unavoidable) and one identifier on the shared
`use` line of `crates/metaharness-cli/src/lib.rs`. Done-when 4's "moves by that number" was wrong:
the count moved by 4, not 3 (a default refusal outcome keeps the three guards auditable). Added by
the adversary: `crates/metaharness-cli/tests/adversary_project_from.rs` (6 cases kept). Decided by
the coordinator on the adversary's findings: `vendor_transcript.path` is the file name only;
`source_line` on a replay is the record line the reader read the event from; same-named `--html`
columns carry a digest suffix. The website `cli.md` rows that said `project` refuses with exit 2
(stale since a15) were corrected by the coordinator at close.
