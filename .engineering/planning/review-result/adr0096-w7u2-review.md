---
format: aep.planning-md/3
id: review-result:adr0096-w7u2-review
kind: review-result
status: active
title: 'Wave 7 unit 2 adversary review: the pairing checked before the spawn'
relations:
- reviews: story:spec-auditor-pairing-checked-before-spawn
revision: 1
---
# Review: wave 7 unit 2, the expectation-document and auditor pairing before the spawn

`aep:adversary` pass 1 over the uncommitted unit 2 change (base `00cd0f8`), 2026-10-21. Three warnings,
all present before the unit, and one note on the unit's own tests. Two red test files
(`crates/metaharness/tests/adversary_w7u2.rs`, `crates/metaharness-cli/tests/adversary_w7u2.rs`), kept in
the change.

Revision 1 by `aep:implementor` resolved every finding as the coordinator decided: a document path that is
not UTF-8 is refused as `SpecUnreadable` naming its bytes; the auditor's first word is resolved before the
spawn (a word with `/` must name an executable regular file, a bare word must be found on `PATH`, a word
starting with `-` is refused) as `AuditorNotInvokable`; auditor arguments without `--auditor` are refused as
the new `AuditorArgsWithoutAuditor`; one test fixes which refusal wins when two rows of a43 apply. The gap
sentence in `website/docs/reference/cli.md` was removed. No adversary case was changed or deleted.

After revision 1: gate 6 of 6 steps exit 0, 1164 passed, 0 failed, conformance 103/103; 23 planted mutants
of `auditor.rs` and `builder.rs`, all caught.

```findings
[
{"file":"crates/metaharness/src/auditor.rs","line":147,"category":"refusal-gap","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"a non-UTF-8 --spec path passes check_document but the auditor receives the lossy display() path, which does not exist, after the harness ran; refused before the spawn as SpecUnreadable in revision 1"},
{"file":"crates/metaharness/src/auditor.rs","line":193,"category":"refusal-gap","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"check_pairing does not resolve the auditor program (missing path, not on PATH, quoted word, flag as first word), so the harness runs and the audit then exits 2; resolved before the spawn as AuditorNotInvokable in revision 1"},
{"file":"crates/metaharness/src/auditor.rs","line":195,"category":"refusal-gap","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"arguments after -- with no --auditor are accepted and dropped, so a --spec named there is checked by nobody; refused before the spawn in revision 1"},
{"file":"crates/metaharness/src/auditor.rs","line":200,"category":"test-gap","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"mutant M15 (document check ahead of the wants-audit test) is caught by no test; a refusal-order test added in revision 1"}
]
```
