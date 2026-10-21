---
format: aep.planning-md/3
id: story:retire-trace-formats-beside-trace-ir
kind: story
status: active
title: The trace formats beside trace-ir/1 are projected or retired
summary: Each trace format ADR 0096 lists besides trace-ir/1 becomes a projection of it or stops being written, one story per repository.
owner: metaharness
refs:
- provider: atlas
  reference: adr-0096
relations:
- decomposes: epic:session-format
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/metaharness-aep/src/drive/ess_conformance.rs
- confidence: cited
  path: crates/metaharness-cli/src/lib.rs
- confidence: cited
  path: crates/metaharness-protocol/src/lib.rs
- confidence: cited
  path: crates/metaharness-protocol/src/trace_summary.rs
- confidence: cited
  path: crates/metaharness/src/viewer.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: cited
  path: spec/domains/trace.yaml
- confidence: cited
  path: spec/monetary/observations.yaml
- confidence: cited
  path: website/docs/reference/cli.md
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T19:41:36Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-21T19:41:36Z", actor: "human:timo", revision: 4}
---
# Story: the trace formats beside trace-ir/1 are projected or retired

## Scheduling

Draft, filed 2026-10-21 at the close of wave 5 on the orchestrator's word. Each repository's move is its own
story in that repository; this one tracks the set from metaharness, which owns the session format. Wave 7
(`approval-record:adr-0096-wave-7`) makes the per-row decision below and builds the one projection the
surviving formats read. The per-repository moves are handed to the orchestrator as draft story bodies; this
story stays `active` until each row cites its repository's story and commit.

## Outcome

Every trace format ADR 0096 lists besides `trace-ir/1` is either a projection of `trace-ir/1` or no longer
written, per repository.

## Context

- ADR 0096 (`atlas/architecture/adr/0096-the-agent-stack-has-one-owner-per-layer.md`, atlas `242f11a2`):
  line 40 lists five trace formats (harness `--json`, metaharness `trace-ir/1`, `takt-trace/1`, Loom's jsonl,
  an external session IR); decision 7 (lines 71-73) says metaharness owns the session format, that Claude
  Code, Codex and harness transcripts are normalized into `trace-ir/1`, and that other formats become
  projections of it or are retired; line 86 leaves open which survive as projections. That choice is
  metaharness's under decision 7; it is made here and changes no ADR.
- Who still writes each, by grep across the sibling checkouts on 2026-10-21:

| format | repository | where (revision) |
|---|---|---|
| harness `--json` | harness | `crates/harness-cli/src/lib.rs:1024-1026` (the flag), `:1052` (stdout under `--json` is the event record), `:476` (a command that rewrites a `--json` loop record as `metaharness.event/1`) (harness `bb9da693`) |
| `takt-trace/1` | takt | `crates/takt/src/trace.rs:30` (`FORMAT`), `crates/takt-cli/src/main.rs:63` (takt `a587509`, branch `wave/29`) |
| Loom's jsonl | loom | `loom run --output jsonl`: `crates/loom-cli/src/main.rs:476-484` (loom `9e63c11b`) |
| external session IR | unknown | not named in ADR 0096 or ADR 0088 (`0088-session-mining-lives-in-metaharness.md` names `trace-ir/1` at lines 22 and 29); no match for "session IR" in harness, takt, loom, metaharness, agentplugins or aep |

## Decisions (wave 7, 2026-10-21)

| format | decision | reason |
|---|---|---|
| harness `--json` | stays, as a **source**; the `harness events` rewrite (`lib.rs:476`) is retired | It is the L1 harness's own transcript, the third of the transcripts decision 7 normalizes into `trace-ir/1`, beside Claude Code's stream-json and Codex's rollout: `metaharness project --from harness-json` (amendment a28) already reads it through `metaharness-b10x`'s production reader. The `harness events` command writes a second normalization of the same record into `metaharness.event/1`; metaharness's reader is the one decision 7 names, and no metaharness crate calls the harness command (grep 2026-10-21). |
| `takt-trace/1` | survives, as a **projection** for its harness steps | It is takt's case-level record, and most of it is takt's own (validator, forge, takt, human and idle operations, case header, acceptance). Its harness-step rows (model, tokens, cost, turns, durations, outcome, tool calls) are today read from Claude's `--output-format json` result and a runner shim's `steps.jsonl` (`crates/takt/src/trace.rs`); they become a projection of the step's `trace-ir/1` document through `trace-ir-summary/1`. |
| Loom's jsonl | survives, as a **projection** for agent work | It is Loom's L3 run record (route, approval, terminal, and the turns of Loom's own model calls), and Loom's events module calls it "a projection of what the run does" (`crates/loom-cli/src/events.rs:6-7`, loom `9e63c11b`). For agent work run through metaharness it stops re-encoding the session: it names the `trace-ir/1` document by digest and carries its `trace-ir-summary/1`. |
| external session IR | **closed**: not a format anyone writes | Grep on 2026-10-21 across every checkout under the workspace for `session IR`, `session-ir` and `sessionir`: no writer, no reader, no specification. The one other `trace-ir/1` declaration, AEP's `crates/observe/trace-domain/src/ir.rs:44` (`IR_FORMAT = "trace-ir/1"`, aep `70b3179c`), is the same format, not a sixth one. |

The two surviving formats read one declared projection, `trace-ir-summary/1`, specified in
`spec/domains/trace.yaml` and produced by `metaharness project --to trace-ir-summary`. One projection
rather than a takt-shaped and a Loom-shaped one: takt and Loom own their formats (ADR 0096, one owner per
layer), so metaharness declares what a session says and each consumer maps it into its own record. The
consumers reach it through the `metaharness` binary, not a crate (`AGENTS.md` § *What this repository
owns*: binary first, no repository takes a crate of this workspace as a dependency), so their moves need a
metaharness release that carries the flag, not a crate publication.

## Acceptance

- expected_before: today harness, takt and loom each write their own trace format, none produced as a
  projection of `trace-ir/1`. After: for each row above, the owning repository either produces the format
  as a projection of `trace-ir/1` or no longer writes it, and the row cites that repository's story and
  commit.
- The external session IR row names an owner or is closed as not a format anyone writes.
- `trace-ir-summary/1` is a declared projection in `spec/domains/trace.yaml`, `ess specify validate`
  passes, and a conformance scenario per recorded source (Claude Code, Codex, harness) runs the production
  projection in `task check`.

## Scope

metaharness: `spec/domains/trace.yaml`, `crates/metaharness-protocol` (the projection),
`crates/metaharness-cli` (`project --to trace-ir-summary`), `crates/metaharness-aep/src/drive/ess_conformance.rs`
(the conformance target), `docs/design/metaharness-protocol-v0.1.md` (the amendment), `CHANGELOG.md`, and
this story's record. The moves are in harness, takt and loom.
