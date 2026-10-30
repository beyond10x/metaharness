---
format: aep.planning-md/3
id: story:claude-prompt-cache-ttl-declared
kind: story
status: active
title: A caller declares the Claude prompt-cache TTL, and each usage record says which class the writes took
summary: --prompt-cache-ttl 5m|1h sets CLAUDE_CODE_PROMPT_CACHE_TTL; usage carries cache_creation_5m/1h_input_tokens
owner: metaharness
tags:
- prompt-cache
relations:
- decomposes: epic:loom-agent-work-through-metaharness
scope:
- confidence: cited
  path: crates/metaharness-claude/src/launch.rs
- confidence: cited
  path: crates/metaharness-protocol/src/event.rs
- confidence: cited
  path: crates/metaharness-protocol/src/spec.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T17:48:54Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-30T17:48:55Z", actor: "human:timo", revision: 6}
---
# Story: a caller declares the Claude prompt-cache TTL, and each usage record says which class the writes took

## Scheduling

Filed 2026-10-30 for trial finding C5, the open question the trial analyst left to metaharness:
"whether the 1-hour TTL can be set to 5 minutes for print sessions". One wave with the other C5
stories, integration branch `wave/2026-10-30-c5`.

## Outcome

A caller can declare the prompt-cache TTL a Claude run writes with (`RunSpec::prompt_cache_ttl`,
`--prompt-cache-ttl 5m|1h`). The launch sets `CLAUDE_CODE_PROMPT_CACHE_TTL` to that value in the
child's constructed environment; without a declaration nothing is set and the vendor's automatic
choice holds (one hour on a subscription login, five minutes on an API key). `codex` and `b10x`
refuse the flag by name before the spawn, as they refuse other flags they cannot carry.

The `usage` event carries the split of the cache writes by class, read from the vendor's own
`usage.cache_creation` object: `cache_creation_5m_input_tokens` and
`cache_creation_1h_input_tokens`, absent when the vendor does not say. So whether the declared class
was applied is read from the run's record (invariant 3), not from the flag.

## Why

The trial's Claude steps wrote every cache entry in the one-hour class (610,704 tokens
`ephemeral_1h`, 0 `ephemeral_5m`, report 2 item 6), which is the dearer write for sessions under a
minute; the steps of one case ran 7 to 11 s apart (p1-1, p2-1, p4-1, p5-1 step timestamps). A host
whose steps are minutes apart may still want the hour, so the class is the caller's to declare and
the default does not move.

Read from the binaries, not driven: 2.1.309 (the pinned binary, SHA-256 `4d704a8a…b658`) and
2.1.314 both choose the main-thread TTL from `CLAUDE_CODE_PROMPT_CACHE_TTL` before the settings and
the automatic choice (`FORCE_PROMPT_CACHING_5M` first, then that variable for the `sdk` query
source). The per-class split is in the stream-json `usage.cache_creation` object of every assistant
message in the trial transcripts (`ephemeral_5m_input_tokens`, `ephemeral_1h_input_tokens`).

## Acceptance

- `crates/metaharness-claude/src/launch.rs`, unit tests:
  - `a_declared_prompt_cache_ttl_reaches_the_child_environment`: `5m` and `1h` each plan
    `CLAUDE_CODE_PROMPT_CACHE_TTL` with that value.
  - `no_declared_ttl_sets_no_ttl_variable`, and an inherited `CLAUDE_CODE_PROMPT_CACHE_TTL` or
    `FORCE_PROMPT_CACHING_5M` does not reach the child.
- `crates/metaharness/src/builder.rs` (or wherever `check_spec` refusals are tested):
  `codex_and_b10x_refuse_a_prompt_cache_ttl_by_name`.
- The Claude usage mapping's tests: `cache_creation_classes_are_carried_from_the_vendor_usage`
  (a vendor line with `ephemeral_5m_input_tokens: 685` and `ephemeral_1h_input_tokens: 0` maps to
  those two fields) and `a_usage_without_cache_creation_leaves_both_classes_absent`.
- Existing golden event fixtures are unchanged where the vendor line has no `cache_creation`
  object (the new fields are omitted when absent).
- Live, once, in the closing report: a `--prompt-cache-ttl 5m` run's usage event reads
  `cache_creation_1h_input_tokens: 0`.

## Out of scope

- Changing the default class.
- The trace-ir projection of the split (a later story if a reader asks for it).
- Codex's and the b10x loop's cache classes.

## Scope

- cited: `crates/metaharness-protocol/src/spec.rs`: `struct RunSpec`, one new field
  `prompt_cache_ttl`, and its per-kind refusal
- cited: `crates/metaharness-protocol/src/event.rs`: `struct Usage` (line 287), two new optional
  fields
- cited: `crates/metaharness-claude/src/launch.rs`: `fn build_env` (about line 1505) and its tests
- inferred: the Claude usage mapping (`crates/metaharness-claude/src/session.rs` or `seam.rs`)
- inferred: `docs/design/metaharness-protocol-v0.1.md` (one amendment for the wire fields)
- Shares `spec.rs` and the CLI with `story:scratch-root-stable-by-declared-name`, and `build_env`
  with `story:claude-child-runs-without-auto-memory`: each adds its own lines, nothing else.
