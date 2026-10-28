# Session fixture: hand-written, not recorded

This directory is synthetic, unlike the recorded wire beside it (`../README.md`). It is shaped like a
Claude Code session as stored on disk, `<session>.jsonl` plus `<session>/subagents/agent-<id>.jsonl`,
and is read by `SessionReader` (design amendment a63, story:session-jsonl-adapter). Every value in it
is invented: ids, paths under `/fixture/operator`, prompts, commands and outputs. No real session
contributed content. Only the field names follow what the vendor writes.

| file | what | read by |
|---|---|---|
| `5e551011-0000-4000-8000-000000000001.jsonl` | the session: one record of every type the reader names, the opaque types it does not, a request split over two records and an `Agent` call | `session.rs` tests, `crates/metaharness-cli/tests/project_session.rs`, the conformance target |
| `5e551011-0000-4000-8000-000000000001/subagents/agent-a0f1c2d3e4b5a697.jsonl` | the sub-agent the `Agent` call spawned | the same |
| `5e551011-0000-4000-8000-000000000001.expected.jsonl` | **generated**: the event stream the reader owes for the two files | `the_committed_session_reads_to_its_committed_expected_stream` |

Regenerate the expected stream with `cargo test --workspace --lib regenerate -- --ignored` and review
its diff. A change to any value here moves the outcomes pinned in `spec/domains/trace.yaml`.
