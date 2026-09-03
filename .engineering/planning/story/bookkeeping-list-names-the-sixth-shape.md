---
format: aep.planning-md/1
id: story:bookkeeping-list-names-the-sixth-shape
kind: story
status: implemented
title: The record count never decided anything, so the sixth bookkeeping shape is named too
relations:
- decomposes: epic:runs-side-by-side
revision: 4
---
# Story: the record count never decided anything, so the sixth shape is named too

## Outcome

A stream from Claude Code 2.1.259 carries no `opaque` line for the vendor's own bookkeeping, including
`system/background_tasks_changed`.

## Context

0.6.1 named five of 2.1.259's new shapes, on a run that carried 183 of them and left every `tool.absent`
row `unk` over *"183 events the adapter could not read"*. The re-recorded run of the same case
(2026-09-03, 1,608 events) carried **2** records of a sixth, `system/background_tasks_changed`, and that
was enough: `the-story-was-walked-through-its-lifecycle` came back `undecidable` on `opaque_events`.

A gate row is decided by whether the adapter could read the stream, not by how many records it could
not read. 183 and 2 are the same finding.

The record narrates a backgrounded `Bash` call starting or ending, which is already on the wire twice —
as that call's `tool.requested` and its `tool.result`. It carries no fact an expectation reads.

## Acceptance

- `system/background_tasks_changed` emits nothing and is not `opaque`.
- The list stays closed: a subtype it does not name still goes `opaque`, and
  `the_vendors_bookkeeping_records_are_recognised_and_emit_nothing` still asserts it.
- A re-recording of the golden-path case reports `opaque` = 0.

## Out of Scope

- Opening the list to a prefix or a wildcard. D4 is why each shape is named one at a time.
