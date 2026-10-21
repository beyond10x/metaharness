---
format: aep.planning-md/3
id: story:session-request-carries-context-files
kind: story
status: implemented
title: A session request carries context files
refs:
- provider: atlas
  reference: adr-0098
relations:
- decomposes: epic:loom-agent-work-through-metaharness
- depends_on: story:claude-code-2-1-304-declared
- depends_on: story:codex-strict-record-rows
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: inferred
  path: crates/metaharness-aep/src/drive/ess_conformance.rs
- confidence: inferred
  path: crates/metaharness-aep/tests/adversary_trace_ir_domain.rs
- confidence: cited
  path: crates/metaharness-b10x/fixtures/c1
- confidence: inferred
  path: crates/metaharness-b10x/src/seam.rs
- confidence: cited
  path: crates/metaharness-b10x/src/vectors.rs
- confidence: cited
  path: crates/metaharness-claude/fixtures/c1
- confidence: cited
  path: crates/metaharness-claude/src/launch.rs
- confidence: inferred
  path: crates/metaharness-claude/src/transcript.rs
- confidence: cited
  path: crates/metaharness-claude/src/vectors.rs
- confidence: inferred
  path: crates/metaharness-claude/tests/adapter.rs
- confidence: inferred
  path: crates/metaharness-claude/tests/native_fixture.rs
- confidence: inferred
  path: crates/metaharness-claude/tests/support/mod.rs
- confidence: cited
  path: crates/metaharness-codex/fixtures/c1
- confidence: cited
  path: crates/metaharness-codex/src/launch.rs
- confidence: inferred
  path: crates/metaharness-codex/src/rollout.rs
- confidence: cited
  path: crates/metaharness-codex/src/vectors.rs
- confidence: inferred
  path: crates/metaharness-codex/tests/native_fixture.rs
- confidence: inferred
  path: crates/metaharness-protocol/src/event.rs
- confidence: inferred
  path: crates/metaharness-protocol/src/lib.rs
- confidence: cited
  path: crates/metaharness-protocol/src/spec.rs
- confidence: inferred
  path: crates/metaharness-protocol/tests/wire.rs
- confidence: inferred
  path: crates/metaharness/fixtures/golden/README.md
- confidence: cited
  path: crates/metaharness/fixtures/golden/contract-result-b10x.json
- confidence: cited
  path: crates/metaharness/fixtures/golden/contract-result-claude.json
- confidence: cited
  path: crates/metaharness/fixtures/golden/contract-result-codex.json
- confidence: cited
  path: crates/metaharness/src/builder.rs
- confidence: cited
  path: crates/metaharness/src/refusal.rs
- confidence: cited
  path: crates/metaharness/src/run.rs
- confidence: inferred
  path: crates/metaharness/src/scripted.rs
- confidence: inferred
  path: crates/metaharness/tests/audit.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: inferred
  path: spec/conformance.json
- confidence: inferred
  path: spec/coverage.md
- confidence: inferred
  path: spec/domains/workspace.yaml
- confidence: cited
  path: website/docs/harnesses/b10x.md
- confidence: inferred
  path: website/docs/harnesses/claude.md
- confidence: inferred
  path: website/docs/harnesses/codex.md
- confidence: inferred
  path: website/docs/protocol/events.md
- confidence: cited
  path: website/docs/reference/cli.md
- confidence: cited
  path: website/docs/status.md
revision: 48
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T06:57:33Z", actor: "human:timo", revision: 45}
- {from: "proposed", to: "active", at: "2026-10-21T06:57:33Z", actor: "human:timo", revision: 46}
- {from: "active", to: "implemented", at: "2026-10-21T09:47:37Z", actor: "human:timo", revision: 48, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"verification":1,"ess_conformance_coverage_v1":1}}}
---
# Story: a session request carries context files

## Scheduling

Unit 4 of wave 4, after units 2 and 3 (shared files), before release 0.11.0; approved 2026-10-21 by
`b10x-chef-orchestrator` under the standing exception (ADR 0099/0101 trial work) with the acceptance below.
Claude `@path` stays a later, measured option.

## Outcome

An embedder names the files a session should start with as a harness-neutral list on the session
request, with bounds metaharness enforces, and each adapter renders the list in its harness's own form,
so the embedder does not need to know any vendor's syntax and the model does not spend a read on files it
was handed.

## Context

- Relayed: takt's next candidate names a step's scope files as `@path` mentions, which Claude Code `-p`
  expands, so the model skips its Read; takt's probe measured 6,444 to 7,165 fewer tokens per `tests.write`
  step. Today takt has to know that this is Claude syntax.
