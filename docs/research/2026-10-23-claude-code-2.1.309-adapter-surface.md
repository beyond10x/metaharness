# Claude Code 2.1.309 adapter surface

Status: read on 2026-10-23 against Claude Code **2.1.309**, beside **2.1.305** as the control, with
**no paid run, no login and no credential**. It is the evidence `metaharness-claude::PINNED_VERSIONS`
moved on, from `2.1.304`, `2.1.305` to `2.1.304`, `2.1.305`, `2.1.309`. It is dated, on a8's rule:
the rows below name the binaries they were read from, and a later release does not inherit them.
2.1.304 was not re-run; its rows stay those of `2026-10-21-claude-code-2.1.304-adapter-surface.md`.
2.1.306 is not on this machine, and 2.1.307 and 2.1.308 are on it and stay off the pin.

Each row carries one of three labels:

| label | means |
|---|---|
| **observed** | a run of the named binary in this work shows it |
| **binary** | read from the named binary's strings or its `--help`, not driven |
| **unverified** | neither; stated so that nobody reads silence as a check |

A row that names no version holds for both binaries, each read or run on its own.

## The binaries

| fact | 2.1.305 | 2.1.309 |
|---|---|---|
| version (`--version`, exit 0, offline scratch home) | `2.1.305 (Claude Code)` | `2.1.309 (Claude Code)` |
| file | a downloaded copy under `~/.cache/`, 290,532,720 bytes, SHA-256 `86cb722ab68348877dba7971e4d0c659919e05e6228c05e006e83da320e656ef`, the digest the 2.1.304 page records | `~/.local/share/claude/versions/2.1.309`, 293,469,552 bytes, SHA-256 `4d704a8aecbe038affa3000f498ae17c1b4f753add8facb8764120affc99b658` |
| `--help` (exit 0) | 313 lines | byte-identical to 2.1.305's |

Off the pin, on disk: 2.1.308 (292,232,560 bytes, SHA-256
`356733f185fa988e6bf3aa9a555269fdd167982a2e2a80d3c8a78037bb92ad58`), the lane's off-pin binary and
otherwise asked for `--version` and its built-in registration only; 2.1.307 (291,974,512 bytes,
SHA-256 `6021eb72d20d0682b720bdc63acd2a5cd2270895d1f10d9ca2a88cbfa7b9dd0e`), not run. 2.1.303, the
off-pin binary of the 2.1.304 page, is no longer on this machine.

Every binary was run by explicit path, in a user, network and mount namespace with only `lo`, with
a fresh scratch `HOME` and `CLAUDE_CONFIG_DIR` per invocation and an empty directory bound over the
operator's `~/.claude`; never logged in, never pointed at a real provider. The driver is
`target/debug/metaharness` built from this change (`cargo build --locked --bin metaharness`),
SHA-256 `7f985eebccf931421e0afb4b4cb17be7e14e871e8ac010fb5fbf1f107275acb8`; both lanes ran
against that one file.

## The native fixture

`crates/metaharness-claude/tests/native_fixture.rs`, ignored in `task check`, run as the 2.1.304
page describes: `METAHARNESS_NATIVE_NETNS=loopback-only`, `METAHARNESS_NATIVE_CLAUDE` set to one
binary per lane, `METAHARNESS_NATIVE_CLAUDE_OFF_PIN` to 2.1.308, `--ignored --test-threads=1`.

| lane | result | run directories |
|---|---|---|
| 2.1.309 | **9 passed, 0 failed**, 1 filtered out (27.51 s) | 30: 21 sessions through the driver, 2 refused 2.1.308 launches, 7 direct vendor runs (5 mention routes, 2 memory controls) |
| 2.1.305 | **9 passed, 0 failed**, 1 filtered out (20.12 s) | 30, the same split |
| 2.1.309, the base tree (`PINNED_VERSIONS` without 2.1.309) | 1 passed, 8 failed | the red half of the pin move: 7 cases fail the fixture's own check that the binary under test is pinned, the audit case fails because the base driver refuses 2.1.309 under `--strict-version` before the spawn, and the off-pin case passes |

The two credential-free modes are the 2.1.304 page's: `none` (the child holds the endpoint
placeholder `ANTHROPIC_API_KEY`, the stub admits that `x-api-key`) and `loopback` (the child holds a
per-run placeholder bearer, the stub admits only the test's fake token, attached by metaharness's
proxy).

