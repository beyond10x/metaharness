---
format: aep.planning-md/3
id: story:retire-trace-formats-beside-trace-ir
kind: story
status: draft
title: The trace formats beside trace-ir/1 are projected or retired
summary: Each trace format ADR 0096 lists besides trace-ir/1 becomes a projection of it or stops being written, one story per repository.
owner: metaharness
refs:
- provider: atlas
  reference: adr-0096
relations:
- decomposes: epic:session-format
revision: 1
---
# Story: the trace formats beside trace-ir/1 are projected or retired

## Scheduling

Draft, filed 2026-10-21 at the close of wave 5 on the orchestrator's word. Each repository's move is its own
story in that repository; this one tracks the set from metaharness, which owns the session format.

## Outcome

Every trace format ADR 0096 lists besides `trace-ir/1` is either a projection of `trace-ir/1` or no longer
written, per repository.

## Context

- ADR 0096 (`atlas/architecture/adr/0096-the-agent-stack-has-one-owner-per-layer.md`, atlas `242f11a2`):
  line 40 lists five trace formats (harness `--json`, metaharness `trace-ir/1`, `takt-trace/1`, Loom's jsonl,
  an external session IR); decision 7 (lines 71-73) says metaharness owns the session format and other
  formats become projections of it or are retired; line 86 leaves open which survive as projections.
- Who still writes each, by grep across the sibling checkouts on 2026-10-21:

| format | repository | where (revision) |
|---|---|---|
| harness `--json` | harness | `crates/harness-cli/src/lib.rs:1024-1026` (the flag), `:1052` (stdout under `--json` is the event record), `:476` (a command that rewrites a `--json` loop record as `metaharness.event/1`) (harness `bb9da693`) |
| `takt-trace/1` | takt | `crates/takt/src/trace.rs:30` (`FORMAT`), `crates/takt-cli/src/main.rs:63` (takt `a587509`, branch `wave/29`) |
| Loom's jsonl | loom | `loom run --output jsonl`: `crates/loom-cli/src/main.rs:476-484` (loom `9e63c11b`) |
| external session IR | unknown | not named in ADR 0096 or ADR 0088 (`0088-session-mining-lives-in-metaharness.md` names `trace-ir/1` at lines 22 and 29); no match for "session IR" in harness, takt, loom, metaharness, agentplugins or aep |

## Acceptance

- expected_before: today harness, takt and loom each write their own trace format, none produced as a
  projection of `trace-ir/1`. After: for each row above, the owning repository either produces the format
  as a projection of `trace-ir/1` or no longer writes it, and the row cites that repository's story and
  commit.
- The external session IR row names an owner or is closed as not a format anyone writes.

## Scope

metaharness: `spec/` (the `trace-ir/1` domain) and this story's record only; the moves are in harness, takt
and loom.
