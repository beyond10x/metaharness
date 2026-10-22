---
format: aep.planning-md/3
id: story:first-user-documents-agree-with-cli
kind: story
status: implemented
title: The first-user documents agree with the released CLI
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/metaharness-cli/tests
- confidence: cited
  path: crates/metaharness-docs/src/lib.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: cited
  path: website/docs/control-seam.md
- confidence: cited
  path: website/docs/frames.md
- confidence: cited
  path: website/docs/harnesses
- confidence: cited
  path: website/docs/quickstart.md
- confidence: cited
  path: website/docs/reference/cli.md
- confidence: cited
  path: website/docs/status.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-22T00:51:01Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-22T00:51:01Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-22T07:46:31Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
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

Derived 2026-10-22 by `story-scoper` at `a34551e` (release 0.13.0); decisions D8–D10 by the coordinator.
Runs after every other wave-8 unit has merged, so counts and pages include their changes.

- The story's line references are to 0.12.0. At `a34551e`: `website/docs/status.md:61` (`project` row,
  released since 0.13.0, `crates/metaharness-cli/src/lib.rs:242`, `:266`), `:64`, `:89`;
  `website/docs/quickstart.md:17` (local build), bare `metaharness` at `:38`, `:46`, `:66-67`, `:77-78`,
  `:97` with no install or `PATH` step, no-`--cwd` run at `:97`, gate prerequisites at `:20-23`, vector
  counts at `:88-89`, exit table at `:143-150`; `website/docs/control-seam.md:70-72`, `:119-120`;
  `website/docs/frames.md:14-16` (the story's `protocol/frames.md` does not exist).
- Drifting counts: vectors (`quickstart.md:88-89` 28/21; `harnesses/claude.md:15`, `harnesses/codex.md:13`,
  `status.md:22` 30/23; `harnesses/b10x.md:72` "seven" against `status.md:22` 8); verbs
  (`website/docs/reference/cli.md:13` "Six verbs" against 8 `Verb` variants at `lib.rs:44-75`,
  `anti_drift.rs:107-118` checking six); gate prerequisites (`Taskfile.yml:7-8,12`, ESS 0.67.0).
- Hand-typed counts in `crates/metaharness-docs/src/lib.rs:217-222`, `:230-236`.
- Tests: a new `crates/metaharness-cli/tests/website_claims.rs` checking each count against its source
  (`conformance_vectors(kind).len()`, `Cli::command()`, `metaharness-docs` `PAGES`); `anti_drift.rs` updated.
- `CHANGELOG.md` under `[Unreleased]`.

Decisions:

- D8 Codex observe and allow (`control-seam.md`): the design document governs (invariant 7). Where a
  recorded run in `docs/research/` shows the behaviour, an amendment (a49) records it with that citation
  and the page follows; where none does, the page says unverified, as the design document does.
- D9 `reference/cli.md` lists all eight verbs, `mcp-serve` marked as started by a client rather than typed;
  the test checks the table against `Cli::command()`.
- D10 `frames.md` states what the release does: `frame.set` is refused (`harnesses/claude.md:99`,
  `crates/metaharness-claude/src/seam.rs:318`).