| observation, each binary | `none` | `loopback` | label |
|---|---|---|---|
| opening record reports the binary's own `claude_code_version` (21 of 21 per lane) | yes | yes | observed |
| success, provider HTTP 400, usage, the `PreToolUse` hook's allow and deny, interrupt and halt, `--max-turns 1` (the 2.1.304 page's rows) | each as at 2.1.304 | each as at 2.1.304 | observed |
| requests the stub saw | `POST /v1/messages?beta=true` only, 38 per lane; no `HEAD`, no `count_tokens` | the same, every one through the proxy | observed |
| MCP list in the opening record | `[]` | `[]` | observed |
| plugins in the opening record | `cc-plugin-agents-md`, `cc-plugin-plugin-authoring`, each `<name>@builtin` with `path: "builtin"` (21 of 21 per lane) | the same | observed |
| `apiKeySource` in the vendor's `system/init` | `"ANTHROPIC_API_KEY"` | `"none"` | observed |
| `--strict-version` with 2.1.308 on the child's `PATH` | CLI exit 2 before the spawn, the refusal names 2.1.308; nothing reached the stub, no `session.started` | the same | observed |
| floor with `--audit`: H9 `ok`, H4 `ok`, H1a `ok` naming both built-ins | yes | yes | observed |
| the opening `session.started` records, with scratch directory names masked | equal to 2.1.305's, all 21 | the same | observed |
| records the reader mapped to `opaque` | 0 | 0 | observed |

## The record

| fact | label |
|---|---|
| record types and subtypes the transcripts hold: `system/init`, `assistant`, `user`, `result/success`, `result/error_max_turns`, the same set and counts as 2.1.305 | observed |
| every field path of every record is 2.1.305's, plus one: `result.model_usage_since_last_result`, on both `result/success` and `result/error_max_turns`, keyed by model with the same twelve fields as `modelUsage` | observed |
| the schema describes it as *"@internal Per-model usage and cost of the calls this process counted since it wrote its previous result (since it started, for its first), keyed and shaped like modelUsage and covering the same calls"*, `optional()` | binary |
| `modelUsage`'s own description is unchanged (cumulative, *"read the latest result rather than summing across results"*) | binary |
| the reader ignores a field it does not know (D4), so the new field changes no event; nothing in metaharness reads it | observed (the lanes; no `opaque`) |
| `--setting-sources` parser: `if(e==="")return[]`, the same text as 2.1.305 under another minified name | binary |
| the plugins field's schema, with the `name@builtin` sentinel, equal to 2.1.305's after minified names are masked | binary |

## Built-in plugins

| fact | label |
|---|---|
| the registration function guarded by `builtinPluginsInitialized` registers the same twelve names in the same order as 2.1.305 (and as 2.1.308), the twelve of the 2.1.304 page | binary |
| `claude plugin list --json` prints `[]` | observed |
| `claude plugin details <name>@builtin` in a fresh offline scratch home resolves `cc-plugin-agents-md`, `cc-plugin-telemetry`, `cc-plugin-plugin-authoring`; with `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1` it resolves two (`cc-plugin-telemetry` is gone); the output is byte-identical to 2.1.305's | observed |
| no `cc-plugin-telemetry` in any of the 28 `system/init` records per lane | observed |
| an operator-login session's built-ins | **unverified**: not driven, nothing relayed for 2.1.309 |

So `BUILTIN_PLUGINS` gains a 2.1.309 row equal to the others, and the constant is renamed
`COMPILED_IN_2_1_304_2_1_305_AND_2_1_309`.

## H11: memory files

The memory-shapes string is the same in both binaries,
`[["CLAUDE.md"],["CLAUDE.local.md"],["AGENTS.md"],[".claude","CLAUDE.md"],[".claude","AGENTS.md"],[".claude","rules"]]`
(binary). Project-memory discovery is gated as at 2.1.305: `if(S){… "CLAUDE.md" …}` with
`S=…("projectSettings")&&!s`, `.claude/rules` returns `[]` unless `projectSettings` is on, and
`CLAUDE.local.md` is loaded under `("localSettings")` (binary; the same text under other minified
names).

