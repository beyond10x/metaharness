---
format: aep.planning-md/3
id: story:hook-program-quotes-channel-paths
kind: story
status: draft
title: The generated hook program quotes its channel paths
relations:
- decomposes: epic:loom-agent-work-through-metaharness
revision: 1
---
# Story: the generated hook program quotes its channel paths

## Scheduling

Draft, filed 2026-10-21 from the repository review of that date (finding 3). Wave 5 candidate.

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
