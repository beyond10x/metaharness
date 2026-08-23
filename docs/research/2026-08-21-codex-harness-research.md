> Migrated from `engineering-protocols/docs/reviews/`, 2026-08-22, under that repository's
> `epic:metaharness-migration`: this record is the `metaharness-codex` adapter's evidence base
> and belongs beside it. The original stays in the subject repository's history.

# Codex as a second harness — research record, 2026-08-21

Evidence for the wave-4 Codex adapter (W4.4) and for the enforcement layer's portability claim.
Verified against a local **codex-cli 0.145.0** install, 2,437 rollout files (2026-04 → 2026-08),
and official documentation fetched today. Every fact is labelled: **V** verified locally ·
**D** official docs · **I** inferred · **?** unknown. A fact without a label does not belong here.

## The adapter's input: rollout JSONL, not stdout

- `codex exec [PROMPT]` is the `claude -p` analogue; `--json` emits a thread/turn/item JSONL
  stream on stdout — but that stream carries **no timestamps, no durations, no cost**, and usage
  only once per turn. (V, D `exec_events.rs`)
- The richer record is the session rollout:
  `$CODEX_HOME/sessions/YYYY/MM/DD/rollout-<ISO8601>-<uuid7>.jsonl` — written by exec runs too
  (`session_meta.originator="codex_exec"`; 1,101 of 2,437 local files). Every line
  `{timestamp: ISO8601 ms, type, payload}`. (V)
- Mapping onto `trace-ir/1`: `session_meta` → session_start (session_id, cwd, cli_version,
  model_provider, git commit/branch, base_instructions); `custom_tool_call`/`function_call`
  paired to outputs by `call_id` → tool_call/tool_result; `event_msg/token_count` → usage AND
  rate limits (`rate_limits{used_percent, window_minutes, resets_at, plan_type}`);
  `event_msg/task_complete` → run_outcome (`duration_ms`, `time_to_first_token_ms`). (V)
- **No stability guarantee is documented** for the rollout format (?), and drift is already
  observable inside one install: Apr-2026 files use `exec_command_begin/end`, Aug-2026 use
  `custom_tool_call{name:"exec"}` + `patch_apply_end`. An adapter must version-gate on
  `session_meta.cli_version` and treat unknown shapes as opaque (unk, never fail). (V)
- **Cost is never emitted** (zero cost keys across 2,437 files) — derive from tokens × price and
  mark the derivation. (V)
- Recommended flow: `codex exec --json` only to learn the `thread_id`, then read the matching
  rollout file. (I, from the above)

## Enforcement portability: Codex has a real PreToolUse hook

- Hooks are **stable and enabled by default** in 0.145.0 (`codex features list`); config in
  `~/.codex/hooks.json`, `<repo>/.codex/hooks.json` (project wins), or `config.toml`; plugins
  bundle `hooks/hooks.json`. (V+D — third-party posts claiming "experimental/off by default" are
  stale)
- Same decision contract shape as Claude Code: stdout `permissionDecision: "allow"|"deny"` +
  `permissionDecisionReason`, `updatedInput`, `continue`/`stopReason`; **exit 2 + stderr also
  blocks**. stdin carries `tool_name`, `tool_input`, `cwd`, `turn_id`, `transcript_path`. (D + V
  in the binary: `PreToolUseHookSpecificOutputWire`)
- PreToolUse covers shell, `apply_patch`, MCP tools; matchers include `Bash`, `apply_patch`,
  `Edit`, `Write`, `mcp__server__tool`. Event set mirrors Claude Code's
  (SessionStart…PreToolUse…Stop). (D)
- Trust: non-managed hooks need explicit trust (`/hooks`); `--dangerously-bypass-hook-trust`
  exists for automation; org enforcement via `requirements.toml`. (V+D)
- Approval provenance is **recorded per turn**: `turn_context{approval_policy, sandbox_policy,
  permission_profile, model, effort}` — the audit trail for what ran auto-approved. Approval
  modes are `untrusted | on-request | never` (the suggest/auto-edit/full-auto naming is
  obsolete). The approval request/decision *wire shape* is unverified — none of the local
  sessions ran a prompting mode (?). Default: ship the adapter without it; add after one
  `-a untrusted` run captures a real request.

## Skills and instructions reach Codex natively

- AGENTS.md is native: root-to-cwd walk, one file per directory, `AGENTS.override.md` wins,
  concatenated root-first; observed live in rollouts (`world_state.state.agents_md`). (D, V)
- Skills follow the open agent-skills standard at `.agents/skills/` (repo-walked), `$HOME/.agents/skills`,
  `/etc/codex/skills` — `SKILL.md` + YAML frontmatter, `scripts/ references/ assets/`. The
  planning skill's content is portable; its Claude-specific invocation notes are not. (D)

