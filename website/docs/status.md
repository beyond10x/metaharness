---
id: status
title: Status
sidebar_position: 9
---

# Status

**Pre-v1.** The design in `docs/design/` is what is binding — where the code and that document
disagree, **the document is amended** rather than the disagreement left in the code.

The governed comparison subject, evaluation crate, and fixtures now use AEP's canonical naming;
the migration leaves the pinned frame and transcript wire formats unchanged.

## Built

| | |
|---|---|
| **AEP-governed model execution** | `metaharness aep drive` hosts model-backed maps, native execution hooks and live evaluation through AEP's neutral library. AEP retains planning, governance, command/operator execution, run storage and offline stream ingestion. Compatible paused runs keep their recorded spend limits. |
| **Claude Code, driven end to end** | `metaharness run claude --hermetic -p "…"` spawns 2.1.304 or 2.1.305 into a scratch config home, installs a blocking `PreToolUse` hook, answers it per call, streams protocol events on stdout, takes steering on stdin, retains the raw transcript, and exits `0` when the stream closes complete on the vendor's terminal record, `3` otherwise; `--audit` or `--hermetic strict` adds the hermetic floor's verdict ([quickstart](./quickstart.md#4-a-real-run)). |
| **Codex, driven end to end (CX-M2)** | `metaharness run codex --hermetic -p "…"` starts a real `codex exec` into a scratch `CODEX_HOME`, copies `auth.json` per spawn, declares a blocking `PreToolUse` hook, and tails the **session rollout** for events. |
| **Free verbs** | `capabilities`, `conformance` and `doctor` work with **no model and no credential**. `conformance claude` runs **30** vectors; `conformance codex` runs **24** vectors, **4** of which drive a real process and the real hook program; `conformance b10x` runs **8** vectors, recorded launch, loop, version and enforcement checks. Each count is `conformance_vectors(kind).len()`, checked by `crates/metaharness-cli/tests/website_claims.rs`. |
| **Trace projection** | `metaharness project <EVENTS>` writes the `trace-ir/1` document (design amendment a15), `--to trace-ir-summary` its summary (a42), and `--html` one or two runs as one static page. `crates/metaharness-cli/tests/website_claims.rs` runs it over `evals/aep/runs/decomposer-clean.events.jsonl` and reads exit `0`. |
| **Capture mode** | `--decisions observe` allows every call and records every call, through the same hook the enforcing modes use: the same events, the same correlation keys, `decided_by: "observe"`. It exists so an unsteered run and a steered one can be scored by one set of expectations. It is refused beside a `--frame`, and an `allow` on this wire *grants* — the attestation says so on every observe launch. |
| **Plugin injection** | `--plugin-dir` copies the directory into the run's own scratch tree, digests it before the copy, and points the vendor at the copy. The copy list and the digest are readable off the launch plan before anything is spawned, and `session.started` carries the digest and the source — or an explicit empty list. A directory that is missing or empty is refused with exit `2`. |
| **The loopback provider — Claude Code, and now the API-key half of Codex** | `--credentials loopback` puts metaharness in front of the vendor: the child holds a per-run placeholder and **no credential file at all**, and one custody attaches the real token on the way out. On Codex the child is pointed at the proxy by a `[model_providers.metaharness_loopback]` entry in the scratch `CODEX_HOME`. A **ChatGPT-plan** Codex login is **refused by name** — whether the vendor routes subscription traffic through a custom provider is unverified, and no run here guesses. |
| **Frames across the process boundary (a5)** | `--frame step.frame.json` takes a sealed `metaharness.frame/1` document — digest-verified on load, refused by name when unreadable, untagged, misshapen or edited after sealing, enforced per call from the first turn. |
| **The b10x loop, observed** | `metaharness run b10x --decisions observe` spawns the beyond10x loop under a constructed environment, with no hook and no control request — the published toolset is the policy — and the operator's other tiers (write scope, ceiling, content hook, driver program, plugin) travel as flags. `frame` and `ask` are refused at launch because either would claim a decision seam this adapter does not have. See [the b10x adapter](./harnesses/b10x.md). |
| **The eval machinery** | Lives in `evals/`, migrated from `AEP`. Reads its denial census from `tool.decided` events in the run's own streams — and, on the b10x arm, which has no decision seam, from `hook-refused`, `approval-denied`, `unpublished-tool` and `program-refused` warnings. Two arms, one step map, one plugin: Claude Code through `run claude`, the b10x loop through `run b10x`, both driven by `aep-eval driven`. On 2026-08-29 the b10x arm read **30 pass / 0 fail, `EVAL_EXIT=0`**. **Nothing under `evals/` is part of `task check`.** |

## Proven against paid runs — on both vendors

A frame that admitted no shell met a prompt that asked for one. The call did not run, and **the
vendor's own record says so**.

| Vendor | The vendor's own evidence |
|---|---|
| Claude Code | `Bash` in `permission_denials` |
| Codex | `Command blocked by PreToolUse hook…` with an **empty** `Output:` |

And the grant half, driven 2026-08-23 (R2.4): the hook held a real `Bash` call, metaharness
answered `permissionDecision: allow`, the command ran, and the rollout's own
`custom_tool_call_output` carried its output — census `allowed: 1, denied: 0`. Caveat carried with
the observation: the child's `PATH` resolved codex **0.144.0** while the pin was 0.145.0, and no
record of that run is kept in `docs/research/`.

That is the claim the whole design exists to be able to make, and it is the one thing no free test
tier can reach.

On the current pin, **0.153.4**, the grant half is read from a recorded run that spent nothing: a
fixture provider on loopback, the real `codex` binary, every launch planned in `--decisions
observe`, and the real hook was answered `allow` through the production renderer. The real
command wrote its marker on exit 0 and on exit 7, and the otherwise identical denied command wrote none
(`docs/research/2026-10-03-native-codex-fixture.md`, rows *Command exit0*, *Command exit7* and
*Hook denial*). On the current pin, that record is the evidence for `observe` on Codex being
declared `delivered` (design amendment a49). It is not a hosted model and not a paid run, and the
`allow` was written by the probe, not by metaharness's observe loop inside a binary run (a49 § 4).

Live runs cost money and are **never** part of `task check`. `METAHARNESS_LIVE=1` is checked by
`metaharness aep drive run` and `metaharness aep drive resume` for a step map with an `llm` step, by
`metaharness aep drive eval run` without `--stream`, by the repository's `aep-eval` runner with
`--spend`, and by its ignored live tests. A plain `metaharness run` does not check
`METAHARNESS_LIVE`: it spawns the harness with the operator's credentials whatever the variable
holds.

## Not built — and refusing by name

| Verb | Waiting on |
|---|---|
| `metaharness audit` over a transcript metaharness did not itself launch | named at the refusal: the launch facts the hermetic floor compares a record against. |

It refuses with **exit 2**, naming what it waits for (`crates/metaharness-cli/src/lib.rs`,
`execute`; `website_claims.rs` reads the exit).

| Also refused | Why |
|---|---|
| `--tool-surface owned` on `codex` and `b10x` | Strategy C means metaharness implements the tools itself, and per-step re-listing is unverified vendor behaviour (**Q1**). |
| `steer` on Claude Code | Claude Code headless has no mid-turn tier. |
| Claiming Codex's opening record confirms an injected plugin | It does not: `session.started.plugins` is `null` on this vendor, so **H1a reads `unk`**. One live run *did* observe the plugin's skills reaching the model's context (**Q19**) — that is the treatment arriving, not the vendor enumerating what it loaded, and the attestation says which of the two it is. |
| Passing `--hermetic strict` on Codex | A clean run still exits **3**. H1b reads `ok` from the record since 2026-10-21 when no `/etc/codex` or project config layer exists (the personality the launch wrote is the one the session recorded; that is the session's record of the setting, not proof of what the model was told), while H1a, H4 and H5 stay `unk`: the 0.153.4 rollout carries no plugin list, credential source or MCP server list, and a row is never decided from what the launch imposed (design amendment a34). Use `--hermetic on` for Codex trials. |
| `--credentials loopback` on a **ChatGPT-plan** Codex login | Whether subscription traffic can be routed through a `model_providers` entry at all is unverified (**V-LP6**). Refused by name, never degraded to the credential-copy path. |
| `--write-scope`, `--scope-announce`, `--toolchain`, `--prices` and the substrate flags on `claude` or `codex` | A vendor arm's scope travels sealed in `Frame.subjects`; a flag would be a second, unsealed copy that disagrees the first time one moves (`ScopeUnsupported`). The others declare things a vendor's own sandbox and price catalogue already own. `--context` left this row on 2026-10-21 (design amendment a35): it reaches every kind — the appended system prompt on Claude Code, inline on codex. |
| A `--context` file without `--cwd`, absolute or with `..`, through a symbolic link, not a regular file, not UTF-8 text, untracked by git in the tree, or past 16 files or 65,536 bytes; a vendor argument past 131,072 bytes | Refused before the spawn, naming the path, or the argument with its size and the files with theirs (design amendment a35). |
| Passing Claude Code a task that holds an `@path` | 2.1.304 and 2.1.305 read the mentioned file into the request with no tool call and no record, from any route into the user turn (`-p`, stdin, stream-json); measured, not closed. `--context` files avoid it by going to `--append-system-prompt` (design amendment a35). |

## Built, and not driven live — the labels that have not moved

| Claim | Where it stands |
|---|---|
| The Codex loopback door | **Built free, paid confirmation outstanding.** The provider entry, the placeholder, the empty copy list and the custody swap are all proven without a vendor binary. That `codex` itself honours the entry is one live turn nobody has spent. |
| Codex loading an injected plugin | **Driven once, 2026-08-23 (Q19).** A directed probe copied a plugin to `$CODEX_HOME/plugins/<name>` and asked the model to answer from its runtime context with no tools; it quoted the injected **skills catalog** — *"Available skills catalog — `## Skills`"* — with **zero tool calls**. Two limits travel with it: the child was codex **0.144.0** against a 0.145.0 pin, and the vendor's opening record still lists no plugins, so H1a is `unk`. Nothing is claimed about how well the skill is *used*. |

## Open questions in flight

| Q | Question |
|---|---|
| **Q1** | Per-step tool re-listing — is it a verifiable vendor behaviour? Gates `--tool-surface owned`. |
| **Q9** | A reader for `trace-ir/1` outside this repository. `metaharness project` writes the document since design amendment a15; what Q9 still gates is a reader, not the writer. |
| **Q13** | Credential lifetime. Answered in part by the `auth.expired` event and the per-spawn copy. |

Q19 — whether Codex loads a plugin from a directory placed in its `CODEX_HOME` — left this table on
2026-08-23, answered **yes** by one directed probe: a plugin copied to `$CODEX_HOME/plugins/<name>`
had its **skills catalog** surfaced into the model's context, quoted back with **zero tool calls**,
with no marketplace manifest behind it. It is answered rather than closed, and both limits travel
with the claim wherever it is stated: the child was codex **0.144.0** against a 0.145.0 pin, and the
vendor's opening record still enumerates no plugins — so H1a is `unk` and the evidence is the
model's answer, not the vendor's plugin list.

Q18 — the version pair disagreeing — left this table with CT-3 (amendment a8): doctor and the spawn
were resolving **different installed binaries**, doctor now resolves on the child's own `PATH`, and a
`golden-version-pair` vector warns a disagreement by name. What remains — one install or two on the
machine — is the operator's, not a question the code can close.

## Roadmap directions

Each is a direction, not a work order. Ordered roughly by how much it de-risks everything else.

| # | Direction |
|---|---|
| 1 | **Contract-test every adapter mapping — built.** `conformance <kind> --contract` emits byte-pinned `contract_result` evidence, and every adapter declares obligations through one checked shape. Claude and Codex pin both vendor faces and their version pair. b10x now pins its launch, its `--context` launch (design amendment a35), real loop record, captured version banner and four provider-emulated enforcement outcomes (`checked: 8`); its hook-input row is an explicit N/A because the observe-only adapter has no metaharness hook seam. CT-4 first exposed Codex's absent launch face (`checked: 10 → 17`) and now exposed b10x's ambient-profile and wrong-attestation launch drifts before its contract was accepted. |
| 2 | **More harnesses: `pi`, `opencode`** — same pattern, research record first, live proof last. Whichever comes first fills the contract checklist before it is believed. |
| 3 | **metaharness usage ≡ flux usage** — UX parity with `flux run --stream-json`. A `metaharness-flux` adapter is **refused**: nothing flux-related is embedded here. |
| 4 | **Sandbox inversion — rejected.** The `0.4.0` scripted contract remains as historical evidence vocabulary, but no production provider, Substrate dependency, or vendor-adapter migration is planned. |

### Why contract testing, concretely

CX-M2's single paid run surfaced **three real drifts**, and none was a bug in the adapter's logic
— each was a *fact about a vendor* the adapter asserts and could be wrong about.

| Drift | What a contract test pins |
|---|---|
| The hook's `tool_name` is `Bash`, not the `exec` the rollout uses or the `shell` the model-facing list uses. | A recorded real hook input; the rendering table must agree with it. |
| `codex --version` and `session_meta.cli_version` disagree. | The pinned version is a **pair of sources to reconcile**, not one string. |
| The hook `tool_use_id` and the rollout `call_id` do not join. | The correlation key the adapter claims must be the one the records share. |

## The gate

`task check`, with the steps it runs in order and what it needs installed, is on
[the quickstart](./quickstart.md#the-gate), read from `Taskfile.yml`. Green before any push. A paid run is never part of a gate.
