# Claude Code offers `Glob` and `Grep` only when the launch names them

Status: read on 2026-10-23 against Claude Code **2.1.305** and **2.1.309** (pinned) and **2.1.307**
and **2.1.308** (off the pin), with **no paid run, no login and no credential**. It is the evidence
behind design amendment a53 and closes the open item a51 § 7 names
(`story:claude-dir-list-search-unregistered`). It is dated, on a8's rule: a later release does not
inherit these rows. 2.1.304 is not on this machine and was not run; its `NATIVE_TOOLS` row is
**unverified**.

Labels, as on the 2.1.309 page: **observed** (a run of the named binary shows it), **unverified**
(not run, stated so silence is not read as a check).

## Method

Every binary was run by explicit path, in a user, network and mount namespace with only `lo`, with
a fresh scratch `HOME` and `CLAUDE_CONFIG_DIR` per invocation and an empty directory bound over the
operator's `~/.claude`. The launch was the native one without metaharness:
`-p hello --output-format stream-json --verbose --setting-sources "" --strict-mcp-config
--max-turns 1`, with `ANTHROPIC_BASE_URL=http://127.0.0.1:9` (a closed port),
`ANTHROPIC_API_KEY=metaharness-model-endpoint` (the launch's placeholder) and
`CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1`. The offered set is the `tools` array of the
`system/init` record, which the vendor writes before its first model request. Every case exited
124 (the 40 s timeout on the closed port) after writing that record, so the record is what was
read and nothing reached a model.

The binaries are the ones the 2.1.309 page records: 2.1.305 SHA-256 `86cb722a…656ef`, 2.1.309
`4d704a8a…b658`, 2.1.308 `356733f1…ad58`, 2.1.307 `6021eb72…dd0e`.

`BASE` below is the 21 names a default session opens with: `Task, Bash, CronCreate, CronDelete,
CronList, Edit, EnterWorktree, ExitWorktree, ListAgents, NotebookEdit, Read, ReportFindings,
ScheduleWakeup, SendMessage, Skill, TaskStop, ToolSearch, WebFetch, WebSearch, Workflow, Write`.

## Without the switch

| case | 2.1.305 | 2.1.309 | 2.1.307 | 2.1.308 |
|---|---|---|---|---|
| no switch | `BASE` (21), no `Glob`, `Grep`, `TodoWrite` | same | same | same |
| `EMBEDDED_SEARCH_TOOLS=0` | `BASE` (21) | `BASE` (21) | not run | not run |
| `EMBEDDED_SEARCH_TOOLS=1` | `BASE` (21) | `BASE` (21) | not run | not run |
| `CLAUDE_CODE_USE_NATIVE_FILE_SEARCH=0` | `BASE` (21) | `BASE` (21) | not run | not run |
| `CLAUDE_CODE_USE_NATIVE_FILE_SEARCH=1` | `BASE` (21) | `BASE` (21) | not run | not run |
| `USE_BUILTIN_RIPGREP=1` | `BASE` (21) | `BASE` (21) | not run | not run |

No environment switch tried brings `Glob` or `Grep` back. All rows **observed**.

## With the switch

| case | 2.1.305 | 2.1.309 | 2.1.307 | 2.1.308 |
|---|---|---|---|---|
| `--tools "Read,Glob,Grep"` | `Glob, Grep, Read` (3) | same | not run | not run |
| `--allowedTools "Glob,Grep"` | `BASE` + `Glob`, `Grep` (23) | same | not run | not run |
| `--allowedTools "Glob(/nonexistent/**)"` | `BASE` + `Glob`, `Grep` (23) | same | not run | not run |
| `--tools "default,Glob,Grep"` | `Glob, Grep` (2) | same | not run | not run |
| `--tools "default Glob Grep"` | `Glob, Grep` (2) | same | not run | not run |
| `--tools "<BASE>,Glob,Grep"` | `BASE` + `Glob`, `Grep` (23) | same | not run | not run |
| `--tools "<BASE>,Glob,Grep,TodoWrite"` | 23, no `TodoWrite` | same | same | same |
| `--tools "<BASE>,Glob,Grep,NoSuchTool"` | 23, empty stderr, no refusal | same | same | same |

All rows **observed**. `Glob` and `Grep` appear in alphabetical position in the init list, not at
the end. What follows from the table:

- Naming either tool in `--tools` or `--allowedTools` is the opt-in. `--allowedTools` also adds
  bare allow rules, which shadow the `PreToolUse` seam, so a53 rejects it.
- `default` is not expanded inside a list: it offers only the explicit names.
- `TodoWrite` is not offered even when named. Why is **unverified**: nothing here shows it.
- An unknown name is dropped silently. A stale list fails no launch, so the fixture checks the
  offered set against the row instead.

## The native fixture lane

`crates/metaharness-claude/tests/native_fixture.rs`, run with `--ignored --test-threads=1` inside
the same namespace (`METAHARNESS_NATIVE_NETNS=loopback-only`), the stub `/v1/messages` server in
the test, and the driver built from this change (`cargo build --locked --bin metaharness`). The
off-pin binary is 2.1.308. Every case runs under `credentials: none` and `credentials: loopback`.

| binary | driver | result |
|---|---|---|
| 2.1.309 | this change | 10/10 passed |
| 2.1.305 | this change | 10/10 passed |
| 2.1.309 | the driver the 2.1.309 page records (SHA-256 `7f985eeb…acb8`, no `--tools`) | 9/10; `native_launch_offers_its_pinned_tools_and_the_hook_decides_glob` fails, the stub recording `native request did not offer Glob` on every request |

The new case asserts that the opening record's `offered_tools` equals the pinned `NATIVE_TOOLS`
row, that every rendered operation's tool is in it except `TodoWrite`, and that one scripted
`Glob` turn produces `tool.requested` with `name: Glob`, `operations: ["dir.list"]`,
`seam: hook` and `decision_required: true`, then an `allow` `tool.decided` and a `tool.result`
listing the file the test planted. All **observed**, on both pinned binaries.

## Open

- 2.1.304: **unverified**; its row is carried on a33's evidence that 2.1.304 and 2.1.305 register
  the same built-ins.
- Why `TodoWrite` is never offered: **unverified**.
- Whether `--tools` also filters tools contributed by plugins: **unverified**, not probed.