- Rendering per adapter (relayed proposal): Claude as `@path` or inline; Codex as its own form if
  takt-trial's probe shows `codex exec` expands `@path`, otherwise inline; b10x inline.
- The credential-shape scan stays on takt's side before the request.

## Acceptance

`--context <FILE>` (`RunSpec::context`) is accepted on every kind. Before spawn metaharness refuses by name,
naming the path: a context file on a run without `--cwd`; a path that is absolute or contains `..`; a path
that is or passes through a symlink; a file that is not a regular file; a file not tracked by git in the
`--cwd` tree, under `--scratch-cwd` too; a list past the count cap or the byte cap (caps named in the design
amendment; the byte cap leaves room for the prompt below Linux's 128 KiB argv limit). Files are opened beneath
the tree without following links (`openat2` with `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS`), and the bytes
digested are the bytes rendered. Claude and Codex inline each file into the prompt inside a delimiter the
content cannot close; b10x passes each file to the loop's `--context`. `session.started.context_files` lists
each file's path, sha256 and byte count and is absent when none was declared, so no committed stream, fixture
or trace document changes bytes. One C1 vector per kind pins the rendering (`checked` +1 in each
`contract-result-*.json`); the workspace domain gains a `DeclareContextFiles` command with an outcome per
refusal and the count gate moves; `task check` exits 0; `CHANGELOG.md` names `--context` widened to every kind.

## Out of scope

- Scanning file contents for credentials (takt's side).

## Scope

- To be derived by `aep:story-scoper`. Likely: `crates/metaharness-protocol/src/spec.rs` (`RunSpec`),
  `crates/metaharness/src/builder.rs` (`check_spec`), the three adapters' launch planning, the opening
  record, vectors, `docs/design/metaharness-protocol-v0.1.md`, `website/docs/reference/cli.md`,
  `CHANGELOG.md` (all inferred).

## Measured: `codex exec` 0.153.4 does not expand `@path` (relayed 2026-10-20)

`b10x-chef-orchestrator` read takt-trial's probe log (`21-probe-codex-mention.log` in takt's trial state):
each call planted a random marker line atop `todo.py` and asked the model to quote it. Without a mention:
input 19,164 tokens, reply NONE. With `@todo.py @tests/test_todo.py` (2 calls): input 18,339 each, reply
NONE. With paths outside the clone: input 18,366, reply NONE. No mention grew the input and the marker
never reached the model. So the Codex adapter inlines the file content into the prompt; Claude keeps
`@path`; b10x inlines.
## Ambiguities

- Q1 answered by `b10x-chef-orchestrator` (2026-10-21, from `git ls-files` on a takt scratch copy): takt's
  `--scratch-cwd` copy is a git work tree with its own toplevel and a `.git` gitfile pointing at a sibling
  git directory; the fixture files (12, among them `todo.py`, `tests/test_todo.py`, `AGENTS.md`, `CLAUDE.md`)
  are tracked; the only untracked files are takt's records under `.engineering/cases`. So "tracked by git"
  holds under `--scratch-cwd` as well, and the tracked check must follow a gitfile.
- Open for the implementor and the amendment: how "tracked" is checked (an external `git ls-files -z
  --error-unmatch` with a constructed environment, or reading the index with gitfile indirection), the cap
  values, the record placement's exception to the absent-means-null rule, the inline delimiter.
## Delivered (wave 4 of 2026-10-21, unit 4)

Commit `0d218e4`, merged `d0a0b85`; two `aep:adversary` passes
(`review-result:adr0096-w4-context-files-adversary-1`, `-2`), one correction round and a final correction,
verified by the coordinator. `--context <FILE>` on every kind, refused by name before spawn outside the
stated bounds (tracked by git in the `--cwd` tree, submodules included; at most 16 files and 65,536 bytes);
files opened beneath the tree without following links. Claude carries the block in
`--append-system-prompt`, because 2.1.304 and 2.1.305 expand `@` mentions in prompt text and read the file
with no record; `b10x-chef-orchestrator` accepted that with three conditions, all met: the task text's own
exposure is `story:claude-task-text-mentions`, the record names each file's channel, and a per-version
mention canary sits beside the H11 canary. Codex inlines into its prompt; b10x hands the loop a copy and
records the channel as `system-prompt`. `session.started.context_files` (amendment a35); conformance
90 → 103. On b10x `--context` is narrowed (needs `--cwd`, refuses what the previous release passed through).
