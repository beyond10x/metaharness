---
format: aep.planning-md/3
id: story:mcp-arguments-checked-against-declared-types
kind: story
status: active
title: MCP tool arguments are checked against the declared types
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/metaharness-tools/src/caller.rs
- confidence: cited
  path: crates/metaharness-tools/src/server.rs
- confidence: cited
  path: crates/metaharness-tools/tests/caller_catalogue.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-22T00:51:00Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-22T00:51:00Z", actor: "human:timo", revision: 5}
---
# Story: MCP tool arguments are checked against the declared types

## Scheduling

Draft for wave 8, from the operator's user trial of release 0.12.0 on 2026-10-21 (finding 04; the
evidence is local to the operator and not in this repository).

## Outcome

`metaharness mcp-serve` refuses a `tools/call` whose argument does not have the type the tool's
`inputSchema` declares, with a tool result marked `isError` naming the argument and the declared type (D1).

## Context

- `fs_read.offset` is declared `integer`. Observed on 0.12.0: `"second"`, `1.5` and `false` were accepted
  and read from line 1; `2` read from line 2.
- The design defers field-type validation; missing and extra arguments are already refused.

## Acceptance

- expected_before: `fs_read` with `offset` `"second"`, `1.5` or `false` returns the file from line 1.
  After: each returns a tool result marked `isError` naming `offset` and `integer`; `offset: 2` is unchanged.
- The same check covers every declared argument type of every owned tool, by one test over the declared
  catalogue.

## Scope

Derived 2026-10-22 by `story-scoper` at `a34551e` (release 0.13.0); decisions D1–D4 by the coordinator.

- Primary surface: `crates/metaharness-tools`, the caller-catalogue road (`metaharness mcp-serve --catalogue`).
- `crates/metaharness-tools/src/caller.rs:206-256`: `CatalogueEntry::argument_problems` gains a type check
  over the published schema (`schema()` at `:610-641`, `json_kind` at `:259`).
- `crates/metaharness-tools/src/server.rs:51-60` (doc comment); the call site `:168-180` is unchanged and the
  refusal is the existing `isError` result (`failed()` at `:261`).
- Tests: `crates/metaharness-tools/tests/caller_catalogue.rs`, beside the sibling refusal test at `:559`;
  one test walks every declared argument of every catalogue entry.
- Design: amendment a45 to `docs/design/metaharness-protocol-v0.1.md`, replacing the a40 § 5 deferral at
  `:3507` ("A field's value is not checked against its type here").
- `CHANGELOG.md` under `[Unreleased]`.

Decisions:

- D1 a mistyped argument is answered as the missing/extra-argument refusal is: a tool result with
  `isError: true` naming the argument and the declared type (`server.rs:174-179`, design `:3500`). The
  Outcome's "JSON-RPC error" means this result.
- D2 `null` on a non-required field reads as absent and is accepted; on a required field it is refused.
- D3 `integer` is a JSON number serde_json holds as i64 or u64: `1.0`, `1.5`, `"2"` and `false` are refused;
  range checks are not this story.
- D4 the three-verb road (`tool_invoke.arguments`, typed only `object`) is not this story.

Safety: the check only adds refusals before `call_verb` (`server.rs:172-185`); a call that passes is
performed with the same arguments.
