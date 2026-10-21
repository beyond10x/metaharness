---
format: aep.planning-md/3
id: story:several-tool-contract-families-per-session
kind: story
status: draft
title: A session can be given more than one tool-contract family
summary: A run given catalogues of different families offers their union; unscheduled.
owner: metaharness
refs:
- provider: atlas
  reference: adr-0098
relations:
- decomposes: epic:loom-agent-work-through-metaharness
- informed_by: review-result:adr0096-w5-catalogue-adversary-1
revision: 1
---
# Story: a session can be given more than one tool-contract family

## Scheduling

Draft, unscheduled. Filed 2026-10-21 at the close of wave 5 from `review-result:adr0096-w5-catalogue-adversary-1`
(finding 7); the orchestrator routed that finding to a story. Whether Loom needs it is not established.

## Outcome

A run can be given catalogues of more than one `tool-contract/1` family (for example `fs.*` reads and
`code.*` writes) in one session.

## Context

- The tool-contract validator requires every operation name in a document to start with that document's
  family (`src/validate.rs` in `beyond10x/tool-contract` at `60d4d0e`, the revision the lock pins), so one
  document is one family.
- `--catalogue` takes one file (`crates/metaharness-tools/src/caller.rs`, read at `:148` in the reviewed tree),
  so a session today gets one family.

## Acceptance

- expected_before: today a run given two catalogues of different families cannot be expressed (one
  `--catalogue` file, one family per document), so such a session is refused. After: a run given two
  catalogues of different families offers the union of both families' entries, and a duplicate rendering
  across them is refused by name before the spawn.

## Scope

Inferred: `crates/metaharness-tools/src/caller.rs`, `crates/metaharness/src/builder.rs`,
`crates/metaharness-cli/src/lib.rs`, the a40 section of `docs/design/metaharness-protocol-v0.1.md`.
