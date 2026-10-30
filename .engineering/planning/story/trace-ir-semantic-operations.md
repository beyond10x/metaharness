---
format: aep.planning-md/3
id: story:trace-ir-semantic-operations
kind: story
status: implemented
title: Every tool call in a trace-ir/1 document carries its semantic operations
summary: 'Copy context-lens (MIT) semantic extraction: one node per shell command with executable, kind and structure, plus wrapper unwraps'
owner: metaharness
tags:
- spec
- trace
relations:
- decomposes: epic:session-format
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T02:12:04Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-30T02:12:05Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-30T02:40:24Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"ess_conformance_coverage_v1":1}}}
---
# Story: every tool call in a trace-ir/1 document carries its semantic operations

## Scheduling

Filed 2026-10-30 on the operator's instruction of that day: "Decider must use meta harness, and meta
harness must copy what is done in context lens". Unit 1 of `docs/design/context-lens-parity.md`, the
unit decider waits on. One wave, one integration branch (`wave/context-lens-semantic`).

## Outcome

A consumer of a `trace-ir/1` document reads what each tool call does from the document itself: for a
shell call, one node per command the shell runs, with its executable, its semantic kind
(`file.read`, `stream.select`, `vcs.status`, `validation.test`, ...), the sequence, pipeline, loop
and conditional structure around it, and the `bash -c` unwrap. Today `metaharness project` parses no
shell command at all, so decider would have to re-implement that parse outside metaharness.

## Context

- context-lens (MIT) `src/semantic/extract.rs` and `src/semantic/mod.rs` at `d72b3b4` are the
  behaviour to copy: tree-sitter 0.25 over Bash and JavaScript, conservative, never executing.
- `metaharness-protocol` may not depend on tree-sitter (AGENTS.md invariant 1), so the extractor is
  a new crate, `metaharness-semantic`, and the production writers (`metaharness::project_file`,
  `metaharness::project_vendor_record`) annotate the document after projecting it.
- Added on top of the source: `env`, `timeout`, `xargs`, `sudo`, `nice`, `nohup` and `time` unwrap
  to the program they run; a script run as `./x.sh` or `python3 x.py` names the script's basename.
- Design: amendment a68 of `docs/design/metaharness-protocol-v0.1.md`. Additive under decision D3,
  so the format stays `trace-ir/1`.

## Acceptance

The conformance scenarios `metaharness.trace.ExtractOperations/outcome/<name>` pass in `task check`
for every outcome the domain declares: `native-read`, `pipeline-read`, `sequence`, `shell-unwrap`,
`env-unwrap`, `timeout-unwrap`, `xargs-unwrap`, `sudo-unwrap`, `nice-unwrap`, `nohup-unwrap`,
`time-unwrap`, `nested-wrappers`, `script-path`, `interpreter-script`, `codex-javascript`,
`malformed-shell` and `pattern-withheld`, each read off a document `metaharness::project_file`
wrote, where today no tool-call node carries `semantic_operations`.

## Done when

1. `spec/domains/trace.yaml` declares `metaharness.trace.SemanticOperation` and the command
   `metaharness.trace.ExtractOperations`; `ess specify validate --path spec --strict-requires` exits 0.
2. The 23 tests of the source's `extract.rs` are ported and pass, beside new tests for the unwraps
   and the script name that failed before the code existed.
3. The committed `evals/aep/runs/*.trace-ir.json` goldens are regenerated and the diff is only the
   added fields.
4. No argument text is stored beyond what the source stores: patterns and argv are hashed.

## Out of scope

- Conversation classification and outcomes, fragments, the store, report and dashboard
  (later units of the parity map).
- Per-record working directory tracking (the source reads `cwd` per record; this unit uses the
  session's opening `cwd`).

## Scope

- `crates/metaharness-semantic/` (new), `crates/metaharness/src/viewer.rs`,
  `crates/metaharness-protocol/src/trace_ir.rs` (one optional field in the metaharness block),
  `spec/domains/trace.yaml`, `spec/conformance.json` (regenerated),
  `crates/metaharness-aep/src/drive/ess_conformance.rs`, `evals/aep/runs/*.trace-ir.json`,
  `docs/design/metaharness-protocol-v0.1.md`, `docs/design/context-lens-parity.md`, `CHANGELOG.md`.
