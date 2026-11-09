---
format: aep.planning-md/3
id: story:loom-run-adapter
kind: story
status: active
title: metaharness run loom observes a b10x-loom run on the event wire
summary: 'Issue 28: a metaharness-loom adapter beside metaharness-b10x; the default-kind change stays out'
tags:
- from-issue
refs:
- provider: github
  reference: beyond10x/metaharness#28
relations:
- decomposes: epic:loom-agent-work-through-metaharness
- decomposes: epic:github-zero-2026-11-09
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/metaharness-cli/tests/adversary_w8u5.rs
- confidence: inferred
  path: crates/metaharness-cli/tests/loom_pin.rs
- confidence: cited
  path: crates/metaharness-cli/tests/website_claims.rs
- confidence: cited
  path: crates/metaharness-docs/src/content.rs
- confidence: cited
  path: crates/metaharness-loom
- confidence: cited
  path: crates/metaharness-protocol/src/spec.rs
- confidence: cited
  path: crates/metaharness-protocol/src/spec_document.rs
- confidence: cited
  path: crates/metaharness/Cargo.toml
- confidence: cited
  path: crates/metaharness/fixtures/golden/contract-result-loom.json
- confidence: cited
  path: crates/metaharness/src/builder.rs
- confidence: cited
  path: crates/metaharness/src/custody.rs
- confidence: cited
  path: crates/metaharness/src/doctor.rs
- confidence: cited
  path: crates/metaharness/src/lib.rs
- confidence: cited
  path: crates/metaharness/src/qualify.rs
- confidence: inferred
  path: crates/metaharness/src/run.rs
- confidence: cited
  path: crates/metaharness/src/vectors.rs
- confidence: cited
  path: crates/metaharness/tests/contract_golden.rs
- confidence: cited
  path: crates/metaharness/tests/contract_symmetry.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: inferred
  path: spec/conformance.json
- confidence: inferred
  path: spec/coverage.md
- confidence: inferred
  path: spec/domains/trace.yaml
- confidence: cited
  path: website/README.md
- confidence: cited
  path: website/docs/control-seam.md
- confidence: cited
  path: website/docs/frames.md
- confidence: cited
  path: website/docs/harnesses/loom.md
- confidence: cited
  path: website/docs/quickstart.md
- confidence: cited
  path: website/docs/reference/cli.md
- confidence: cited
  path: website/docs/status.md
revision: 39
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T22:08:16Z", actor: "human:timo", revision: 38}
- {from: "proposed", to: "active", at: "2026-11-09T22:08:16Z", actor: "human:timo", revision: 39}
---
## Scheduling

Draft, filed 2026-11-04 from the idle audit of open GitHub issues:
https://github.com/beyond10x/metaharness/issues/28 (opened 2026-10-05, no store artifact until now).

## Outcome

`metaharness run loom` drives a `b10x-loom run` session and emits `metaharness.event/1` and accepts
the same steering commands as the other kinds, so a consumer of the event stream sees no protocol
change. `metaharness-b10x` stays as it is and selectable.

## Context