| run, 2.1.309 | canaries planted | sent | label |
|---|---|---|---|
| the vendor run directly, `--setting-sources project,local` (the positive control) | 12 | **12** | observed |
| the production launch (`--setting-sources ""`), `none` | 12 | 0 | observed |
| the production launch, `loopback` | 12 | 0 | observed |

`MEMORY_CANARIES` gains `{2.1.309, planted 12, control_sent 12, launch_none 0, launch_loopback 0}`;
the fixture case compares what it measured with that row field by field, so a copied row would
have failed it. 2.1.305 measured the same today. User memory and managed memory were not probed
(unverified).

## `@`-mentions

| route for `@marker.txt`, 2.1.309 | result | label |
|---|---|---|
| the `-p` argument | expanded | observed |
| stdin, with `-p` | expanded | observed |
| stdin, `--input-format stream-json` | expanded | observed |
| `--append-system-prompt` | not expanded, byte-exact in `system` | observed |
| `--system-prompt` | not expanded, byte-exact in `system` | observed |
| `--context` files: byte-exact in the appended system prompt, no file read into any request, `channel: "system-prompt"` in the opening record | yes | observed |
| paths outside the tree, through symbolic links, untracked files, directories | not measured on 2.1.309 | **unverified** |

`MENTION_CANARIES` gains the 2.1.309 row (expanded, expanded, expanded, text, text).

## a41 § 3 and § 5, re-read

a41 § 4 says a pin move re-reads § 3 and § 5. Both were read from the 2.1.309 binary; neither was
driven (invariant 4).

| reading | 2.1.305 | 2.1.309 | label |
|---|---|---|---|
| the key on a `type: command` settings hook | `onFailure`, `["continue","block"]`, optional, description word for word as a41 § 3 quotes | the same text | binary |
| when `block` applies to a command hook | `e.onFailure==="block"&&e.async!==!0&&e.asyncRewake!==!0` (`k1e`) | the same text (`MGe`) | binary |
| exempt events | `Stop`, `SubagentStop`, `TaskCompleted`, `TeammateIdle` | the same | binary |
| a spawn failure yields `non_blocking_error` | yes | yes | binary |
| stdout that fails validation yields `hook_non_blocking_error` / `non_blocking_error` | yes | yes | binary |
| **what it converts** | `e.outcome==="cancelled"&&r?.aborted!==!0?"timed out"` | `e.outcome==="cancelled"&&(r?.aborted!==!0\|\|e.stoppedAfterItsTimeout===!0)?"timed out"` | binary |
| **the cancelled message's `timedOut`** | `timedOut:!y?.aborted` | `timedOut:!$l`, where `$l=y?.aborted===!0&&os.stoppedAfterItsTimeout!==!0` | binary |
| the stdout parser (§ 5) and what it calls | 842 bytes | equal to 2.1.305's after minified names are masked | binary |
| the exit-other-than-0-or-2 path | `type:"hook_non_blocking_error"` 13 times, `type:"hook_cancelled"` 8 times | the same counts; the path itself not traced in either | binary (counts only) |

The change is `stoppedAfterItsTimeout`. The runner sets it when the action's abort signal fires
after the hook has already run for its own timeout (`monotonicNow()-start>=timeout`, not in the
background). 2.1.305 converted a cancelled hook to "timed out", and so to a block under
`onFailure: "block"`, only when the action was not aborted. 2.1.309 also converts one that the abort
reached after the hook's timeout had passed. Every case that blocked at 2.1.305 still blocks, and
one more case does now. That is the direction a41 wants (the hook fails closed), so the adapter
needs no change; the launch test's recorded vector gains a 2.1.309 row with an `aborted_binding`
field that names the new binding, and the timeout test builds the expected `timedOut` flag from it.

## What else moved

| fact | label |
|---|---|
| new control-request subtypes in the schema: `list_cloud_sessions`, `get_cloud_session`, `get_cloud_session_transcript`, `submit_survey_feedback`; new internal types `projects_thread_ask`, `search_result_location`. metaharness sends none of them, and no `cloud_session` string is in any record of either lane | binary; observed (absence in the lanes) |
| `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` occurs 36 times (2.1.305: 34); the two new ones are in one new message, *"Cloud sessions are unavailable while CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC is set."* The mode function (`essential-traffic`, `no-telemetry`, `default`) is the same text | binary |
| managed-settings strings: `"/etc/claude-code"` 1, `managed-settings.json` 29, `managed-settings.d` 11, `/mnt/c/Program Files/ClaudeCode` 3, `CLAUDE_CODE_MANAGED_SETTINGS_PATH` 8, the same counts as 2.1.305; the paths were not traced through the 2.1.309 code | binary (counts only) |
| the literals a non-plugin settings hook may not name, `${CLAUDE_PLUGIN_ROOT}` and `${CLAUDE_PLUGIN_DATA}`, and the error text that refuses them (a37 § 6) | binary, the same text |

