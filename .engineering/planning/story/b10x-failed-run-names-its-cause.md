---
format: aep.planning-md/3
id: story:b10x-failed-run-names-its-cause
kind: story
status: draft
title: A failed b10x run names its cause in session.ended
summary: The b10x adapter carries finished.stop.cause, message and after_compaction into session.ended
relations:
- decomposes: epic:loom-agent-work-through-metaharness
revision: 1
---
## Context

From Harness 0.21.0 on, a b10x-harness run that started and then failed ends its event stream with
one `finished` line whose stop is
`{"kind":"failed","cause":"<cause>","message":"<one line>","after_compaction":<bool>}`
(harness `story:failed-run-ends-with-a-finished-line`). `cause` is `provider-<wire error code>`
(`provider-refused`, `provider-transport`, ...), `environment`, `budget` or `config`.

The b10x adapter's `"finished"` arm (`crates/metaharness-b10x/src/seam.rs`) reads only
`stop.kind`: such a run becomes `session.ended{is_error: true, subtype: "failed",
terminal_reason: "failed"}`, and `cause`, `message` and `after_compaction` are dropped. The Loom
session executor formats `subtype` only (`the session ended in an error (failed)`), so the cause
still does not reach the caller's record.

Seen in a one-hour trial on 2026-10-26: 7 of 41 harness steps ended with "the session ended
without the JSON line naming its action" (Harness 0.20.0 wrote no `finished` line on a failed run;
the three transcripts read ended after a `conversation-compacted` warning).

## Outcome

A b10x run that failed reaches `session.ended` with its cause: the adapter carries `stop.cause`,
`stop.message` and `stop.after_compaction` into the terminal record, so a caller formatting the
record can say why the run ended.

## Acceptance

1. A fixture line `{"kind":"finished","stop":{"kind":"failed","cause":"provider-refused","message":"m","after_compaction":true},"turns":3}`
   becomes one `session.ended` with `is_error: true`, `num_turns: 3`, and the cause, message and
   compaction flag in fields a caller reads (which fields is this story's design decision; a
   `subtype` such as `failed:provider-refused` is the smallest that reaches today's Loom message).
2. A `finished` line from an older Harness (no `cause`) maps exactly as today.

## Out of scope

- Harness's own line (shipped in Harness 0.21.0).
- The Loom executor's message format (Loom's store), beyond what `subtype` already carries.