- The issue's dependency is met: Loom `story:run-event-stream` is implemented; `b10x-loom run --output
  jsonl` writes the versioned `intake.events.RunEventLine` stream (loom `crates/loom-cli/src/events.rs`,
  read at loom `origin/main` 2026-11-04).
- The issue asks for the Loom adapter to be the native default. Changing which kind is the default
  changes what every caller of `metaharness run` gets; that part is a design decision (invariant 8)
  and is not in this story's acceptance.
- AGENTS.md invariant 2: everything Loom-specific lives in a new `metaharness-loom` crate.
- ADR 0098 points the other way for governance (Loom's session executor runs metaharness sessions);
  this adapter observes a Loom run and is not per-call governance.

## Acceptance

Amended 2026-11-09 by the coordinator of `epic:github-zero-2026-11-09`, from the scoper's reading of
Loom `origin/main` `3cb82b7c` (`ess/intake/domains/events.yaml`): the `RunEvent` union is `Route`,
`Turn`, `ToolCall`, `Usage`, `Approval`, `Session`, `Terminal`. It has no tool-result record and no
session-start record, so the first draft's "one `tool.result` per Loom tool call" has no source
line. The first draft's item 1 is replaced; items 2 and 3 are unchanged; 4 and 5 are added.

1. A recorded `b10x-loom run --output jsonl` stream of a pinned Loom release (the fixture names the
   release and how it was captured; no paid run in the gate) replays through `metaharness-loom` into
   a `metaharness.event/1` stream with one `session.started`, one `tool.requested` per Loom
   `ToolCall` record carrying `decision_required: false` and no seam, and one `session.ended` that
   carries the `Terminal` record's stop reason and exit status; a golden expected stream pins it. No
   `tool.result` is emitted, because the stream carries none, and the adapter says so: the design
   amendment and the harness page state it as unobserved, never as "no result".
2. `metaharness run loom --strict-version` refuses an off-pin `b10x-loom` binary before the spawn.
3. `metaharness run b10x` is unchanged; `task check` exits 0.
4. A line whose `schema_version` the adapter does not know is refused by name, and a stream that
   ends without a `Terminal` record ends the session as failed, not as completed.
5. The design document carries the amendment before the code (invariant 8): the mapping per record
   kind, what `loom` declares for each adapter obligation, and what stays unobserved. No Cargo
   dependency on Loom: Loom depends on metaharness, so the line format is read as JSON.

## Out of scope

- Making `loom` the default native kind (a design amendment first).
- Steering a Loom run per tool call.

## Scope

Derived 2026-11-09 by `aep:story-scoper` at `bff6a66`; confidence medium. The first draft's
inferred scope named `crates/metaharness-cli/src/lib.rs` as the kind list; the kind list is
`Kind` in `crates/metaharness-protocol/src/spec.rs:19` (it derives `clap::ValueEnum`).

- cited: new `crates/metaharness-loom/` (source, recorded stream, golden stream, tests).
- cited: `crates/metaharness-protocol/src/spec.rs:19-43` (`Kind`, `Kind::as_str`),
  `crates/metaharness-protocol/src/spec_document.rs:93,107-111` (`KINDS`, the `MissingKind` text).
- cited: `Cargo.toml` members, `crates/metaharness/Cargo.toml`, `Cargo.lock`.
- cited: exhaustive matches on `Kind` in `crates/metaharness/src/builder.rs`, `vectors.rs`,
  `doctor.rs`, `qualify.rs`, `custody.rs`, and `ADAPTERS` in `crates/metaharness/src/lib.rs:140`.
- cited: tests that enumerate every kind: `crates/metaharness/tests/contract_symmetry.rs`,
  `contract_golden.rs` with a new `fixtures/golden/contract-result-loom.json`,
  `crates/metaharness-cli/tests/website_claims.rs`, `adversary_w8u5.rs`.
- cited: `docs/design/metaharness-protocol-v0.1.md` (a new amendment), `website/docs/harnesses/loom.md`,
  `crates/metaharness-docs/src/content.rs`, `website/README.md`, `website/docs/status.md`,
  `quickstart.md`, `control-seam.md`, `frames.md`, `reference/cli.md`, `CHANGELOG.md`, `README.md`.
- inferred: `crates/metaharness/src/run.rs` (`== Kind::B10x` conditions), hand-written three-kind
  lists in other tests, `spec/domains/trace.yaml` (adapter-id enum) with `spec/conformance.json` and
  `spec/coverage.md`, a new `crates/metaharness-cli/tests/loom_pin.rs`.
- Collides with any unit touching `builder.rs`, `vectors.rs`, `spec.rs`, the amendment tail of the
  design document, `CHANGELOG.md` or the website claim pages.
- Not established by the scoper: which record `session.started` derives from, the pin mechanism,
  and no recorded stream of a pinned Loom release exists in either repository yet.
