---
id: claude
title: Claude Code
sidebar_position: 1
---

# The `claude` adapter

Pinned to **2.1.304** and **2.1.305**. Adapter class: `harness` — the vendor keeps its loop, its sessions, its
tools, its authentication and its credential custody; metaharness drives its documented outside
surface.

```bash
metaharness run claude --hermetic -p "…"
metaharness conformance claude    # 30 vectors, no model / no network / no credential
metaharness doctor claude
```

## What a run does

1. Spawns `claude` into a **scratch config home**.
2. Installs a blocking `PreToolUse` hook and answers its calls **per call**.
3. Streams the session out as protocol events on stdout; takes steering on stdin.
4. Retains the raw transcript.
5. Exits on the [hermetic floor's](../hermetic.md) verdict.

:::warning 2.1.304 and 2.1.305 read the files a prompt mentions, and nothing records it
Claude Code expands an `@path` in prompt text — the `-p` argument, stdin, a stream-json user
message — into a synthesised Read result in the request it sends: no tool call, no `PreToolUse`
hook, no `tool.requested`, no line in the transcript. Paths outside the working tree, through
symbolic links, untracked files and directories expand alike, under every hermetic mode. Measured
against the native fixture's stub endpoint (design amendment a35). **This holds for the task you
pass with `-p`**: an `@path` in it is read with nothing to show for it.
:::

`--context <file>` therefore does **not** use the user turn. The declared files go to the value of
`--append-system-prompt`, verbatim inside tags whose closing lines they do not contain, introduced
as reference data and not instructions; `-p` carries the task alone. Neither pinned binary expands a
mention there, and the block arrives byte-exact in the request's `system` field. The trade is that
the files' text carries system-prompt authority, and `session.started.context_files` names each
file with `"channel": "system-prompt"` so a reviewer can see which text that was. The checks are on
[the CLI reference](../reference/cli.md#--context-files-the-run-is-handed).

| route for `@marker.txt` | 2.1.304 | 2.1.305 |
|---|---|---|
| the `-p` argument | expanded | expanded |
| stdin, with `-p` | expanded | expanded |
| stdin, `--input-format stream-json` | expanded | expanded |
| `--append-system-prompt` | not expanded | not expanded |
| `--system-prompt` | not expanded | not expanded |

The adapter records this per pinned version (`MENTION_CANARIES`). A pin without a row fails a unit
test; a row copied from the previous pin passes it, so the measurement is the native fixture's
canary case, run beside the H11 canary on each pin move — `task check` does not run it.

## Built-in plugins

Claude Code compiles plugins into its binary, so no scratch config home can remove them. 2.1.304
and 2.1.305 each compile in the same twelve, and the adapter declares each one per version, by name,
with the source a record gives it, `<name>@builtin`. H1a reads a plugin in the opening record as
`ok` only when it is a plugin the run declared or one of the twelve the reported version compiles
in, **by name and source**, and the row names every built-in it saw. A plugin that borrows a
built-in's name from anywhere else stays a `gap`, and so does any built-in in a record on an
unpinned version.

| Credential mode | Built-ins in a 2.1.304 or 2.1.305 opening record | What the launch does |
|---|---|---|
| `none`, `loopback` | `cc-plugin-agents-md`, `cc-plugin-plugin-authoring` (observed on both) | sets `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC`, which keeps `cc-plugin-telemetry` out |
| operator login | also `cc-plugin-telemetry` and, for Team and Enterprise organizations, `cc-plugin-sec-default` (relayed for 2.1.304, not driven here) | admits them by name and `@builtin` source; the traffic switch is not set under a real credential |

`--strict-version` reads the resolved binary's `--version` before the spawn and refuses one that is
not pinned; 2.1.303 is refused.

Memory files: at both pins no memory file in the working directory or above it reached the model
under the launch's `--setting-sources ""` — `CLAUDE.md`, `.claude/CLAUDE.md`, `CLAUDE.local.md`,
`.claude/rules/*.md`, `AGENTS.md` or `.claude/AGENTS.md`, twelve canaries in the cwd and its parent,
all twelve sent by the same binary with the project and local sources on. H11 rests on that control
and the fresh scratch config home: the adapter records the canary per pinned version
(`MEMORY_CANARIES`), and the floor reads H11 `ok` only on a record version whose canary kept every
shape out; a version with no canary, or one whose canary reached a request, reads `gap`. The ancestor
walk looks for all six shapes and names what it finds in the H11 row; it refuses nothing. User memory
(a `CLAUDE.md` in the config home) and managed memory were not probed. Design amendments **a33** and
**a36**; evidence in `docs/research/2026-10-21-claude-code-2.1.304-adapter-surface.md`.

## Declared capabilities

| Tier | Status |
|---|---|
| `registration` | `delivered` |
| `call` | `delivered` |
| `turn` | `unverified` |
| `kill` | `delivered` |

| Command | Support |
|---|---|
| `tool.decide` | honoured |
| `interrupt` | honoured |
| `halt` | honoured |
| `frame.set`, `message.inject`, `steer`, `permission.set` | refused — `UNSUPPORTED_CONTROL` |

`steer` is refused because **Claude Code headless has no mid-turn tier**. That is a property of
the surface, not an omission in the adapter.

## Operation rendering

| Neutral operation | Vendor tool |
|---|---|
| `file.read` | `Read` |
| `file.write` | `Write` |
| `file.edit` | `Edit` |
| `dir.list` | `Glob` |
| `search` | `Grep` |
| `shell` | `Bash` |
| `web.read` | `WebFetch` |
| `skill.load` | `Skill` |
| `subagent.spawn` | `Task` |
| `task.todo` | `TodoWrite` |
| `mcp.call` | *(none)* |

Print it yourself: `metaharness capabilities claude --render`.

## Three things this crate deliberately does not do

| | |
|---|---|
| **It spawns nothing.** | `plan_launch` returns the command line, the child environment, the settings document, the hook definition and the copy list as **values** — because a consumer asserts its own argv rather than trusting it, *"because every one of the failures would be silent"*. The hook program is the same rule applied to the seam's executable. |
| **It reads no clock.** | Every event's timestamp is the one the vendor recorded, passed through — or absent. |
| **It drops nothing.** | An unmappable record becomes `opaque` with the vendor's `type`, `subtype`, a digest and its 1-based source line. An unrecognised *field* is ignored in silence; an unrecognised *record* is preserved. |

## The live proof

A frame that admitted no shell met a prompt that asked for one.

- metaharness denied the call at the hook.
- The call did not run.
- **The vendor's own terminal record** listed `Bash` in `permission_denials`.

### Two defects that only a paid run could find

Both were metaharness's, and neither was reachable without a real session:

1. The hermetic floor read **the wrong field** for "was an API key in use".
2. It treated *"this run pinned no documents"* as *"nobody found out whether the documents
   moved"* — which made `--hermetic strict` unpassable.

Both are fixed, both are regression-tested, and both are recorded as design amendment **a4**.

## Evidence discipline

Every claim this adapter makes about the binary is either read from the design document's § 2.7 —
where each row carries its own method — or **labelled unverified at the point of use**, with the
open question that would close it.

> A string in a binary is weaker than a driven call, and this crate says which it has.
