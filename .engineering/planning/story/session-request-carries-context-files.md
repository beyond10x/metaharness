---
format: aep.planning-md/3
id: story:session-request-carries-context-files
kind: story
status: draft
title: A session request carries context files
refs:
- provider: atlas
  reference: adr-0098
relations:
- decomposes: epic:loom-agent-work-through-metaharness
revision: 1
---
# Story: a session request carries context files

## Scheduling

Draft, filed 2026-10-20 on Timo's idea (22:3x) relayed by `b10x-chef-orchestrator`. To be scoped; if it
is one unit it becomes unit 4 of wave 4 before release 0.11.0, otherwise first in wave 5. Its depends_on
and acceptance go to `b10x-chef-orchestrator` before implementation.

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

The session request (the `run` arguments and the library's `RunSpec`) accepts a list of context files;
metaharness refuses, before spawn and by name, a path that is absolute, escapes the working tree, is not a
regular file tracked in the tree, is or passes through a symlink, or exceeds a count cap or a byte cap
(caps named in the design amendment); each kind renders the accepted list natively (Claude `@path` or
inline; Codex as measured; b10x inline) and the opening record names the files and their digests; a vector
per kind pins the rendering; `task check` exits 0; the changelog names the field.

## Out of scope

- Scanning file contents for credentials (takt's side).

## Scope

- To be derived by `aep:story-scoper`. Likely: `crates/metaharness-protocol/src/spec.rs` (`RunSpec`),
  `crates/metaharness/src/builder.rs` (`check_spec`), the three adapters' launch planning, the opening
  record, vectors, `docs/design/metaharness-protocol-v0.1.md`, `website/docs/reference/cli.md`,
  `CHANGELOG.md` (all inferred).
