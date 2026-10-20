---
format: aep.planning-md/3
id: epic:session-format
kind: epic
status: draft
title: 'metaharness owns the session format: trace-ir/1 as an ESS domain'
summary: trace-ir/1, the form every recorded agent session is normalized into, specified in spec/ and checked by conformance like the other domains (Atlas ADR 0096 point 7).
owner: metaharness
tags:
- spec
- trace
refs:
- provider: atlas
  reference: adr-0096
revision: 1
---
## Status

Draft, recorded 2026-10-20 from Atlas ADR 0096 point 7 (owner's decisions of 2026-10-20: metaharness
is the one harness interface, L2, and owns the session format). Its first story is proposed for the
wave of 2026-10-20; the second is a later draft.

## Outcome

`trace-ir/1`, the form every recorded agent session is normalized into, is specified as an ESS domain
in this repository's `spec/` and checked by the same conformance machinery as the five existing
domains. Claude Code, Codex and harness transcripts reach it through this repository's adapters, and
the mapping from the `metaharness.event/1` wire into it is a declared, executable contract rather than
a table in a design page.

## Context

- The document form exists in code: `crates/metaharness-protocol/src/trace_ir.rs` writes a
  `TraceIrDocument` (`format`, `transcript_digest`, `adapter`, `events`, `requests`, `metaharness`)
  from a `metaharness.event/1` stream; `crates/metaharness-protocol/src/projection.rs` holds the ten
  `IrFamily` variants and the nine `CONTROL_PLANE_EVENTS`. The verb is `metaharness project`
  (`story:trace-ir-reader`, implemented).
- The decisions behind it are `docs/design/runs-side-by-side-v0.1.md` § 1 (P1–P4, the twenty-row
  event → family table) and `docs/design/metaharness-protocol-v0.1.md` § 4.4 (D1, D6, D6a,
  amendments a15 and a17).
- `spec/system.yaml` declares five domains (`metaharness.session`, `.workspace`, `.spending`,
  `.boundaries`, `.observations`) under `ess/15`; `spec/conformance.json` holds 45 generated scenarios
  executed against production code by `crates/metaharness-aep/src/drive/ess_conformance.rs`
  (count asserted at line 874). None of the five names the trace document.
- The three vendor records are read by `metaharness-claude/src/transcript.rs` (stream-json),
  `metaharness-codex/src/rollout.rs` (the rollout) and `metaharness-b10x/src/seam.rs`
  (`b10x-harness --json`), each into the same wire. Each adapter carries a committed golden record
  and its expected stream under `fixtures/golden/`.

## Parts

1. `story:trace-ir-domain` — the domain `metaharness.trace` in `spec/domains/trace.yaml`, with
   conformance executed against the production projection and the production readers. The wave.
2. `story:project-reads-vendor-transcripts` — `metaharness project` accepts a vendor record that did
   not pass through this driver. Later.

## Out of scope

- Session mining (redaction, episodes, the session-JSONL adapter): `epic:session-mining`, ADR 0088.
- Which of the five trace formats ADR 0096 lists survive as projections of `trace-ir/1`: open in
  ADR 0096, decided in Atlas, not here.
- Changing the family set of `trace-ir/1`. The Rust definition consumed by `aep observe trace` is
  still `trace-domain` in beyond10x/aep, pinned here at rev `6d7a44d3607d2d9a6ffdf0a165993c546c43d0db`
  (`crates/metaharness-aep/Cargo.toml:20`); ADR 0078 point 1 moves that crate into this repository
  and has not been executed. This epic specifies the document form metaharness writes; moving the
  crate is a question for the operator, not a story here.

## Done when

`spec/system.yaml` declares `metaharness.trace`; `task check` executes its generated scenarios
against production code with none skipped, unsupported or refused; and `metaharness project` reads
each of the three vendor records directly.
