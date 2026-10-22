---
format: aep.planning-md/3
id: story:exit-status-separates-task-control-audit
kind: story
status: active
title: The run's exit status documents the task, control and audit verdicts separately
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/metaharness-cli/tests
- confidence: cited
  path: crates/metaharness/src/audit.rs
- confidence: cited
  path: crates/metaharness/tests/audit.rs
- confidence: cited
  path: crates/metaharness/tests/stream_closed.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: cited
  path: spec/conformance.json
- confidence: cited
  path: spec/domains/session.yaml
- confidence: cited
  path: website/docs/protocol/events.md
- confidence: cited
  path: website/docs/reference/cli.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-22T00:51:00Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-22T00:51:00Z", actor: "human:timo", revision: 5}
---
# Story: The run's exit status documents the task, control and audit verdicts separately

## Scheduling

Draft for wave 8, from the operator's maintainer campaign on release 0.12.0 on 2026-10-21 (finding
F6; the evidence is local to the operator and not in this repository).

## Outcome

A caller can tell from documented output, without reading vendor records, whether the harness child
exited 0, whether every control held, and whether the hermetic audit was conclusive.

## Context

- Observed: audited real sessions exited 3 with child exit 0 and correct task effects, because rows
  H1a, H4 and H5 read unknown. Automation that treats every nonzero exit as task failure reads these
  as failed tasks.
- Unknown rows stay unknown (invariant 3); this story documents and exposes the verdicts, and does not
  turn configured isolation into a pass.

## Acceptance

- expected_before: the CLI reference names exit 3 without saying that the child may have exited 0.
  After: the CLI reference and the audit output name the child exit, the control verdict and the
  audit verdict as separate fields, and a test holds the page to the output.
- An audit with an unknown row still exits nonzero.

## Scope

Derived 2026-10-22 by `story-scoper` at `a34551e` (release 0.13.0); decisions D11–D12 by the coordinator.

- Primary surface: `crates/metaharness/src/audit.rs`. `AuditReport` (`:151-191`) gains the child exit;
  `AuditReport::render` (`:240-347`) prints child exit, control verdict and audit verdict as separate
  lines; `Run::audit` (`:1270-1279`) fills the child exit from the last `stream.closed` `process`
  (`crates/metaharness/src/run.rs:1296`; `ProcessTermination` at `crates/metaharness-protocol/src/event.rs:26-38`).
- `AuditReport::exit` (`:217-236`) is unchanged: it never reads `process`, so no exit code moves, and an
  audit with an `unk` gating row still exits nonzero (`tests/audit.rs:1668-1679`).
- Tests: `crates/metaharness/tests/audit.rs`, `crates/metaharness/tests/stream_closed.rs` (struct literals
  and render assertions), and a new test in `crates/metaharness-cli/tests/` that holds
  `website/docs/reference/cli.md` to the rendered output (precedent `live_scope_documented.rs`).
- Documents: `website/docs/reference/cli.md:289-296` (exit table); `website/docs/protocol/events.md:47`
  (`stream.closed` row names `process`); amendment a48 to `docs/design/metaharness-protocol-v0.1.md` § 9.4
  (`:1553`, table `:1609-1614`).
- ESS: `spec/domains/session.yaml:169-172` maps the process exit status that is UNMAPPED today, with a named
  conformance scenario and its production arm; `spec/conformance.json` regenerated (ESS 0.67.0).
- `CHANGELOG.md` under `[Unreleased]`.
- Not in scope: `website/docs/quickstart.md:143-150` (its exit table belongs to
  `story:first-user-documents-agree-with-cli`, which runs after this one).

Decisions:

- D11 the conformance scenario is part of this story.
- D12 control verdict = the gating rows folded `gap` > `unk` > `ok`; audit verdict = the `RunExit` (`ok`,
  `gap`, `no verdict`); a child exit that was not measured renders `unknown`, never 0.
