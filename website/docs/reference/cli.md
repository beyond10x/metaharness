---
id: cli
title: CLI reference
sidebar_position: 1
---

# CLI reference

```text
metaharness <VERB> [OPTIONS]
```

Six verbs. Three of them need no model, no network and no credential.

| Verb | What it does |
|---|---|
| `run <KIND>` | Run a harness session: events as JSON lines on stdout, commands as JSON lines on stdin. |
| `capabilities <KIND>` | What an adapter says it can do: declared tiers, pinned versions, operation rendering. |
| `conformance <KIND>` | The free conformance vectors: no model, no network, no credential. |
| `doctor <KIND>` | The installed vendor version against the adapter's pin. |
| `project <EVENTS>...` | Project an event stream into `trace-ir/1`, byte-stable, or render one or two runs as one static page (`--html`); `--from <FORMAT>` reads a vendor record through its adapter's production reader instead (design amendment a28). |
| `audit --transcript <FILE>` | Judge a transcript offline. **Refuses with exit 2**. |

`<KIND>` is `claude`, `codex` or `b10x`. The first two are vendors metaharness drives; the third is
the beyond10x loop, which it observes — see [the b10x adapter](../harnesses/b10x.md).

## `metaharness run`

### The run itself

| Flag | Default | Meaning |
|---|---|---|
| `<KIND>` | — | Which harness. |
| `-p, --prompt <TEXT>` | — | The prompt to start with. |
| `--model <NAME>` | — | The model to ask for. Passed through to the vendor, which resolves it. |
| `--max-turns <N>` | — | A ceiling on turns. |
| `--cwd <DIR>` | scratch | A named working directory for the child, **which it can write to**. Alone, the operator's real tree: trades two hermetic rows for real work — see below. |
| `--scratch-cwd` | off | Declares the `--cwd` directory as the embedder's **disposable scratch copy**. H7 is attested `declared`, H11 is attested from the copy as over a scratch cwd, and `--hermetic strict` passes on the pair when the record's cwd is the copy; on b10x that is the directory the loop reports, and where it reports none (`--substrate <socket>`) H7 is `unk` and it exits 3 — see below. Refused without `--cwd`. |
| `--context <FILE>` | *(none)* | A file the run is handed before it starts, named relative to the `--cwd` tree. Repeatable, **every kind**. See below. |

### `--context`: files the run is handed

Each file is opened beneath the `--cwd` tree without following a symbolic link, read once, and the
same bytes are digested and handed over:

| kind | where the files go | `channel` in the record |
|---|---|---|
| `claude` | the value of `--append-system-prompt`; `-p` carries the task alone | `system-prompt` |
| `codex` | inline in the prompt, ahead of the task | `user-prompt` |
| `b10x` | metaharness's copy of the bytes, through the loop's own `--context`; the loop wraps the file itself (`<context kind="provided_context" trust="operator" …>`, no preamble of ours) in its standing instruction | `system-prompt` |

On `claude` and `codex` the list is introduced as reference data, not instructions, and wrapped in
`<context-files-N>`; each file sits between `<context-file-N path="…" bytes="…" sha256="…">` and
`</context-file-N>`; each `N` is the smallest number whose closing line no file (for the wrapper)
or not this file (for the file) contains. On `b10x` the loop's own rendering applies, and it
escapes `<` and `&` in the file's text, so the model reads the loop's escaped form of the digested
bytes. `session.started.context_files` lists each file's `path`, `sha256`, `bytes` and `channel`,
and the key is absent when no file was declared.

Claude Code is not given the files in its user turn because 2.1.304 and 2.1.305 turn an `@path`
there into a file read that no tool call, hook or record shows — see
[the claude adapter](../harnesses/claude.md). The task text is still the user turn, and the same
holds for an `@path` in it.

Refused before the spawn, naming the path: no `--cwd`; an absolute path or one with `..`; a path
that is or passes through a symbolic link; something other than a regular file; content that is not
UTF-8 text; a file git's index does not name in the tree (a `.git` gitfile, as an embedder's scratch
copy has, is followed; nothing is fetched to answer); more than **16** files; more than **65,536**
bytes of content and rendered path together. The byte cap bounds what the files add, not the task:
every argument the launch builds is then measured against Linux's limit on one argument (131,072
bytes with its terminating NUL, `MAX_ARG_STRLEN`), and one that does not fit is refused by name —
the argument, its size and each context file with its size — before the spawn.

### Hermeticity

