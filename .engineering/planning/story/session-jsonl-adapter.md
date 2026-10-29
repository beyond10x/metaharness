---
format: aep.planning-md/3
id: story:session-jsonl-adapter
kind: story
status: implemented
title: metaharness project normalizes a Claude Code session as stored on disk
summary: A reader in metaharness-claude maps an on-disk Claude Code session and its sub-agent files into trace-ir/1, total by construction; first consumer is the decider corpus.
relations:
- decomposes: epic:session-mining
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-28T10:35:40Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-28T10:35:40Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-29T08:23:29Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A Claude Code session as Claude Code stores it on disk (`~/.claude/projects/<project>/<session>.jsonl`,
with its sub-agent transcripts under `<session>/subagents/`) is normalized into a `trace-ir/1`
document by `metaharness project --from claude-code-session <file>`, through a reader in
`metaharness-claude` that is total by construction like `TranscriptReader`: every record becomes at
least one protocol event, an unmapped record becomes `Event::Opaque`, unknown fields are ignored,
no clock is read.

This is the "session-JSONL adapter" part of `epic:session-mining` (Atlas ADR 0088). Its first
consumer is the decider corpus (operator, 2026-10-28: normalize Claude and Codex sessions first,
then remove sensitive data, then derive training data; the corpus will later grow to far more
sessions of both harnesses).

## Context

- `metaharness project --from` accepts `claude-code-stream-json`, `codex-rollout` and
  `harness-json` (`crates/metaharness-cli/src/lib.rs:144`, `story:project-reads-vendor-transcripts`,
  which names the session JSONL as a different format and out of its scope).
- `crates/metaharness-claude/src/transcript.rs` reads `stream-json` only: it has no handling of the
  session envelope (`parentUuid`, `isSidechain`, `sessionId`, `uuid`, `summary` and `system`
  records, `toolUseResult`), 0 matches on 2026-10-28.
- The session file and `stream-json` share the inner `message` shape (`user` and `assistant`
  records whose `content` holds `text`, `tool_use`, `tool_result`, `thinking`); the envelope
  differs.
- `spec/domains/trace.yaml` declares the source kinds of `trace-ir/1` (`story:trace-ir-domain`);
  a new source kind is declared there first and the conformance suite regenerated.

## Scope

- `crates/metaharness-claude/src/session.rs` (new): the session reader, built over the shared
  message mapping rather than a copy of it.
- `crates/metaharness-claude/fixtures/golden/session/`: a synthetic session and one synthetic
  sub-agent file, written by hand for this story, with their expected event stream. No recorded
  transcript is committed: no real prompt, path, name or tool output.
- `crates/metaharness-cli/src/lib.rs` and the library's source-kind parser: `claude-code-session`.
- `spec/domains/trace.yaml`, the generated conformance suite, the CLI reference.

## Acceptance

`metaharness project --from claude-code-session crates/metaharness-claude/fixtures/golden/session/<session>.jsonl`
exits 0 and writes a `trace-ir/1` document whose node census equals the census of the committed
expected stream, with the sub-agent's events linked to the parent's spawning tool call, where today
the verb refuses `claude-code-session` as an unknown format.

## Done when

- The acceptance command holds in a test.
- Every record type of the fixture maps to a named event or to `Opaque`; a test over a record of an
  unknown `type` yields `Opaque` with its digest and line.
- The reader runs over a real local session without a panic. That check runs on the implementor's
  machine and only its counts are reported; no real record enters the repository.
- `task check` exits 0.

## Out of scope

- Redaction before ingestion (a later story under `epic:session-mining`).
- Episodes, semantic actions, proposals.
