---
format: aep.planning-md/3
id: story:catalogue-road-leftovers
kind: story
status: draft
title: Catalogue road leftovers
summary: Ask-mode record, catalogue-named denial, revision pin, FIFO race and argument value types on the caller-catalogue road.
owner: metaharness
refs:
- provider: atlas
  reference: adr-0098
relations:
- decomposes: epic:loom-agent-work-through-metaharness
- informed_by: review-result:adr0096-w5-independent-review
- informed_by: review-result:adr0096-w5-catalogue-adversary-2
revision: 1
---
# Story: catalogue road leftovers

## Scheduling

Draft, filed 2026-10-21 at the close of wave 5 from `review-result:adr0096-w5-independent-review` (s8-3,
s8-4, s7-5 with its duplicate s10-4, s7-3), `review-result:adr0096-w5-catalogue-adversary-2` (finding 5) and
the deviation reported by unit 4's fix round 2.

## Outcome

The caller-catalogue road records what it decided under every decision mode, names its own cause, pins its
format crate by revision, and checks argument value types.

## Context

- Ask mode: under `DecisionMode::Ask` a call resolving to no catalogue entry goes to the embedder, which may
  allow it; the server then refuses, but the record says allowed (`crates/metaharness/src/run.rs:811`,
  `:851`; s8-3).
- Wording: a call outside the catalogue is denied with the vocabulary reason and warning, not one naming the
  run's catalogue (`run.rs:1018-1035`; s8-4).
- Pin: `b10x-tool-contract` is pinned by `tag = "0.2.0"` (`crates/metaharness-tools/Cargo.toml`), not by
  revision as invariant 10 pins harness; the lock holds `60d4d0e` (s7-5, s10-4).
- Race: a FIFO swapped in between the regular-file check and the open makes the open block
  (`crates/metaharness-tools/src/caller.rs`, the read path; s7-3); the post-open re-check is reachable only
  by that race (adversary pass 2, finding 5).
- Types: the argument check added in fix round 2 checks keys, not value types; a string where a number
  belongs reaches the harness verb, which refuses it (fix round 2 report, deviations).

## Acceptance

- expected_before: today a catalogue run under `--decisions ask` can record an allow for a tool the catalogue
  never offered; a catalogue miss is denied with vocabulary wording; the manifest pins tool-contract by tag;
  a FIFO raced in blocks the open; a wrongly typed argument reaches the verb. After: the ask branch records
  a deny naming the catalogue for a call that resolves to no entry; the deny and warning name the run's
  catalogue; the manifest pins tool-contract by `rev`; the open does not block on a FIFO; a wrongly typed
  argument gets `isError` naming the entry and the field, and nothing runs.

## Scope

Inferred: `crates/metaharness/src/run.rs`, `crates/metaharness-tools/src/caller.rs`,
`crates/metaharness-tools/src/server.rs`, `crates/metaharness-tools/Cargo.toml`, `CHANGELOG.md`.
