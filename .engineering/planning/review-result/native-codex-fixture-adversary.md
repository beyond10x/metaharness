---
format: aep.planning-md/3
id: review-result:native-codex-fixture-adversary
kind: review-result
status: active
title: Native fixture subprocess bounds review
relations:
- reviews: story:current-adapter-compatibility
revision: 1
---
unit: native Codex fixture over 943d6fb6, managed unit wt-a72e7996f153
verdict: NEEDS-CHANGE
cases: coordinator supplied four passing native tests; reviewer executed zero
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: address subprocess cleanup and version probe bounds

Independent read-only review by agent plan_parallel found two static boundary defects: answer_hooks or try_wait could panic after spawning without reaping the child, and the direct vendor --version Command::output call had no deadline or environment isolation. These were not reproduced failures. Native assertions and their documented evidence limits were otherwise useful. Vendor exits0/1 and CLI verdict exits0/3 were correctly distinguished. The reviewer did not run native tests, spend, edit or verify driver build provenance.

```findings
- file: crates/metaharness-codex/tests/native_fixture.rs
  line: 254
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Hook-service or polling failures can unwind past the live child without killing or reaping its process group; add a cleanup guard.
- file: crates/metaharness-codex/tests/native_fixture.rs
  line: 489
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The vendor banner subprocess is unbounded and inherits the environment.
```