## Every adapter claim, at 2.1.309

| claim | label |
|---|---|
| every flag the adapter passes is in `--help` | binary (the help is byte-identical to 2.1.305's); observed (every fixture run) |
| `--max-turns` is accepted | binary: still `.hideHelp()`; observed (turn ceiling) |
| `--setting-sources ""` means no sources (H2) | binary (the parser); observed (the memory control) |
| `--strict-mcp-config` (H5) | observed (MCP list `[]`) |
| `CLAUDE_CONFIG_DIR` is the scratch home (H1b) | observed (audit `ok`) |
| `ANTHROPIC_BASE_URL` plus the placeholder key sends every API request to the base | observed (`none`) |
| `ANTHROPIC_AUTH_TOKEN` is sent as a bearer, relayed by the proxy with custody | observed (`loopback`) |
| a `PreToolUse` command hook with matcher `""` holds `Bash`, and its decision is obeyed | observed, allow and deny |
| the hook's `onFailure: "block"` reading (a41) | binary, above; not driven |
| the vendor's behaviour when the hook exceeds its timeout (Q10) | unverified |
| `--plugin-dir`, `--tools ""` with the owned surface, a bare `--allowedTools` entry, `--effort`, `--max-budget-usd` | unverified (not driven), as at 2.1.304 |
| an operator login or an API key; the new cloud-session control requests | unverified (not driven) |

## Differences from 2.1.305

| difference | label | the adapter |
|---|---|---|
| `result.model_usage_since_last_result`, a per-result delta beside the cumulative `modelUsage` | observed; binary (description) | ignores it (D4); a later change may read it |
| `onFailure` converts a cancelled hook marked `stoppedAfterItsTimeout` even when the action was aborted, and `timedOut` follows the same binding | binary | accepts it: the blocking cases grew and none was lost; the recorded vector gains the row |
| four new control-request subtypes and two internal types | binary | never sends or reads them |
| one new message naming `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` (cloud sessions) | binary | none; the switch's mode function is unchanged |
| the binary is 2,936,832 bytes larger | binary | none |

Nothing else the adapter reads or the floor asserts moved, and no surface needs a design change.

## `metaharness doctor claude`

Run with that driver in the same namespace, from a scratch `HOME` whose `.local/bin/claude` is the
binary, output to a file and the exit status read directly (observed):

| binary | first line | exit |
|---|---|---|
| 2.1.309 | *"2.1.309 — on the adapter's pin (2.1.304, 2.1.305, 2.1.309)"* | 0 |
| 2.1.308 | *"2.1.308 — OFF the adapter's pin (2.1.304, 2.1.305, 2.1.309) … --strict-version refuses the run rather than reporting it"* | 1 (`gap`) |

Both print a second line, *"every flag the adapter sends is one the binary declares"*. For claude
that check never runs: `metaharness::flag_surface` (`crates/metaharness/src/doctor.rs:206`) returns
`None` for every kind but b10x, and `doctor` (`crates/metaharness-cli/src/lib.rs:539-566`) reads
`None` as no faults. The line predates this change and is not changed here; this page's flag claim
rests on the help comparison above, not on the doctor.

## Private evidence

Under `~/.cache/metaharness-qualify-2026-10-23/claude/`: `native-evidence/final-2.1.30{5,9}/` (the
lanes above, with the final driver; one directory per vendor process: events, stderr with the
audit, recorded request bodies, retained transcripts), `native-evidence/pinned-2.1.30{5,9}/` (the
same lanes on an earlier build of this change), `native-evidence/red-base-2.1.309/`, `evidence/` (lane logs, help text, registration excerpts for 2.1.305, 2.1.308 and
2.1.309, the `plugin details` tables, the onFailure excerpts per binary, the stdout parser excerpts,
the field-path and opening-record comparisons, managed-settings and traffic counts). The scripts
that produced them sit beside them. No transcript or request body is committed.