## Other load-bearing facts

- `session_id`/`thread_id` are UUIDv7 (time-sortable); `turn_id` on nearly every payload; rollout
  is append-only with monotonic timestamps, `session_meta` first (enforced — the binary has a
  `session_configured_not_first_event` error). (V/I)
- Error taxonomy for run_outcome: `usage_limit_reached, server_overloaded,
  context_window_exceeded, quota_exceeded, request_timeout, sandbox_denied, interrupted`. (V)
- Live-control alternatives to transcript parsing: `codex mcp-server` (stdio MCP) and
  `codex app-server` (JSON-RPC `thread/start`, `turn/start`). (V)

## Appendix, 2026-08-23: provider, credential and hook-response facts read from the binary

Added while building the codex loopback door (LP-4) and the allow half of the decision wire. The
method is **narrower** than the record above and is labelled as such: these are literals and serde
field lists read out of the pinned `codex` 0.145.0 binary (`strings /usr/bin/codex`), not observed
behaviour. A field name in a struct is evidence about the *shape* the binary parses; it is **not**
evidence about what the binary does with it.

### Custom model providers

- `ModelProviderInfo` deserialises `base_url`, `env_key`, `env_key_instructions`,
  `experimental_bearer_token`, `aws`, `query_params`, `http_headers`, `request_max_retries`,
  `stream_max_retries`, `stream_idle_timeout_ms`, `websocket_connect_timeout_ms`,
  `requires_openai_auth`, `supports_websockets`, `wire_api`. (V — the binary's own serde field list)
- A custom provider id may not collide with a built-in one:
  `model_providers contains reserved built-in provider IDs: … Built-in providers cannot be
  overridden. Rename your custom provider (for example, openai-custom)`. metaharness's entries are
  therefore `metaharness_endpoint` and `metaharness_loopback`. (V — verbatim literal)
- `https://api.openai.com/v1/responses` is a literal in the same binary: the host an API-key run
  talks to, and what the loopback proxy forwards to when the run named no gateway. (V)
- **Unknown (?):** whether a ChatGPT-plan (subscription) session honours a custom provider's
  `base_url` at all, or rewrites it to `chatgpt.com/backend-api/codex`. Both hosts are literals in
  the binary; which path wins is behaviour, and behaviour needs a run. This is **V-LP6's open half**
  and it is why the codex loopback door refuses a subscription login by name.

### `auth.json`, the two login classes

- `struct AuthDotJson with 7 elements`: `OPENAI_API_KEY`, `auth_mode`, `tokens`, `last_refresh`,
  `agent_identity`, `personal_access_token`, `bedrock_api_key`; the token object carries
  `access_token`, `refresh_token`, `account_id`, `id_token`, `chatgpt_user_id`, `plan_type`, …
  (V — the binary's own serde field list)
- So the two classes are **distinguishable before a run starts**, which is what lets metaharness
  refuse the unrouted one by name instead of failing at the first request an hour in.

### What a `PreToolUse` hook response may say

Every literal below is verbatim, and each one is a rule the adapter's renderer obeys (V):

- `PreToolUse hook returned permissionDecision:deny without a non-empty permissionDecisionReason`
- `PreToolUse hook returned permissionDecisionReason without permissionDecision`
- `PreToolUse hook returned updatedInput without permissionDecision:allow`
- `PreToolUse hook returned unsupported permissionDecision:ask`
- `PreToolUse hook returned unsupported decision:approve`
- `PreToolUse hook returned unsupported continue:false` / `… unsupported stopReason` /
  `… unsupported suppressOutput`

**And the one that cuts the other way:** the same table also holds
`PreToolUse hook returned unsupported permissionDecision:allow`, next to
`PermissionRequest hook denied approval`. Two readings, opposite in consequence — either `allow` is
refused at this event, or that literal belongs to another surface — and **a string table cannot say
which** (?). Note that the `updatedInput` literal *requires* `permissionDecision:allow`, which is
hard to reconcile with allow being refused outright at the same event. The adapter renders `allow`,
proves the rendering and its delivery free at C3, and leaves the vendor half labelled undriven until
`tests/live_codex.rs`'s allow vector is spent. Nothing was upgraded on the strength of a string.

## What this changes for wave 4

The W4.4 adapter is smaller than budgeted (the rollout carries more than stream-json does), and
the enforcement mapping's portability claim upgrades from "three adapter points" to "three
adapter points **plus a hook contract that is near-identical on both harnesses**" — the same
hook script shape, the same deny semantics, a per-turn approval-policy record the trace spec can
assert on. The open questions are exactly two: rollout-format stability (version-gate, unk on
unknown) and the approval-event wire shape (one `-a untrusted` run answers it).
