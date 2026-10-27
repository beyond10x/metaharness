---
format: aep.planning-md/3
id: story:qualify-a-binary-without-a-model-call
kind: story
status: active
title: metaharness reports a harness binary's floor rows without a model call
summary: qualify(kind, program, hermetic) from --version only
tags:
- priority:high
scope:
- confidence: inferred
  path: crates/metaharness-claude/src/launch.rs
- confidence: inferred
  path: crates/metaharness-claude/src/lib.rs
- confidence: inferred
  path: crates/metaharness-codex/src/launch.rs
- confidence: inferred
  path: crates/metaharness-codex/src/lib.rs
- confidence: inferred
  path: crates/metaharness/src/lib.rs
- confidence: inferred
  path: crates/metaharness/src/qualify.rs
- confidence: inferred
  path: crates/metaharness/tests/qualify.rs
- confidence: inferred
  path: spec/system.yaml
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-27T12:13:55Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "proposed", to: "active", at: "2026-10-27T12:13:55Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":4}}}
---
# Story: qualify-a-binary-without-a-model-call

## Outcome

A caller can ask metaharness, before any session, whether a harness binary would pass the hermetic floor. The
answer comes with no model call: `metaharness::qualify(kind, program, hermetic)` resolves `program` as the child's
`PATH` would, runs its `--version`, and returns each floor row the launch would claim for that version, with its
verdict and whether it gates under `hermetic`. Loom's session executor and takt can then refuse an unqualified binary
before they spend a session on it.

## Context

- On a downstream trial (2026-10-27), takt drove Claude Code 2.1.311 under `HermeticMode::Strict`. The session ran,
  spent 37,234 tokens, and only then was its proposal held for an H11 `gap`: the version has no memory canary.
  2.1.309, already installed, was qualified.
- The pins and canaries the verdict rests on are this crate's: `PINNED_VERSIONS` (`crates/metaharness-claude/src/lib.rs:207`),
  `MEMORY_CANARIES` and `memory_canary` (`lib.rs:447-520`), and the H11 reading "gap on a record version with no
  canary" (`crates/metaharness-claude/src/launch.rs:205-222`).
- `--strict-version` already reads the resolved binary's `--version` without a model call (`lib.rs:87-93`), but only
  for H9 and only as a refusal at spawn.
- A caller that copied the pin table would drift with every vendor release. The answer belongs here.

## Acceptance

Each is a named test run by `task check`.

1. `qualify::an_unpinned_claude_gates_on_h11_under_strict`: a fake `claude` printing `2.1.311 (Claude Code)` gives H9
   `gap` and H11 `gap`, H11 gating under `Strict`. The fake records one `--version` call and no other invocation.
2. `qualify::a_pinned_claude_is_ok`: a fake printing `2.1.309` gives every gating row `ok`.
3. `qualify::the_rows_match_the_launch`: for each entry of `PINNED_VERSIONS` and one unpinned version, the rows
   `qualify` returns equal the rows the launch's attestation claims for the same version and mode.
4. `qualify::a_missing_binary_is_named`: a `program` that resolves to nothing is an error naming the program and the
   `PATH` searched.
5. `qualify::codex_rows_match_its_pins`: a fake `codex` printing a version in the Codex `PINNED_VERSIONS`
   (`crates/metaharness-codex/src/lib.rs:106`) gives H9 `ok`. One printing an unpinned version gives H9 `gap`, gating
   only under `strict_version`, as the Codex launch claims it.

## Scope

- `crates/metaharness/src/qualify.rs` (new: the public `qualify` and its `Qualification`)
- `crates/metaharness/src/lib.rs` (re-export)
- `crates/metaharness-claude/src/lib.rs`, `crates/metaharness-claude/src/launch.rs` (row derivation shared with the launch)
- `crates/metaharness-codex/src/lib.rs`, `crates/metaharness-codex/src/launch.rs` (the same for Codex)
- `crates/metaharness/tests/qualify.rs` (new)
- `spec/system.yaml` (the `Qualification` noun, before code)

## Delivery

The consumer is takt's `story:an-unqualified-harness-stops-before-the-first-session`, through loom-sdk. The chain
metaharness -> loom -> takt is delivered in one workspace:
- loom's and takt's `Cargo.toml` get uncommitted `[patch]` path overrides to this working tree.
- The end-to-end check is a `takt run` with a fake unqualified `claude` that stops before its first model call.
- Then releases go out in chain order as pin-only hops. There is no release of this story before the end-to-end
  check passes.

## Out of scope

- Making H11 advisory.
- Qualifying a new vendor version. That remains the native fixture's measured canary.
