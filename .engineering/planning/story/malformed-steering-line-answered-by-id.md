---
format: aep.planning-md/3
id: story:malformed-steering-line-answered-by-id
kind: story
status: active
title: A steering line that does not parse is answered with a MALFORMED command result
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/metaharness-cli/src/lib.rs
- confidence: cited
  path: crates/metaharness-protocol/src/command.rs
- confidence: cited
  path: crates/metaharness/src/run.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: cited
  path: website/docs/protocol/commands.md
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T20:32:26Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-21T20:32:26Z", actor: "human:timo", revision: 3}
---
# Story: a steering line that does not parse is answered with a MALFORMED command result

## Scheduling

Wave 7 unit 3 (`approval-record:adr-0096-wave-7`), from the operator's user trial of release 0.12.0 on
2026-10-21 (finding 02; the evidence is local to the operator and not in this repository).

## Outcome

Every steering line `metaharness run` reads on stdin gets exactly one `command.result` on stdout. A line
that does not parse as a `metaharness.command/1` command gets `result: refused` with code `MALFORMED`,
under the `id` the line carried, or under `-` when no `id` can be read. The protocol page's example
parses.

## Context

- `steer` in `crates/metaharness-cli/src/lib.rs:470-487`: its doc says the refusal "goes out as an event
  under the id the caller used, or under `-`"; the code writes only a stderr line.
- `RefusalCode::Malformed` exists (`crates/metaharness-protocol/src/command.rs:86`, wire `MALFORMED`).
- `website/docs/protocol/commands.md:12` shows `"decision":"allow"`; the decoder takes
  `"decision":{"decision":"allow"}` (`Decision` is internally tagged, `command.rs`). Observed on 0.12.0:
  the flat form produced `malformed tool.decide: invalid type: string "allow", expected internally tagged
  enum Decision` on stderr and no `command.result`, and a controller waiting on the id timed out after 45 s.

## Acceptance

- expected_before: a `tool.decide` line with `"decision":"allow"` and `"id":"c1"` produces no event naming
  `c1`. After: one `command.result` event for `c1` with `refused.code` `MALFORMED` and the parse error as
  its reason; a line with no readable id is answered under `-`; a blank line still produces nothing.
- The library path an embedder uses refuses the same way, in the same words (design D11).
- `website/docs/protocol/commands.md` shows the nested decision form for allow and deny, and a test parses
  every JSON example on that page with the production decoder.
- Amendment a44 of `docs/design/metaharness-protocol-v0.1.md` if the wire changes; `CHANGELOG.md`
  `[Unreleased]` one line under `### Fixed`.

## Scope

`crates/metaharness-cli/src/lib.rs`, `crates/metaharness-protocol/src/command.rs` and the run's
`send_as` path in `crates/metaharness`, their tests, `website/docs/protocol/commands.md`,
`docs/design/metaharness-protocol-v0.1.md`, `CHANGELOG.md`.
