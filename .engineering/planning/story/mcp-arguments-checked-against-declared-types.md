---
format: aep.planning-md/3
id: story:mcp-arguments-checked-against-declared-types
kind: story
status: draft
title: MCP tool arguments are checked against the declared types
revision: 1
---
# Story: MCP tool arguments are checked against the declared types

## Scheduling

Draft for wave 8, from the operator's user trial of release 0.12.0 on 2026-10-21 (finding 04; the
evidence is local to the operator and not in this repository).

## Outcome

`metaharness mcp-serve` refuses a `tools/call` whose argument does not have the type the tool's
`inputSchema` declares, with a JSON-RPC error naming the argument and the declared type.

## Context

- `fs_read.offset` is declared `integer`. Observed on 0.12.0: `"second"`, `1.5` and `false` were accepted
  and read from line 1; `2` read from line 2.
- The design defers field-type validation; missing and extra arguments are already refused.

## Acceptance

- expected_before: `fs_read` with `offset` `"second"`, `1.5` or `false` returns the file from line 1.
  After: each returns a JSON-RPC error naming `offset` and `integer`; `offset: 2` is unchanged.
- The same check covers every declared argument type of every owned tool, by one test over the declared
  catalogue.

## Scope

The MCP server's argument decoding in `crates/metaharness-tools` (to be confirmed by the story scoper),
its tests, the design document's deferral note, `CHANGELOG.md`.