| Flag | Default | Meaning |
|---|---|---|
| `--hermetic [off\|on\|strict]` | `off`; bare `--hermetic` means `on` | See [the hermetic contract](../hermetic.md). |
| `--credentials <SOURCE>` | `operator-login` | `operator-login`, `api-key`, `loopback`, or `none`. |
| `--plugin-dir <DIR>` | *(none)* | Plugin directories to load, **and only these**. Repeatable. Copied into the scratch tree and digested before the child starts; on `b10x`, handed to the loop as its own `--plugin-dir`. |
| `--strict-version` | off | Refuse before the run when the installed vendor version is outside the adapter's pin, instead of warning. |

**`--credentials none`** means the run is expected to fail at the first request — and that is
sometimes exactly what a test wants.

### Model, effort and actor

| Flag | Meaning |
|---|---|
| `--model-endpoint <BASE_URL>` | A model gateway, as its **root** URL (no `/v1`); each harness reaches its own dialect under it. |
| `--model-wire <WIRE>` | Which API the harness speaks under that root — only `b10x` has a choice: `openai-responses` or `anthropic-messages`. |
| `--subscription-token-file <FILE>`, `--subscription-token-env <NAME>`, `--subscription-token-pointer <POINTER>` | A subscription token for a route that takes one instead of an API key; the pointer selects a field inside a JSON source. Not interchangeable with `--credentials api-key`. |
| `--effort <LEVEL>` | Reasoning effort in the vendor's vocabulary, passed through and validated by the vendor. |
| `--actor <ACTOR>` | Who the run's store writes are made *as*, carried to the child as `AEP_ACTOR` — declared, never inherited. |
| `--retain-dir <DIR>` | Copy the raw vendor wire out of the scratch root before it is deleted. |

**`--credentials api-key`** is a declaration: without it, `ANTHROPIC_API_KEY` is not in the child
environment at all.

**`--credentials loopback`** puts metaharness in front of the vendor: the child holds a per-run
placeholder and **no credential file**, and one custody attaches the real token on the way out. On
Claude Code the child is pointed at the proxy by `ANTHROPIC_BASE_URL`; on codex by a
`[model_providers.metaharness_loopback]` entry in the scratch `CODEX_HOME`. A **ChatGPT-plan** codex
login is refused by name — see [the codex adapter](../harnesses/codex.md).

### Control

| Flag | Default | Meaning |
|---|---|---|
| `--frame <FILE>` | — | A sealed `metaharness.frame/1` document. See [frames](../frames.md). |
| `--decisions <MODE>` | `frame` | `frame` (the adapter decides from the admitted set) or `ask` (the embedder decides; one round trip per call). |
| `--tool-surface <SURFACE>` | `native` | `native`, or `owned` — which this build **refuses** on `codex` and `b10x`. |
| `--catalogue <FILE>` | *(none)* | A caller's `tool-contract/1` document whose entries are the run's whole tool set. **`claude` only**; implies `--tool-surface owned`. See below. |

The frame is a **path**, not a parsed value, on purpose: resolving it is the library's job, and
parsing it in the binary would put protocol logic in the CLI.

### `--catalogue`: the caller's tools instead of the vendor's

On `claude` the launch removes the built-in tools (`--tools ""`) and names one MCP server,
`metaharness mcp-serve --catalogue <copy>`, which lists each entry under its `mcp` rendering and
nothing else, instead of the verbs the owned surface lists by default; Claude Code offers each as
`mcp__metaharness__<name>`. A call to anything else, or to an entry this build does not perform,
resolves to no operation, so a frame denies it as `UNCOVERED_TOOL`. The file is read and checked
once, before the frame and before the spawn, and the server reads a scratch copy of the checked
document. That copy is JSON with the characters YAML refuses raw or reads as line breaks written
as `\u` escapes, and it is read back before the spawn. A leading byte-order mark in the file is
dropped before it is parsed. A live launch with a catalogue has not been driven; the vector that
pins it is synthetic (design amendment a40).

A `tools/call` to an entry answers `isError` and performs nothing when its arguments are not an
object, carry a key that is not one of the entry's input fields, or leave out (or give as null) a
field the entry requires. The answer names the entry and the field.

Refused before the spawn with exit 2, by name: `catalogue-not-carried` on `codex` and `b10x`;
`catalogue-needs-owned-surface` beside `--tool-surface native`; `catalogue-with-allow-program`
beside `--allow-program`, because a server handed a catalogue publishes no `run` for the programs
to be started by; `catalogue-unreadable` for a path that is not a regular file, cannot be read, is
past 4 MiB or is not UTF-8; `catalogue-not-tool-contract` or `catalogue-malformed` for a file that
is not a catalogue, and `catalogue-malformed` as well for one whose scratch copy would be past
4 MiB or would not read back as the same catalogue; `catalogue-copy-differs` when the copy written
into the scratch tree reads back as anything but the checked catalogue. `mcp-serve --catalogue <FILE>` serves the same catalogue on
its own, and refuses `--allow-program` beside it with the same code.

