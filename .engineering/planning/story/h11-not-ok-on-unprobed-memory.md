---
format: aep.planning-md/3
id: story:h11-not-ok-on-unprobed-memory
kind: story
status: active
title: A Claude H11 is not ok when memory sits in a surface the canary never probed
summary: 'From the U3 adversary pass of the 0.29.0 wave: the walk finds memory above the parent or in deep rules and H11 still reads ok'
tags:
- from-finding
relations:
- derived_from: review-result:wave-0290-u3-adversary-1
- depends_on: story:claude-memory-canary-deeper-ancestors
scope:
- confidence: inferred
  path: crates/metaharness-claude/src/lib.rs
- confidence: inferred
  path: crates/metaharness/src/audit.rs
- confidence: inferred
  path: crates/metaharness/src/builder.rs
- confidence: inferred
  path: docs/design/metaharness-protocol-v0.1.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-05T11:09:21Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-05T11:09:21Z", actor: "human:timo", revision: 3}
---
## Scheduling

Draft, filed 2026-11-05 from `review-result:wave-0290-u3-adversary-1`, from its out-of-scope finding. The
finding predates the 0.29.0 wave and is not part of it. This is an a36 design question.

## Outcome

A Claude H11 does not read `ok` when the memory walk finds a memory file in a surface the canary never
probed. Those surfaces are ancestors above the cwd's parent, `.claude/rules` below the rules directory's
first level, and user-level rules in the config home. Either the row says what was found there, or the
canary probes it.

## Context

- `crates/metaharness/src/builder.rs` (about line 3278, at the U3 head `5fbb9af`): H11 stays `ok` when the
  walk finds a memory file in a surface the canary never probed. Since U3, the `ok` detail names those
  surfaces as unprobed, but the verdict does not change.
- The unprobed list U3 added does not name user-level `.claude/rules` in the config home.
- Inferred from reading. Not driven against a real binary.

## Acceptance

- expected_before: a run whose scratch tree has a `CLAUDE.md` above the cwd's parent reads H11 `ok`.
- After: that run's H11 reads `gap` or `unk`, naming the file and the unprobed surface. Alternatively, the
  canary measures that surface per pinned version, and the row records the measurement.
- a36 states which of the two was chosen.
