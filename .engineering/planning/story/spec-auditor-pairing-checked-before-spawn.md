---
format: aep.planning-md/3
id: story:spec-auditor-pairing-checked-before-spawn
kind: story
status: implemented
title: The expectation-document and auditor pairing is checked before the spawn
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/metaharness-cli/src/lib.rs
- confidence: cited
  path: crates/metaharness/src/audit.rs
- confidence: cited
  path: crates/metaharness/src/auditor.rs
- confidence: cited
  path: crates/metaharness/src/run.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: cited
  path: website/docs/reference/cli.md
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T20:32:26Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-21T20:32:26Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-22T00:04:37Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
# Story: the expectation-document and auditor pairing is checked before the spawn

## Scheduling

Wave 7 unit 2 (`approval-record:adr-0096-wave-7`), from the operator's user trial of release 0.12.0 on
2026-10-21 (finding 01; the evidence is local to the operator and not in this repository).

## Outcome

`metaharness run` refuses, with exit 2 and before any harness process or provider request, every
combination of `--spec`, `--auditor` and the audit request whose expectation document would not be
checked by the external auditor.

## Context

- `website/docs/reference/cli.md:158-160`: "A `--spec` with no `--auditor` is a refusal, not a skip."
- The pairing is checked only inside `Run::audit` (`crates/metaharness/src/audit.rs:1243-1262`, through
  `crate::auditor::run_auditor`), which runs after the session has ended; `verdict` in
  `crates/metaharness-cli/src/lib.rs:503-506` skips `audit` entirely when the run does not want one
  (`wants_audit`, `audit.rs:1212`).
- Observed on 0.12.0: `run --spec <file>` with no `--auditor` and no `--audit` completed the session and
  exited 0; with `--audit` it exited 2 with the auditor refusal after one provider request.

## Decision (coordinator, 2026-10-21)

Checked in the library's `start`, before the scratch tree, any proxy or the spawn, so an embedder gets the
same refusal as the CLI (design D11). Refused there, each with the name the audit-time refusal already uses:
`--spec` without `--auditor`; `--auditor` without `--spec`; `--spec` or `--auditor` on a run that does not
want an audit (no `--audit` and not `--hermetic strict`); an expectation document that is not a readable
regular file. The audit-time checks stay.

## Acceptance

- expected_before: `run --spec <file>` with no `--auditor` spawns the harness and exits 0; with `--audit`
  it exits 2 only after the session. After: each case above exits 2 with no harness process started,
  shown by a test whose fake harness records whether it was launched.
- `run --audit --spec <file> --auditor <prefix>` behaves as before.
- `website/docs/reference/cli.md` § Audit and amendment a43 of `docs/design/metaharness-protocol-v0.1.md`
  state the pre-spawn refusals; `CHANGELOG.md` `[Unreleased]` carries one line under `### Fixed`.

## Scope

`crates/metaharness/src/run.rs` or the `start` path it uses, `crates/metaharness/src/auditor.rs`,
`crates/metaharness/src/audit.rs`, `crates/metaharness-cli/src/lib.rs`, their tests,
`website/docs/reference/cli.md`, `docs/design/metaharness-protocol-v0.1.md`, `CHANGELOG.md`.
