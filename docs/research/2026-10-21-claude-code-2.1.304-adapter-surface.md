# Claude Code 2.1.304 and 2.1.305 adapter surface

Status: read on 2026-10-21 against Claude Code **2.1.304** and **2.1.305**, with **no paid run, no
login and no credential**. It is the evidence `metaharness-claude::PINNED_VERSIONS` moved on —
`2.1.259` → `2.1.304`, `2.1.305` (design amendment a33) — and it is dated, on a8's rule: the rows
below name the binaries they were read from, and a later release does not inherit them. 2.1.259 is
no longer on this machine; rows a25 read from 2.1.288 (`2026-10-03-native-claude-fixture.md`) are
re-read here, not carried over.

Each row carries one of three labels:

| label | means |
|---|---|
| **observed** | a run of the named binary in this work shows it |
| **binary** | read from the named binary's strings or its `--help`, not driven |
| **unverified** | neither; stated so that nobody reads silence as a check |

A row that names no version holds for both binaries, each read or run on its own.

## The binaries

| fact | 2.1.304 | 2.1.305 |
|---|---|---|
| version (`--version`, exit 0, offline scratch home) | `2.1.304 (Claude Code)` | `2.1.305 (Claude Code)` |
| file under `~/.local/share/claude/versions/` | 288,730,480 bytes, SHA-256 `ea0f135784631619a7a409d07322ecec964f9cc4fd377508ed1ba3f141f27912` | 290,532,720 bytes, SHA-256 `86cb722ab68348877dba7971e4d0c659919e05e6228c05e006e83da320e656ef` |
| installed `claude` | until 2026-10-20 23:13 | since then |
| `--help` | 313 lines | byte-identical to 2.1.304's |

Both were always run by explicit path, in a user, network and mount namespace with only `lo`, with
a fresh scratch `HOME` and `CLAUDE_CONFIG_DIR` per invocation and an empty directory bound over the
operator's `~/.claude`; never logged in, never pointed at a real provider. 2.1.303
(SHA-256 `375988f457756524234695cf2d8f64ace99c5f68682e47ea24862a843c191450`) is on disk, stays off the
pin, and was only asked for `--version`. The driver is `target/debug/metaharness` built from this
change (`cargo build --locked --bin metaharness`), SHA-256
`e820b748104a7849275eb4ed448c71c879937848263ea78ab7ab81ff51efa24a`.

## The native fixture

`crates/metaharness-claude/tests/native_fixture.rs`, ignored in `task check`, run in a loopback-only
namespace as `2026-10-03-native-claude-fixture.md` § *Reproduction* describes, with
`METAHARNESS_NATIVE_CLAUDE` set to one binary per lane, `METAHARNESS_NATIVE_CLAUDE_OFF_PIN` to 2.1.303,
and the operator's `~/.claude` hidden (`unshare --mount`, an empty directory bound over it). Lanes:
**2.1.304 7 passed, 0 failed** (13.28 s); **2.1.305 7 passed, 0 failed** (11.63 s); 24 run
directories each: 22 sessions and the 2 refused 2.1.303 launches. Two credential modes, both
credential-free:

