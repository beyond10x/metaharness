---
format: aep.planning-md/3
id: epic:session-mining
kind: epic
status: draft
title: Mine recorded agent sessions into composition and protocol proposals
tags:
- trace
refs:
- provider: atlas
  reference: adr-0088
revision: 1
---
## Status

Draft, recorded 2026-10-04 from Atlas ADR 0088 (session mining lives in Metaharness). Not scheduled;
it stays in `draft` until the operator opens it.

## Outcome

A session miner reads recorded agent sessions (a main session, its peers and its sub-agents) and
proposes a `composition/1` diff (Atlas ADR 0087) plus protocol diffs, mapped onto library protocols.

Pipeline: redact → `trace-ir/1` → episodes (trigger → outcome) → semantic actions (the runtime
binding reversed) → library match with a deviation report (Canon evaluation) → proposal →
`canon diff` against the floor and `canon check` → `protocol.adopt/1` (Atlas ADR 0086).

## Parts in Metaharness

- A session-JSONL adapter into `trace-ir/1` (today's adapters read `stream-json` and
  `metaharness.event/1`).
- Redaction before ingestion: secrets, personal data, customer content.
- Episode segmentation and reverse binding (tool call → semantic action).
- A library matcher with a deviation report, using Canon evaluation.

## Boundaries (ADR 0088)

- Mining proposes designs only; nothing mined changes a case's claims (Atlas ADR 0078).
- Behaviour the library cannot express becomes a candidate library fragment, never an invented claim.
- Outputs carry action names, counts, timings and episode references, never transcript text.

## Before decomposition

Episode, semantic action and proposal are new nouns: their ESS domain comes before any story.
Open (ADR 0088): the composition diff format; `operator.consult` in the ELS vocabulary.
