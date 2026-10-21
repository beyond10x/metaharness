---
format: aep.planning-md/3
id: story:hook-program-quotes-channel-paths
kind: story
status: active
title: The generated hook program quotes its channel paths
relations:
- decomposes: epic:loom-agent-work-through-metaharness
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: crates/metaharness-claude/src/hook.rs
- confidence: inferred
  path: crates/metaharness-claude/src/launch.rs
- confidence: cited
  path: crates/metaharness-codex/src/hook.rs
- confidence: inferred
  path: crates/metaharness-codex/src/launch.rs
- confidence: inferred
  path: crates/metaharness/src/builder.rs
- confidence: inferred
  path: crates/metaharness/src/refusal.rs
- confidence: cited
  path: crates/metaharness/src/spawn_codex_vectors.rs
- confidence: cited
  path: crates/metaharness/src/spawn_vectors.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T12:00:25Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-21T12:00:25Z", actor: "human:timo", revision: 6}
---
# Story: the generated hook program quotes its channel paths

## Scheduling

Wave 5 unit 2 (`approval-record:adr-0096-wave-5`). Decided by the coordinator (D1): option A, POSIX `'\''`
quoting at every site, plus the non-UTF-8 refusal from option C. Condition from `b10x-chef-orchestrator`
(2026-10-21): the report names which callers can reach the non-UTF-8 refusal.

## Outcome

A run whose scratch root contains a `'` (or any byte the shell treats specially) gets a PreToolUse hook that
still parses and still reaches metaharness, or is refused by name before spawn; the hook never fails to
parse.

## Context

- `crates/metaharness-claude/src/hook.rs:155` and `crates/metaharness-codex/src/hook.rs:130` render
  `REQUESTS='{requests}'` and `RESPONSES='{responses}'` into a `/bin/sh` program with the path's display
  form between single quotes, unescaped.
- The root comes from the run's scratch directory (`HookChannelPaths::at_root`, callers in
  `crates/metaharness/src/builder.rs:1293`, `:1377`, `spawn.rs:127`, `spawn_codex.rs:102`), which follows
  the caller's temp or output directory. A `'` there ends the quoted string early.
- Not measured: what each vendor does with a hook that exits on a shell syntax error. If a non-zero exit
  other than the deny code lets the tool call proceed, the guard fails open; this is the first thing to
  measure.

## Acceptance

- A test renders the hook for a root containing `'`, `"`, `$`, a backtick, a space and a newline, runs it
  under `/bin/sh -n` and then against a fake channel, and the request arrives at the expected path.
- Alternatively a root that cannot be quoted is refused before spawn with a named error, with a test.
- Both kinds (Claude, Codex) are covered by the same case.
## Scope

Derived 2026-10-21 by `story-scoper` at `main` `5df311b`.

- cited: `crates/metaharness-claude/src/hook.rs:141-188` (the render at `:155`) and
  `crates/metaharness-codex/src/hook.rs:116-163` (`:130`); the paths are rendered with `PathBuf::display()`.
- cited: the root comes only from `TMPDIR`, through `tempfile::TempDir::new` at `crates/metaharness/src/builder.rs:847`
  (Claude) and `:1128` (Codex). The Context section's "output directory" does not hold.
- observed (bash as `/bin/sh`, cut-down program): one `'` denies every call with exit 0; a `'` pair around a
  space, `;` or `$(...)` runs the text as commands; `'(` is a syntax error, exit 2; an unbound variable at top
  level exits 127.
- cited: exit 2 denies on both vendors (design doc `:298`, Codex research page `:42`). Other non-zero exits:
  Claude open question Q4 (design doc `:1094`, `:1790`); Codex not recorded. Fail-open is not established.
- cited: a second unquoted site, the vendor hook `command` (Claude `launch.rs:1572-1581`, Codex
  `launch.rs:1139-1153`); the Codex TOML quoting (`launch.rs:1335`) does not escape control characters.
- cited: no golden vector or ESS scenario pins the bytes. Tests: both `hook.rs` unit tests;
  `crates/metaharness/src/spawn_vectors.rs:51-88`, `spawn_codex_vectors.rs:63-71`.
- Files: `crates/metaharness-claude/src/hook.rs`, `crates/metaharness-codex/src/hook.rs`, both `launch.rs`,
  `crates/metaharness/src/builder.rs` and `refusal.rs` (the non-UTF-8 refusal), `spawn_vectors.rs`,
  `spawn_codex_vectors.rs`, `CHANGELOG.md`, a design amendment.
- Size: S to M. Shares Claude `launch.rs` with unit 1 (other regions); sequential after it.