`catalogue-copy-differs` is the one catalogue refusal that comes after something has started. It
fires after the scratch files are written, after a loopback proxy has started when the run uses
one, and under `--strict-version` after the vendor binary has been run with `--version`. Its
message says which of these had happened. No vendor session is started.

### Audit

| Flag | Meaning |
|---|---|
| `--audit` | Judge the run: the built-in hermetic floor always, the external auditor when `--spec` is given. |
| `--spec <FILE>` | The expectation document the external auditor is pointed at. |
| `--auditor <PREFIX>` | The external auditor, as an **argv prefix**. |
| `-- <AUDITOR_ARGS>…` | Arguments passed through to the auditor after everything metaharness adds. |

:::danger A `--spec` with no `--auditor` is a refusal, not a skip
A specification nobody checked reads exactly like a specification that passed.
:::

`run` checks the pairing when it starts, before the scratch tree, any proxy or the harness
process. Each case below exits `2` with no harness process started and no provider request made.
The first case that matches is the one reported:

| The run asked for | The refusal says |
|---|---|
| `--spec` and no `--auditor`, with or without an audit | `--spec was given without --auditor` |
| `--auditor` and no `--spec`, with or without an audit | `--auditor was given without --spec` |
| arguments after `--` and no `--auditor`, with or without an audit | `auditor arguments after -- were given without --auditor` |
| `--spec` and `--auditor`, with no `--audit` and `--hermetic` not `strict` | `--spec and --auditor were given on a run that wants no audit` |
| an expectation document whose path is not UTF-8, or that is missing, is not a regular file, or cannot be opened | `the expectation document <FILE> could not be read` |
| an `--auditor` prefix that is empty or only spaces, or whose first word starts with `-` | `the auditor prefix is empty`, or `the auditor prefix starts with "<WORD>", which reads as an option` |
| an `--auditor` program that cannot be started: a path that is not an existing regular file with an execute bit, or a bare name found as no such file on `PATH` | `the auditor program "<WORD>"`, then why |

`--hermetic strict` asks for an audit the way `--audit` does. A bare program name is looked for
on the `PATH` `run` itself was started with, because that is the `PATH` the auditor inherits.
Every check but the last runs again at audit time, after the session, because the document can
change while the session runs. A program removed or changed during the session is still exit `2`
after it.

`--auditor` takes an **argv prefix**, so a single-word program name is a degenerate prefix and a
two-word subcommand is not a special case.

After `decision census:`, `--audit` prints what the machine would not admit, in one of three shapes
because they are three facts: `withheld: <tool> (<reason>); …`, `withheld: none declared`, or
`withheld: not stated by the harness`. The census cannot say it — a tool that was never admitted was
never refused, so `denied=0` reads the same on a run that got everything.

### The `b10x` arm

These reach the beyond10x loop and are **refused by name on `claude` and `codex`**: a vendor arm's
scope travels sealed in `Frame.subjects`, and a flag would be a second, unsealed copy of it.

| Flag | Meaning |
|---|---|
| `--hooks <FILE>` | The operator's own programs, consulted by the loop at each call — its only content-level refusal. Named, never discovered. |
| `--driver <PATH>` | A program the confined run must be able to **start**: staged, mounted read-only, added to the loop's allow-list. Not the same question as `--allow-program`, which says what a `run` may *name*. |
| `--allow-program <PROGRAM>` | A program the loop's `run` tool may start. Repeatable; empty publishes no `run`. |
| `--substrate <SOCKET>`, `--substrate-embedded` | Confinement for the loop: a substrate daemon, or its driver held in-process. Never both. |
| `--cgroup-root <DIR>` | A delegated cgroup subtree; without one no `run` entry is published. |
| `--toolchain <NAME>` | A build toolchain admitted read-only inside the confined workspace. |
| `--prices <FILE>` | A rate card, so the record states what the run cost; the loop has no vendor catalogue behind it. |
| `--write-scope <GLOB=SCOPE>` | `<glob>=allowed\|partial-only\|denied`, ordered, first match wins. The operation is refused, never the run. |
| `--scope-announce stated\|silent` | Whether the scope is also stated in the instruction; `silent` is an experiment control. |

