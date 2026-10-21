---
format: aep.planning-md/3
id: story:first-user-documents-agree-with-cli
kind: story
status: draft
title: The first-user documents agree with the released CLI
revision: 1
---
# Story: the first-user documents agree with the released CLI

## Scheduling

Draft for wave 8, from the operator's user trial of release 0.12.0 on 2026-10-21 (finding 05; the
evidence is local to the operator and not in this repository).

## Outcome

A first user who follows the published pages in order builds, installs and runs the release without a
step the pages do not state, and no page contradicts the CLI's behaviour.

## Context (observed on 0.12.0 source documents)

- `website/docs/status.md:56`, `:59`, `:84` call `project` unbuilt or refused; the released CLI projects.
- `website/docs/quickstart.md:17` builds locally, then `:38` and `:61` call bare `metaharness` with no
  install or `PATH` step; `:92` asks to list the workspace's crates with no `--cwd`, so the run starts in
  an empty scratch directory.
- `website/docs/control-seam.md:70` and `:119` contradict the Codex observe and allow behaviour.
- `website/docs/protocol/frames.md:14` describes frames changing per turn while `frame.set` is refused.
- Counts drift: conformance vector totals, number of CLI verbs, gate prerequisites.

## Acceptance

- expected_before: each line above as cited. After: each line states the released behaviour, with the
  test, command output or source line it was checked against cited in the change.
- Every count on those pages is either generated or checked by a test.

## Scope

`website/docs/status.md`, `website/docs/quickstart.md`, `website/docs/control-seam.md`,
`website/docs/protocol/frames.md`, any page carrying a drifting count, `metaharness-docs` checks,
`CHANGELOG.md`.