| mode | what the child holds | what the stub admits |
|---|---|---|
| `none` (`--model-endpoint` at the stub) | the placeholder `ANTHROPIC_API_KEY=metaharness-model-endpoint` | that `x-api-key` and no `Authorization` |
| `loopback` (`--model-endpoint` is the proxy's upstream) | the per-run placeholder bearer; the scratch `HOME` holds a fake `.credentials.json` written by the test, token issued by no account | only `Authorization: Bearer <that fake token>`, attached by metaharness's proxy, and no key |

| observation, each binary | `none` | `loopback` | label |
|---|---|---|---|
| opening record reports the binary's own `claude_code_version` | yes | yes | observed |
| success: fixture answer as `text`, native exit 0, `stream.closed` last, CLI exit 0 | yes | yes | observed |
| provider HTTP 400: terminal `is_error: true`, native exit 1, CLI exit 3 | yes | yes | observed |
| usage: assistant 3/0, terminal 3/4 (synthetic numbers the stub supplies) | yes | yes | observed |
| `PreToolUse` hook (matcher `""`) holds a real `Bash` call; allow writes the marker, deny leaves it absent and the result `is_error: true` | yes | yes | observed |
| interrupt and halt close a run waiting on a quiet provider within 2 s | yes | yes | observed |
| `--max-turns 1` against a provider that keeps asking for tools ends `error_max_turns` | yes | yes | observed |
| requests the stub saw | one `POST /v1/messages?beta=true` per turn; no `HEAD /api/hello`, no `count_tokens` (2.1.288 sent the `HEAD` first) | the same, every one through the proxy | observed |
| MCP list in the opening record | `[]` | `[]` | observed |
| plugins in the opening record | `cc-plugin-agents-md`, `cc-plugin-plugin-authoring`, each `<name>@builtin` (22 of 22 records per binary) | the same | observed |
| `--strict-version` with the binary on the child's `PATH` | the run proceeds | the same | observed |
| `--strict-version` with 2.1.303 on the child's `PATH` | CLI exit 2 before the spawn: *"--strict-version requires one of ["2.1.304", "2.1.305"]; …/claude reported 2.1.303"*; nothing reached the stub, no `session.started` | the same | observed |
| floor with `--audit`: H9 | `ok` — *the vendor is <version>, which the adapter is pinned to* | `ok` | observed |
| floor with `--audit`: H4 | `ok`: the record says `"ANTHROPIC_API_KEY"`, and the launch attests it holds its own endpoint placeholder | `ok` (`"none"`) | observed |
| floor with `--audit`: H1a | `ok`, naming `cc-plugin-agents-md@builtin` and `cc-plugin-plugin-authoring@builtin` as built-ins the pinned binary compiles in | the same | observed |
| floor with `--audit`: H7, H11 | `gap`: the fixture names its own `--cwd` (amendment a6) | the same | observed, by design |

## Built-in plugins

| fact | label |
|---|---|
| each binary registers exactly twelve built-ins, in one function guarded by `builtinPluginsInitialized`: `cc-plugin-sec-default`, `cc-plugin-agents-md`, `cc-plugin-telemetry`, `cc-plugin-plugin-authoring`, `cc-plugin-mods-guide`, `cc-plugin-tips`, `cc-plugin-mermaid`, `cc-plugin-responsive-mode`, `cc-plugin-diff`, `cc-plugin-you-should-know`, `cc-plugin-easel`, `cc-plugin-claude-test`, in that order; 2.1.303 registers the same twelve. The registration guards nine of them on the entrypoint (not `local-agent`) and some on a further check; which flag or policy enables each in a session was not read | binary |
| a record gives a built-in the source `<name>@builtin`; the binary's schema for the `plugins` field says so (*"`name@builtin` for built-in plugins"*) and every built-in in every fixture record carried it | binary, observed |
| `claude plugin list --json` prints `[]`; it never lists built-ins | observed (2.1.304) |
| `claude plugin details <name>@builtin` in a fresh offline scratch home resolves three: `cc-plugin-agents-md`, `cc-plugin-telemetry`, `cc-plugin-plugin-authoring`; with `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1` it resolves two (`cc-plugin-telemetry` is gone) | observed, each binary |
| `cc-plugin-sec-default` is seated on a machine with managed settings and for Team and Enterprise organizations; the vendor's own debug log (`--debug-file`, a direct offline 2.1.304 run) said *"cc-plugin-sec-default@builtin not seated: no managed settings and not a Team or Enterprise organization (none)"* | binary, observed (2.1.304) |
| an operator-login 2.1.304 session opened with four: `cc-plugin-sec-default`, `cc-plugin-agents-md`, `cc-plugin-telemetry`, `cc-plugin-plugin-authoring`, each `@builtin` | **unverified** here: relayed from takt's own `system/init` record, not read in this work; nothing relayed for 2.1.305 |
| a built-in is compiled in, so a scratch config home cannot remove it | binary |

What the launch does about each, and the H1a rule, are design amendment a33 and `BUILTIN_PLUGINS`'
own table; the list is keyed by version and a test holds it to `PINNED_VERSIONS`. The traffic switch
is set under `none` and `loopback` and not under an operator login or an API key, because it also
gates account-keyed behaviour (the served model catalog's published path, claude.ai plugin
downloads, Projects, the built-in `gh api`) — read from the 2.1.304 binary, the switch's mode
function (`essential-traffic`) present in 2.1.305 too, and not observable without a credential.

## H11: memory files

The fixture planted a canary of each memory shape in `work/` (the cwd) and in the run root (its
parent), then searched every request body the stub recorded. The shapes are the binary's own list
(`[["CLAUDE.md"],["CLAUDE.local.md"],["AGENTS.md"],[".claude","CLAUDE.md"],[".claude","AGENTS.md"],[".claude","rules"]]`,
the same string in both binaries).

| run, each binary | `CLAUDE.md`, `.claude/CLAUDE.md`, `CLAUDE.local.md`, `.claude/rules/canary.md` | `AGENTS.md`, `.claude/AGENTS.md` | label |
|---|---|---|---|
| the vendor run directly, `--setting-sources project,local` (the positive control) | **all 8 canaries** (each shape, cwd and ancestor), as project instructions in the first user message | **all 4**, planted where no `CLAUDE.md`-family file sits in or above the cwd | observed |
| the production launch (`--setting-sources ""`), `none` | none | none | observed |
| the production launch, `loopback` | none | none | observed |

So at both pins no memory file of any shape in the cwd or an ancestor reaches the model under the
launch, and the control shows the probe would have seen each one. The mechanism, from the binary:
project-memory discovery returns nothing unless the `projectSettings` source is on
(`CLAUDE.local.md`: `localSettings`), `cc-plugin-agents-md`'s ancestor walk is off on the same
condition and loads `AGENTS.md` only for a project with no `CLAUDE.md` of its own, and the
`--setting-sources` parser still reads `""` as no sources (`if(e==="")return[]` in both, V12's reading
re-read).

**A correction to the first pass of this page.** It recorded the `AGENTS.md` half as having no
positive control: `cc-plugin-agents-md` had sent nothing with the project source on. That control ran
under the operator's real `HOME`, where `~/.claude/CLAUDE.md` sits above every scratch directory — a
`CLAUDE.md`-family file above the cwd, so the plugin's fallback never applied, and the vendor sent
the operator's own `~/.claude/CLAUDE.md` into the control's request instead (14 control runs, all to
the local stub; the 58 runs through the launch carried no memory file at all). The lane now hides
`~/.claude` and asserts that no memory shape sits above its evidence root before it plants anything.
That is the same file a walk widened to these shapes finds: built that way within this unit, the
walk refused every scratch Claude run on this machine (plain `cargo test --workspace`: 842 passed,
80 failed, every failure tracing to `~/.claude/CLAUDE.md`), so it was reverted. The walk
looks for `CLAUDE.md` and `AGENTS.md` only; it does not look for the other four shapes, which
`story:claude-h11-memory-walk-shapes` (decided: option C) covers (a33 § 6).

User memory (`CLAUDE.md` in the scratch config home) and managed memory were not probed. The relayed
2.1.305 observation (a `--cwd` canary `CLAUDE.md` not quoted under `--hermetic off`) agrees and was
not reproduced as such.

## Every adapter claim, at 2.1.304 and 2.1.305

| claim (where it is made) | label |
|---|---|
| `-p`, `--output-format stream-json`, `--verbose` are declared, and stream-json needs `--verbose` (`build_args`) | binary (help, both); observed (every fixture run streams) |
| `--strict-mcp-config` *"Only use MCP servers from --mcp-config, ignoring all other MCP configurations"* (H5) | binary (help, same words, both); observed (MCP list `[]`) |
| `--setting-sources ""` means no sources (H2, V12) | binary (both parsers); observed for the project and local sources (the canary control) |
| `--settings <file>` loads the hook definition (`SETTINGS_FILE`) | observed (the hook fired from it) |
| `--model`, `--effort`, `--max-budget-usd`, `--plugin-dir`, `--tools`, `--mcp-config`, `--allowedTools` are declared | binary (help, both) |
| `--max-turns` is accepted | binary: registered with `.hideHelp()` in both — **absent from `--help`**, present in the binary; observed (turn ceiling) |
| `--version` prints `<version> (Claude Code)` and is what `--strict-version` and `doctor` read | observed (both, and 2.1.303) |
| `--strict-version` gives `--version` 5 s, then kills the child and refuses by name | unit test only (a fake binary that sleeps); neither pinned binary is that slow |
| `--model` reaches the opening record | observed |
| `--effort`, `--max-budget-usd` change behaviour | unverified (not driven) |
| `--plugin-dir` loads a plugin for one session (H1a's declared half, Q19) | unverified at both (not driven; last driven 2026-09-03 on 2.1.259) |
| `--tools ""` removes every built-in tool and `--mcp-config` + `--allowedTools mcp__metaharness` serve the owned surface (strategy C, V11) | binary (help: *"Use \"\" to disable all tools"*); unverified driven |
| a bare `--allowedTools` entry auto-approves before the hook (V4) | unverified (never driven on any version) |
| `--bare`, `--safe-mode` delete the seam; `--dangerously-skip-permissions`, `--add-dir` are refused (H8) | binary (help: `--bare` *"skip hooks … CLAUDE.md auto-discovery. Sets CLAUDE_CODE_SIMPLE=1"*; `--safe-mode` *"all customizations (CLAUDE.md, … hooks …) disabled"*) |
| `CLAUDE_CONFIG_DIR` is the scratch home (H1b) | observed: H1b `ok` in every audit run (output style default) |
| `ANTHROPIC_BASE_URL` plus the placeholder key sends every API request to the base (MA-V1) | observed (`none`) |
| the record's `apiKeySource` under that placeholder is `"ANTHROPIC_API_KEY"` (H4, a33 § 5) | observed |
| managed settings on Linux are `/etc/claude-code/managed-settings.json` and the `managed-settings.d` drop-in directory beside it (`Gh()` returns `/etc/claude-code` off macOS and Windows), plus the same pair in `/mnt/c/Program Files/ClaudeCode` under WSL; `--setting-sources ""` does not switch them off, and their `env` or `apiKeyHelper` reaches the child (H4, a33 § 5) | binary (2.1.304); the falsification of the placeholder claim was driven by the adversary, not in this work; none of these paths exists on this machine |
| `ANTHROPIC_AUTH_TOKEN` is sent as a bearer and the proxy relays it with custody (LP-3) | observed (`loopback`) |
| `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` closes first-party analytics | binary: setting it selects the `essential-traffic` mode the features above read; its effect on `cc-plugin-telemetry` observed. That no packet left for `api.anthropic.com` is **not** observed: the namespace has no route, so nothing could |
| a `PreToolUse` `type: command` hook with matcher `""` holds every call and its `permissionDecision` is obeyed (V7b, V8, Q11) | observed for `Bash`, allow and deny |
| the vendor's behaviour when the hook exceeds its timeout (Q10) | unverified |
| the transcript shapes the reader maps: `system/init` (version, model, cwd, tools, plugins with `name`/`path`/`source`, `mcp_servers`, `apiKeySource`, `output_style`), assistant `tool_use`, user `tool_result`, `result` with usage | observed |
| `system/task_*`, `tool_progress`, `background_tasks_changed`, `api_retry` (read from 2.1.259's schema) | unverified at both (no fixture produces them) |
| the H11 walk looks for `CLAUDE.md` and `AGENTS.md` only; the loader also reads `.claude/CLAUDE.md`, `CLAUDE.local.md`, `.claude/AGENTS.md`, `.claude/rules/**/*.md` from an ancestor (a33 § 6) | binary (both); each observed as loaded with the sources on and kept out under the launch; the walk does **not look for** the four extra shapes (`story:claude-h11-memory-walk-shapes`, decided: option C) |
| the golden transcript and hook input (`fixtures/golden/`) | unchanged 2.1.240 bytes; `golden-version-pair` warns by design |
| `cc-plugin-sec-default` is the fourth built-in of an operator login | unverified here (relayed, 2.1.304 only) |

## Private evidence

Under `~/.cache/metaharness-wave-adr0096/w4/u2-scratch/`: `native-evidence/` (one directory per
vendor process: events, stderr with the audit, recorded request bodies, provider errors),
`evidence/` (help text of both binaries, registration-function excerpts for 2.1.303/304/305, the
`plugin details` tables, the lane summaries). No transcript or request body is committed. The 14
first-pass control directories whose recorded requests carry the operator's `~/.claude/CLAUDE.md`
are named in the implementor's report for deletion.