`--context` was on this list until design amendment a35; it reaches every kind now, see
[above](#--context-files-the-run-is-handed).

What the launch emits, and how the seam reads the record back, is on
[the b10x adapter](../harnesses/b10x.md).

## `--cwd`: the declaration that trades two rows for real work

The child runs **in this tree**, and **can write to it**. So:

- **H7** ("the working directory is ours") and **H11** ("no memory file outside the copied tree")
  stop being impositions and are attested **unavailable**, naming this directory.
- **The vendor sandbox is widened to that tree** so the work can actually happen — on codex the
  scratch config carries `sandbox_mode = "workspace-write"`, which permits editing files in the
  cwd. The H7 attestation row **says so in words**: a reader sees that the run could write to the
  operator's tree without diffing a scratch config that no longer exists.
- `--hermetic strict` therefore **exits 1** on such a run: the floor reads both rows as gaps.
- Plain `--hermetic` reports it honestly.
- `--add-dir` stays denied either way, and no `writable_roots` widens the grant past this tree.

The embedder that wants a governed run over a real repository accepts exactly that trade. A
scratch-cwd run is unchanged: the child writes nothing, anywhere.

:::note Why the grant is part of the trade
Until 2026-08-23 it was not, and the trade bought nothing: a paid codex run with `--cwd` got a real
repository it could only read, and the vendor's own stream said *"the workspace is read-only, so the
planning-store patch was rejected."* Design amendment **a6.1**.
:::

## `--scratch-cwd`: the embedder's disposable copy

An embedder that made its own scratch copy of the workspace, runs the child in it, and rechecks
the copy's changes before anything reaches the repository is not pointing the run at the operator's
tree, and `--cwd <DIR> --scratch-cwd` says so. The run is otherwise a `--cwd` run — same directory,
same sandbox grant — and the attestation changes:

- **H7** is attested **`declared`**, a third state beside imposed and unavailable: the embedder
  declared the directory as its scratch copy, metaharness did not create it, `--add-dir` is never
  passed. The floor reads it `ok` only when the record's cwd is the planned one; the declaration
  never stands in for the record.
- **H11** is attested exactly as for a scratch cwd, with the ancestor walk run from the copy. On
  Claude Code it rests on `--setting-sources ""` and the fresh scratch config home, held to the
  memory canary of the record's version; the walk looks for every memory shape the pinned loader
  reads and names what it finds in the row, so a memory file above the copy is reported and
  `--hermetic strict` still passes (amendment a36). On Codex and b10x the walk looks for `CLAUDE.md`
  and `AGENTS.md` and decides the row: a file above the copy is a gap, and `--hermetic strict`
  exits 1 after the run. A memory file at the copy's root is the copy's own and is listed in the
  attestation's `ambient_inputs`.
- `--hermetic strict` **passes** on a declared copy with nothing above it, when the record's cwd
  is the copy. On b10x the record's cwd is the directory the pinned loop (0.15.1) reports, compared
  as a directory and not as a spelling; where the loop reports none (under `--substrate <socket>`)
  H7 is `unk` and strict exits 3.
- Without `--cwd` the flag is refused by name on every kind.

Design amendment **a27**. The declaration confines nothing; making the copy is the embedder's job.

## `metaharness capabilities`

| Flag | Meaning |
|---|---|
| `--render` | Print the neutral-operation → vendor-tool table instead of the whole descriptor. |

A rendering that only exists inside a run cannot be asserted on before one — which is why it is
published here.

## `metaharness audit` and `metaharness project`

| Flag | Verb | Meaning |
|---|---|---|
| `<EVENTS>...` | `project` | The event stream(s), positional: one, or two under `--html`. |
| `--from <FORMAT>` | `project` | Read every record given as a vendor record of this format through the adapter's production reader: `claude-code-stream-json`, `codex-rollout` or `harness-json`. Any other name is refused by name; a record that is not the declared format is refused with exit 2 (design amendment a28). |
| `--out <FILE>` | `project` | Write the `trace-ir/1` document here instead of to stdout. |
| `--html <FILE>` | `project` | Render one or two runs as a static two-column page at this path. |
| `--events <FILE>` | `audit` | The event stream that went with the transcript. |
| `--to <FORM>` | `project` | The target form. Default `trace-ir`. |
| `--transcript <FILE>` | `audit` | The raw vendor transcript to judge. |
| `--spec <FILE>` | `audit` | The expectation document. |
| `--auditor <PREFIX>` | `audit` | The external auditor, as an argv prefix. |

`audit` currently **refuses with exit 2, naming what it waits for**. `project` writes the document (design amendment a15); under `--from` the document names the vendor format as its source and its digest is over the record's bytes (a28).

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Completed; every gating hermetic row is `ok`. |
| `1` | A gating row is `gap`. |
| `2` | Refused — a verb or option that is not built. |
| `3` | A gating row is `unk`. Not a softer `gap`. |
