---
format: aep.planning-md/3
id: story:hook-program-quotes-channel-paths
kind: story
status: implemented
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
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T12:00:25Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-21T12:00:25Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-10-21T18:43:46Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":3,"ess_conformance_coverage_v1":1}}}
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

## Delivered

Commit `aff22dc`, merged `6ad241c` into `integration/adr-0096-wave-5`; two `aep:adversary` passes
(`review-result:adr0096-w5-hook-quoting-adversary-1`, NEEDS-CHANGE, corrected in one round;
`review-result:adr0096-w5-hook-quoting-adversary-2`, PASS, 87 test binaries, 1031 passed, 0 failed, 49
ignored). Decision D1, amendment a37: the Claude and Codex hook programs and the vendor hook `command` carry
their paths as POSIX single-quoted words (`shell_quote`, `shell_quote_path`); the Codex `config.toml`
strings escape every control character by TOML basic-string rules, parsed back in a test with the `toml`
crate (dev-dependency of `metaharness-codex` only); the Codex C1 fixtures move by two bytes each. A scratch
root that is not UTF-8 is refused on every kind right after the scratch directory is created
(`scratch-path-not-utf8`, exit 2). A Claude scratch root containing `${CLAUDE_PLUGIN_ROOT}` or
`${CLAUDE_PLUGIN_DATA}` is refused before the spawn (`scratch-path-names-plugin-variable`, exit 2; pass 1
F1). The orchestrator's condition (which callers can reach the non-UTF-8 refusal) was reported; the root
comes only from `TMPDIR` (`crates/metaharness/src/builder.rs:847`, `:1128` at the scoping base). Adversary cases kept in `crates/metaharness-claude/tests/adversary_w5u2.rs`,
`crates/metaharness-codex/tests/adversary_w5u2.rs`, `crates/metaharness/tests/adversary_w5u2.rs` and
`crates/metaharness-cli/tests/adversary_w5u2_adv2.rs`; the coordinator corrected a37 (the fish sentence,
the `--mcp-config` expansion scope) and the CHANGELOG Added line after pass 2.

Whole gate at the wave-6 head: task check, one step at a time, on `integration/adr-0096-wave-6` at
`5ddebbd`: 6 steps exit 0, 96 test binaries, 1128 passed, 0 failed, 49 ignored, conformance 103/103 on ESS
0.67.0.

Deviations: operator paths (`--cwd`, `--hooks`, `--driver`, `--plugin-dir`) are still rendered through the
display form; a37 scopes its claim to paths under the scratch root. The shell Codex runs its hook command
with is unverified (a37 § 1). One pre-existing warning from pass 2 went outside this unit and is withheld
until release 0.12.0.
