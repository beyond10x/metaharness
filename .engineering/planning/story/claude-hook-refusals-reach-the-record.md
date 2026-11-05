---
format: aep.planning-md/3
id: story:claude-hook-refusals-reach-the-record
kind: story
status: active
title: Refusals made by the Claude hook program reach the record
summary: A call the hook program refuses on its own is recorded as denied with its cause, never as allowed.
owner: metaharness
refs:
- provider: atlas
  reference: adr-0096
relations:
- informed_by: review-result:adr0096-w6u1-review
- informed_by: story:claude-seam-fails-closed
scope:
- confidence: inferred
  path: crates/metaharness-claude/src/hook.rs
- confidence: inferred
  path: crates/metaharness/src/run.rs
- confidence: inferred
  path: crates/metaharness/src/spawn.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-05T03:32:35Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-05T03:32:35Z", actor: "human:timo", revision: 3}
---
# Story: refusals made by the Claude hook program reach the record

## Scheduling

Draft, filed 2026-10-21 after release 0.12.0 from `review-result:adr0096-w6u1-review` (s6-3).

## Outcome

A call the Claude hook program refuses on its own is recorded as denied, with the program's cause, and no
`tool.decided` for that call says allow.

## Context

- The hook program refuses a call itself when it has no channel, when the backstop runs out and when the
  answer is unreadable (`crates/metaharness-claude/src/hook.rs:117-140`, at `cacb923`).
- None of these refusals reaches metaharness; `tool.decided` is emitted from metaharness's own decision
  (`crates/metaharness/src/spawn.rs:142-180`, `:447-462`; `crates/metaharness/src/run.rs:1114-1133`, at
  `cacb923`), so the record can say allow for a call the vendor refused. Emission order verified by
  reading.

## Acceptance

- expected_before: today a call the hook program refuses is recorded with metaharness's decision, which can
  be allow. After: for each of the three program refusals a test shows the record holds a deny for that call
  naming the cause, and no allow for it.

## Scope

Inferred: `crates/metaharness-claude/src/hook.rs`, `crates/metaharness/src/spawn.rs`,
`crates/metaharness/src/run.rs`, `CHANGELOG.md`.
