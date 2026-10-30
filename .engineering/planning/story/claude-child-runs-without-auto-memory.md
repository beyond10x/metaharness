---
format: aep.planning-md/3
id: story:claude-child-runs-without-auto-memory
kind: story
status: active
title: A Claude child runs with auto-memory off, so its system prompt names no per-run path
summary: CLAUDE_CODE_DISABLE_AUTO_MEMORY=1 in the constructed environment; the memory path under the scratch config home breaks the prompt cache between sessions
owner: metaharness
tags:
- prompt-cache
relations:
- decomposes: epic:loom-agent-work-through-metaharness
scope:
- confidence: cited
  path: crates/metaharness-claude/src/launch.rs
- confidence: cited
  path: crates/metaharness-claude/tests/native_fixture.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T17:48:53Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-30T17:48:53Z", actor: "human:timo", revision: 5}
---
# Story: a Claude child runs with auto-memory off, so its system prompt names no per-run path

## Scheduling

Filed 2026-10-30 for trial finding C5 (prompt ordering so the provider prompt cache holds), the
metaharness part that Loom's `story:session-executor-stable-copy-path` names: metaharness's own
per-session paths for the child. One wave with the other C5 stories, integration branch
`wave/2026-10-30-c5`.

## Outcome

Two Claude sessions that metaharness launches in one working directory send the same system
prompt, so the second session's first request reads the first one's prompt from the provider cache.
Today the system prompt carries Claude Code's auto-memory section, which names a directory under
the run's scratch config home (`CLAUDE_CONFIG_DIR=<scratch>/claude-home`). The scratch root is a new
temporary directory per run, so that one line differs between every two sessions and the cache
breaks there. The memory directory is deleted with the scratch when the run ends, so nothing a
session writes there is ever read again. The launch sets `CLAUDE_CODE_DISABLE_AUTO_MEMORY=1` in the
child's constructed environment, and the attestation says it did.

## Evidence

- Observed 2026-10-30 against Claude Code 2.1.314 with `metaharness 0.24.1 run claude
  --credentials none --model-endpoint <local recorder> --cwd <one directory>`, two sessions: the
  two first requests are byte-identical in all 14 tools and in every message block, and differ in
  `system[2]` only, at the line `You have a persistent file-based memory at
  <scratch>/claude-home/projects/<cwd>/memory/` (`.tmpKz0UUY` vs `.tmptnlRjW`).
- Observed 2026-10-30 with the operator's login, same cwd, two sessions 20 s apart: the second
  session's first request read 13,287 tokens from the cache and wrote 5,674 (cached share 0.701).
- Read from the binaries (not driven): 2.1.309 (SHA-256 `4d704a8a…b658`, the pinned binary the
  2.1.309 research page records) and 2.1.314 both read `process.env.CLAUDE_CODE_DISABLE_AUTO_MEMORY`
  and return `"off"` when it is truthy.

## Acceptance

- `crates/metaharness-claude/src/launch.rs`, unit tests:
  - `the_child_runs_with_auto_memory_off_under_every_credential_source`: the planned environment
    holds `CLAUDE_CODE_DISABLE_AUTO_MEMORY=1` under `operator-login`, `api-key`, `loopback` and
    `none`; today it holds no such key.
  - `an_inherited_auto_memory_switch_does_not_reach_the_child`: an operator environment with
    `CLAUDE_CODE_DISABLE_AUTO_MEMORY=0` still plans `1`.
- `crates/metaharness-claude/tests/native_fixture.rs`, the native lane (run against the pinned
  2.1.309 binary by explicit path, in the loopback-only namespace, as the 2.1.309 research page
  describes):
  - `native_two_sessions_in_one_cwd_send_one_system_prompt`: the stub records both sessions' first
    request; every `system` block is byte-identical, and no request contains the run's scratch
    root.
- The measured number goes in the closing report: the second session's cached share with the
  operator's login, before and after, by the same command.

## Out of scope

- A stable scratch root for every kind: `story:scratch-root-stable-by-declared-name`.
- The cache TTL class: `story:claude-prompt-cache-ttl-declared`.
- Moving `PINNED_VERSIONS`.

## Scope

- cited: `crates/metaharness-claude/src/launch.rs`: `fn build_env` (about line 1505), the H-row
  attestation in `fn attest`, and their unit tests
- cited: `crates/metaharness-claude/tests/native_fixture.rs` (one new case)
- inferred: `docs/design/metaharness-protocol-v0.1.md` (one amendment line on the constructed
  environment)
