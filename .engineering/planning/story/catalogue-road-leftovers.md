---
format: aep.planning-md/3
id: story:catalogue-road-leftovers
kind: story
status: active
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
scope:
- confidence: inferred
  path: crates/metaharness-tools/Cargo.toml
- confidence: inferred
  path: crates/metaharness-tools/src/caller.rs
- confidence: inferred
  path: crates/metaharness-tools/src/server.rs
- confidence: inferred
  path: crates/metaharness/src/run.rs
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-05T03:32:35Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-05T03:32:36Z", actor: "human:timo", revision: 6}
---
# Story: catalogue road leftovers

## Scheduling

Draft, filed 2026-10-21 at the close of wave 5 from `review-result:adr0096-w5-independent-review` (s8-3,
s8-4, s7-5 with its duplicate s10-4, s7-3), `review-result:adr0096-w5-catalogue-adversary-2` (finding 5) and
the deviation reported by unit 4's fix round 2.

## Outcome

The caller-catalogue road records what it decided under every decision mode, names its own cause, pins its
format crate by revision, and does not block on a catalogue path swapped for a FIFO.

## Context

Read again at `9192a90` when the story was accepted for the 0.29.0 wave, and narrowed to what is still open.
The type check (fix round 2's deviation) shipped: `tools/call` answers `isError` and performs nothing when
an argument value is not of the declared type (CHANGELOG), so it is out of this story.

- Ask mode: under `DecisionMode::Ask` a call is put to the embedder (`crates/metaharness/src/run.rs`, the
  `ask` branch at about line 910) before anything says it resolved to no catalogue entry. The embedder may
  allow it; the server then refuses, but the record says allowed (s8-3).
- Wording: a call outside the catalogue is denied with the vocabulary reason and warning, not one naming the
  run's catalogue (s8-4).
- Pin: `b10x-tool-contract` is pinned by `tag = "0.5.0"` of `beyond10x/tools`
  (`crates/metaharness-tools/Cargo.toml:16`), not by `rev` as invariant 10 pins harness (s7-5, s10-4).
- Race: `crates/metaharness-tools/src/caller.rs` checks the path's metadata, then opens it with a plain
  `File::open` (about lines 550-560). A FIFO swapped in between makes the open block before the post-open
  re-check can refuse it (s7-3; adversary pass 2, finding 5).

Each point is from reading. The implementor shows each one red on base before changing it.

## Acceptance

- expected_before: today a catalogue run under `--decisions ask` can record an allow for a tool the catalogue
  never offered; a catalogue miss is denied with vocabulary wording; the manifest pins tool-contract by tag;
  a FIFO raced in blocks the open. After: the ask branch records a deny naming the catalogue for a call that
  resolves to no entry, and the embedder is not asked about it; the deny and warning name the run's
  catalogue; the manifest pins tool-contract by `rev` (the commit the lock already holds, so only the lock's source
  string changes); the open does not block on a FIFO and refuses it by name. Each is pinned by a test that is red
  on base.

## Scope

Inferred: `crates/metaharness/src/run.rs`, `crates/metaharness-tools/src/caller.rs`,
`crates/metaharness-tools/src/server.rs`, `crates/metaharness-tools/Cargo.toml`, `CHANGELOG.md`.

## Withdrawn

The wave lead withdrew the `rev` pin of `b10x-tool-contract` from the 0.29.0 wave (2026-11-05):

- Harness pulls `beyond10x/tools` by `tag = "0.5.0"` through its own dependencies.
- So a `rev` pin in `crates/metaharness-tools/Cargo.toml` adds a second `b10x-tool-contract` package to the
  lock instead of changing one source string.
- Cargo refuses a `[patch]` that points at the same source.
- The lock already fixes the commit (`99e36ed`).

The pin follows when harness pins tools by `rev`, under `epic:layer-pins-follow-their-owners`. The
implementor's two patches are kept outside the repository.
