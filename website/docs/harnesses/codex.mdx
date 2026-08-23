---
id: codex
title: Codex
sidebar_position: 2
---

# The `codex` adapter

Pinned to **0.145.0**. Adapter class: `harness`.

```bash
metaharness run codex --hermetic -p "…"
metaharness conformance codex     # 17 vectors, 4 of which run a real process and the real hook
metaharness doctor codex
```

## What a run does

1. Starts a real `codex exec` into a scratch `CODEX_HOME`.
2. Copies the operator's `auth.json` in **per spawn**.
3. Declares a blocking `PreToolUse` hook.
4. Tails the **session rollout** for events — the record that carries timestamps, durations and
   per-turn usage, where `codex exec --json` stdout carries none.
5. Retains those bytes for the auditor and answers the hook per call.

`capabilities codex`, `conformance codex` and `doctor codex` all work with **no model and no
credential**.

## Declared capabilities

| Tier | Status | Why |
|---|---|---|
| `registration` | `unverified` | `codex exec` takes no tool allowlist. `sandbox_mode` and `approval_policy` constrain the **process**, not the offered tool set. Declared rather than claimed. |
| `call` | **`delivered`** | Driven, live. See below. |
| `turn` | `unverified` | `thread/inject` is an app-server method; this adapter drives `codex exec`, which has no channel for text between turns. |
| `kill` | `delivered` | By terminating the child — metaharness's own act on a process it started, not a claim about a vendor surface. |

## Operation rendering

Deliberately sparse. An operation with no vendor tool publishes as `None` — **a fact worth
publishing rather than an omission**.

| Neutral operation | Vendor tool |
|---|---|
| `shell` | `Bash` |
| `file.write` | `apply_patch` |
| `file.edit` | `apply_patch` |
| everything else | *(none)* |

The `apply_patch` rendering comes from the vendor's documentation, **not from a driven
observation**, and is labelled as such.

## The live proof, and its exact limits

A policy that admitted no shell met a prompt that asked for one.

| Layer | What it said |
|---|---|
| The hook process | `{"tool_name":"Bash","tool_use_id":"exec-96257928-…"}` |
| metaharness | answered `deny`, with a reason |
| The vendor's session record | `Command blocked by PreToolUse hook: this step admits no shell, so the command did not run` — with an **empty** `Output:` |
| The model's closing message | *"The command was blocked and did not run."* |

So `tool.decide` is `Honoured` and the `call` tier is `Delivered`. Design amendment **a7**.

:::info The `allow` half is **driven live** (2026-08-23)
The paid vector in `tests/live_codex.rs` was spent: the hook received a real `Bash` call,
metaharness answered `permissionDecision: allow`, the command **ran**, and the rollout's own
`custom_tool_call_output` carried its output. The census read `allowed: 1, denied: 0`. The literal
`PreToolUse hook returned unsupported permissionDecision:allow` in the binary's string table
belongs to some other code path — the observed one honours the grant.

One caveat travels with the observation: the child's `PATH` resolved codex **0.144.0** while the
pin is 0.145.0 (the two-install warning fired, as it must). The grant is a driven fact about
0.144.0 and an inference about 0.145.0 until one machine holds one install.
:::

Also not claimed: the turn tier, the registration tier, and the `apply_patch` rendering.

## The vendor sandbox is what the cwd declaration decides

`sandbox_mode` in the scratch `config.toml` is **not a constant** — on this vendor it is the same
setting as "may the child do the work it was pointed at".

| Run | `sandbox_mode` | What the child may write |
|---|---|---|
| scratch cwd (the default) | `read-only` | nothing, anywhere |
| `--cwd <a real tree>` (amendment a6) | `workspace-write` | files in that tree, and nothing outside it |

The vendor's own description of the second value: *"The sandbox permits reading files, and editing
files in `cwd` and `writable_roots`. Editing files in other directories requires approval."* The
child is spawned **in** the named tree, so no `writable_roots` entry is written and the grant stops
at the declared directory. `--add-dir` is still never passed, `danger-full-access` is never written,
and no `[sandbox_workspace_write]` table is emitted — so nothing here claims or changes this
vendor's network default.

The H7 attestation row states the grant in words, so **"this run could write to the operator's
tree" is readable off the run's own record**, not only off a scratch config that is deleted when the
run ends. `--hermetic strict` still refuses a named-cwd run: the grant changes what the child may
*do*, never what the attestation *claims*.

:::warning Why this exists
Until 2026-08-23 every codex run was `read-only`, including `--cwd` ones — so amendment a6's trade
bought a real repository the child could not change. A paid run found it: *"the workspace is
read-only, so the planning-store patch was rejected."* Design amendment **a6.1**.
:::

