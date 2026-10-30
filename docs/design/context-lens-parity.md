# context-lens parity map

Operator decision, 2026-10-30: metaharness copies what context-lens (MIT) does, so that a consumer
such as decider reads one `trace-ir/1` document instead of a second analysis tool. The source is
context-lens (MIT) at commit `d72b3b4`; its licence travels with the copied code in
`crates/metaharness-semantic/LICENSE-context-lens`.

One row per context-lens capability. "Has" cites the metaharness counterpart as of this map;
"missing" is what the source does that metaharness does not; "unit" is the unit that copies it.
Unit 1 is delivered (amendment a68 of `metaharness-protocol-v0.1.md`, story
`trace-ir-semantic-operations`). Units 2 to 5 are listed, not planned: each becomes its own story
under `epic:session-format` when it is taken up.

| context-lens (MIT) | what it does | metaharness has | missing | unit |
|---|---|---|---|---|
| `src/semantic/extract.rs` | Parses a tool call's input with tree-sitter (bash, and JavaScript for a Codex `exec` script) into operation nodes: one per shell command, with semantic kind, method, evidence, resolution, sequence/pipeline/loop/conditional containers, the `bash -c` unwrap, path params relative to `cwd`, an argument hash and a request signature. | `crates/metaharness-semantic/src/extract.rs:1329` `extract`, the source's code with its 23 tests; plus `executable` (basename of `argv[0]`), `script` (basename of a script run as `./x.sh` or `python3 x.py`) and `wrappers` (the unwrapped `env`, `timeout`, `xargs`, `sudo`, `nice`, `nohup`, `time`; `unwrap` at `:1674`, `script_of` at `:1716`). Declared as `metaharness.trace.ExtractOperations` in `spec/domains/trace.yaml:762`. | Nothing for a shell or `exec` input. The b10x adapter's own tool names (`file_read`, `run`, ...) are not in the source's native-tool table and classify as `other`. | 1 (done); the b10x tool table is open under 1 |
| `src/semantic/mod.rs` | Runs the extractor over every imported call into a SQLite sidecar bound to source fingerprints; recovers a call's input from its raw record; groups calls into activity episodes; reports operation, activity, pattern and repeated-signature counts. | `crates/metaharness-semantic/src/lib.rs:33` `annotate`: every `tool_call` node of a projected document carries `semantic_operations`, and `metaharness.semantic_rule` names the rule (`crates/metaharness-protocol/src/trace_ir.rs:188`). Called by `crates/metaharness/src/viewer.rs:138` (`project_file`) and `:342` (`project_vendor_record`). The working directory is the session's opening `cwd`. | Per-call `cwd` from a Claude record (Codex `workdir` is read); the sidecar store; episodes; the operation/activity/pattern/repeat report. | store and episodes: 4; per-record `cwd`: 5 |
| `src/claude.rs` | Normalizes a Claude Code session: messages, tool calls and results, usage per request, thinking and redacted thinking, server tool use and web fetch results, attachments as instructions (prompt snapshot, skill listing, deferred tools, MCP instructions, output style, queued command), `compact_boundary` compaction, API errors, summaries, media; each block assigned a context category, with baseline slots replaced rather than appended. | `crates/metaharness-claude/src/session.rs:127` `SessionReader` (session envelope, subagent files, request dedupe, user versus synthetic origin at `:599`) and `crates/metaharness-claude/src/transcript.rs:49` `TranscriptReader`. Records other than `assistant` and `user` become opaque nodes with their digest. | Attachment records as instruction nodes; compaction and API-error nodes; context categories and baseline slots; per-record `cwd`. | 5 |
| `src/codex.rs` | Normalizes a Codex rollout: response items as canonical, echoed event messages deduplicated, `turn_context` per turn, `compaction`/`compacted`, `turn_aborted` as interruption, errors, `local_shell_call` and `web_search_call`, legacy and record-level token usage. | `crates/metaharness-codex/src/rollout.rs:87` `RolloutReader`: `session_meta`, `response_item` (`function_call`, `custom_tool_call`, their outputs, `message`, `reasoning`), `event_msg` (`task_started`, `task_complete`, `token_count`, `agent_message`), `turn_context` at opening. Other records become opaque nodes. | `compacted`/`compaction` and `turn_aborted` nodes; `local_shell_call` and `web_search_call`; per-turn `cwd` after the opening; echo deduplication across `event_msg` and `response_item`. | 5 |
| `src/conversation/classify.rs` | Labels each user turn with transparent rules (correction, steering, approval, question reply, outcome assertion, native harness origin), an evidence rule id per label, a confidence and an explicit outcome signal; never stores excerpts. | Origin only: `user` versus `synthetic` on the injection node (`crates/metaharness-claude/src/session.rs:599`, `crates/metaharness-claude/src/transcript.rs:559`). | The labels, rule ids, confidence and outcome signal on user-turn nodes. | 2 |
| `src/conversation/outcomes.rs` | Summarizes a session's outcome signals: corrections, steering, approvals, completions, validation passes and failures (a validation counts when one validation operation ran), interruptions and errors, each with its last position. | `crates/metaharness-protocol/src/trace_summary.rs:144` `summarize`: tool calls answered and errored, models and usage. | The outcome signals, which need unit 2's labels and unit 1's `validation.*` kinds. | 2 |
| `src/fragments.rs` | Fingerprints whitespace-normalized paragraphs and 128-word windows of context text, sampled evenly up to 128 each, for duplicate-text analysis. | Nothing. | The fragment fingerprints per text-bearing node. | 3 |
| `src/store.rs` | Imports sessions incrementally into SQLite (file stamps, schema and parser versions, compacted metadata) and runs read-only queries. | Nothing persistent: each projection reads the source file again (`crates/metaharness/src/viewer.rs:115`). | A store of projected documents keyed by source digest and rule versions. | 4 |
| `src/report.rs` | Reports per provider and session: tool families (`tool_family`), explicit timings, token usage, context categories and duplication; renders text. | `crates/metaharness-protocol/src/trace_summary.rs:144` `summarize` and `:124` `render`: per document, not across sessions. | Tool families, timings, cross-session aggregation, the duplication report. | 4 |
| `src/dashboard.rs` | Builds timestamp-bounded dashboard data (activity, operations, fragments) for an HTML page. | Nothing. | The bounded aggregation and its page. | 4 |

## Units

1. **Semantic operations** (done): `crates/metaharness-semantic`, amendment a68.
2. **Conversation classification and outcomes**: `classify.rs` and `outcomes.rs` as labels on
   user-turn nodes and outcome fields in the summary.
3. **Fragments**: `fragments.rs` as fingerprints on text-bearing nodes.
4. **Store, report and dashboard**: `store.rs`, `report.rs`, `dashboard.rs` and the sidecar,
   episode and report half of `semantic/mod.rs`.
5. **Adapter record parity**: the `claude.rs` and `codex.rs` rows' missing records, and per-record
   `cwd` for unit 1.

The source's `main.rs`, `dev.rs`, `html.rs` and `model.rs` are its command line, development
helpers, page shell and in-memory model; each unit takes what it needs of them.
