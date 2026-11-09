---
format: aep.planning-md/3
id: story:loom-native-default
kind: story
status: draft
title: A native run executes on Loom by default
summary: 'Issue 28, second ask: held until the operator decides what the default names'
tags:
- from-issue
refs:
- provider: github
  reference: beyond10x/metaharness#28
relations:
- decomposes: epic:github-zero-2026-11-09
- depends_on: story:loom-run-adapter
revision: 1
---
## Context

GitHub issue 28, <https://github.com/beyond10x/metaharness/issues/28>, asks for three things: a
Loom adapter crate, that adapter as "the native default", and `metaharness-b10x` kept selectable.
`story:loom-run-adapter` carries the adapter and keeps `metaharness-b10x`. This story carries the
default, which that story put out of scope "(a design amendment first)".

Read at `origin/main` `bff6a66`:

- `metaharness run <kind>` has no default kind; the kind is a required argument
  (`crates/metaharness-protocol/src/spec.rs:19`, `Kind`).
- The native arm of a driven run is the kind a step map names: `B10X_HARNESS = "b10x"`
  (`crates/metaharness-aep/src/drive.rs:1618`), matched per `llm` step (`drive.rs:2026`). The step
  maps live in AEP, so which loop `b10x` means is read by another repository.
- The issue wants a consumer that selects the `b10x` kind to run on Loom "without changing its own
  code". With no default kind to move, that means either the name `b10x` starts to mean Loom, or
  the consumer's step maps change. Coordinator's reading, not stated in the issue.
- AGENTS.md: "Changing bytes another repository verifies requires a coordinated migration and an
  ADR", and invariant 8, "A design decision is written before it is built."
- `decision-blocker:aep-drive-future` was cleared with option A: `metaharness aep drive` stays the
  governed-run driver until Loom's session executor runs a case.

## Outcome

A native run executes on Loom by default, on a decision that says what "default" names and who
migrates.

## Acceptance

`decision-blocker:loom-native-default` is cleared by the operator's recorded answer; the design
document carries the amendment; the code follows it; issue 28 is closed with the merge that does.