## The loopback provider on codex — the API-key half

`--credentials loopback` puts metaharness in front of the vendor. The child gets **no `auth.json` at
all**: a `[model_providers.metaharness_loopback]` entry in the scratch `CODEX_HOME` points it at a
per-run `127.0.0.1` port, `env_key` names `METAHARNESS_LOOPBACK_KEY`, and that variable holds a
placeholder worth nothing anywhere else. One custody on metaharness's side of the socket attaches
the real token on the way out.

| Login in `~/.codex/auth.json` | What happens |
|---|---|
| `OPENAI_API_KEY` (an API key) | Routed through the proxy. Free-proven end to end; **that codex honours the provider entry is one paid turn, not yet spent**. |
| `tokens` (a ChatGPT plan) | **Refused by name.** Whether subscription traffic can be routed through a custom provider at all is unverified (**V-LP6**) — so it is stated, never degraded to the credential-copy path the loopback provider exists to replace. |

## Plugin injection — the placement is driven, the plugin list is not

`codex exec` has **no `--plugin-dir`**: `codex plugin` installs from marketplace snapshots, so
there is no flag with which to hand the vendor a directory. So the adapter picks a location —
`$CODEX_HOME/plugins/<name>`, chosen because this binary keeps `plugins/cache` and `plugins/data`
under the Codex home and a marketplace's own entries are `./plugins/<plugin-name>` — copies the
declared directory there, and digests it before the copy.

:::info The placement is **driven live** (2026-08-23, run `codex-2139643`)
A directed probe copied a plugin to that placement and asked the model to answer **from its runtime
context only, using no tools**. It answered *"Available skills catalog — `## Skills`"*, and the run
made **zero tool calls** — census `0/0/0/0`, no `tool.requested` at all — so the catalog could not
have been read off disk. **The vendor surfaced the injected plugin's skills into the model's
context from this path**, with no marketplace manifest and no `codex plugin add` behind it.

Two limits travel with it, and both are carried in the run's own attestation:

- The child was codex **0.144.0** — the binary this machine's constructed `PATH` resolves —
  against a pin of 0.145.0. A driven fact about that binary; an inference about the pin.
- `session.started.plugins` was still `null`. **The vendor enumerates no plugins, so H1a reads
  `unk`.** What was observed is the plugin's *content* reaching the model, which is what a treated
  evaluation arm needs — not the vendor stating what it loaded, which is what H1a asks for.

Not claimed: anything about how *well* the surfaced skill is used.
:::

The launch deliberately writes **no** `[marketplaces]` table to go with the copy. An unrecognised
key under a table this binary reads is dropped without failing the config load, a malformed one can
fail it outright — which here is a run with no seam — and the probe shows the copy alone is enough.

## Three things that cost more to learn than the code using them

Each is a **silent failure**. This is why every claim above is read from the run's own record
rather than from the file that configured it.

### 1. A hook is declared in `config.toml`, not `hooks.json`

A `hooks.json` is a *plugin manifest's* file. An unrecognised key under `[hooks]` is dropped
**without failing the config load** — so a misconfigured seam and a run where nothing was
attempted are the same observation.

### 2. A hook in a fresh `CODEX_HOME` never fires without `--dangerously-bypass-hook-trust`

A scratch home cannot hold persisted trust. The flag's warning is about running *somebody else's*
hook unvetted; the only hook here is the one metaharness wrote a moment earlier.

### 3. The hook speaks Claude Code's tool vocabulary

`tool_name` is `Bash` — where the rollout calls the same call `exec` and the binary's own
model-facing list calls it `shell`.

A rendering table built from the *record* would have denied every shell call and reported it as a
frame decision.

## A version split nobody was looking for

`codex --version` says **0.145.0**. The `session_meta.cli_version` written by the run that same
binary starts says **0.144.0**.

The adapter keeps one pin, the reader **warns rather than widening it**, and the split is tracked
as open question **Q18**.

## The rollout format has no stability guarantee

It drifts within a single install. That is why the version gate is a warning and **never a refusal
mid-read** — and why the pin is a single entry: a verdict that changed because the *reader*
changed must be visible as such, not as a change in the agent's behaviour.

## Evidence labels

The evidence base is the research record `docs/research/2026-08-21-codex-harness-research.md`.
Every claim there is labelled, and this crate inherits the label at the point of use:

| Label | Meaning |
|---|---|
| **V** | Verified locally against the binary and 2,437 rollout files. |
| **D** | Official docs. |
| **I** | Inferred. |
| **?** | Unknown. |
