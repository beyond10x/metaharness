---
format: aep.planning-md/3
id: review-result:adr0096-w8u7-review
kind: review-result
status: active
title: 'Wave 8 unit 7 review: aep-eval native arm removed'
relations:
- reviews: story:aep-eval-native-arm-removed
revision: 1
---
# Review: wave 8 unit 7, the aep-eval native arm removed

One `aep:adversary` pass over revision 1 (base `8edb547`), 2026-10-22, cases in
`crates/metaharness-aep-eval/tests/adversary_w8u7.rs`. 14 cases, 5 red; 10 mutants built, m9 survived.

Findings and what the coordinator decided:

- Warning (introduced): the removed-verb guard saw a quoted literal only inside a `Command::new` span whose
  line named a harness marker. It missed a constant, a verb added by the builder's caller, a harness path
  bound to a local, an `sh -c` string, and the doctor spawn at `crates/metaharness/src/doctor.rs:211-214`.
  D47: a string-literal scan of every source naming the harness, with a (file, item, reason) allowlist, plus
  argv assertions on the b10x adapter, doctor and the confined probe.
- Note (introduced): mutant m9, the `status == open` check removed, survived. D48: a case with the blocker at
  `accepted`, `closed` and `Open`.
- Note (introduced): `Proxy-Authorization`, `Cookie` and `x-goog-api-key` were not detected. D49: detected,
  case-insensitive, a case each.
- Note (pre-existing, not fixed): the harness spawn in the preflight has no wall-clock bound
  (`crates/metaharness-aep-eval/src/lib.rs:1231`); endpoint IO is bounded at 30 s.
- Note (pre-existing, not fixed): the harness JSONL record of the preflight is written and not read, so
  confinement is shown by argv[0] and the store effect only (invariant 3).
- Note (origin unverified): with the scratch root inside a git repository the driver's fence would leave the
  writable subtree and the preflight would fail closed.
- Note: `story:call-seam-for-harness` still named the native arm; its body was updated in the integration
  branch.

Checked and not faulted: the endpoint binds 127.0.0.1 only; malformed, oversized and non-POST requests fail
the verdict; a third request fails; the store sits inside the writable subtree; `REGROUPED_VERBS` is AEP's own
CLI; the documents agree with the code.

Revision 2 by `aep:implementor`: all red cases green, m9 killed. Gate `w8u7r` exit 0 on every step, 1293
passed, 0 failed, 50 ignored; ESS conformance 116/116.

```findings
[
{"file":"crates/metaharness-aep-eval/src/lib.rs","line":null,"category":"test-guard-scope","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"the removed-verb scan missed consts, caller-added verbs, local bindings, sh -c strings and the doctor spawn; revision 2 scans every literal in harness-naming sources with an allowlist and asserts the argv sets"},
{"file":"crates/metaharness-aep-eval/src/lib.rs","line":null,"category":"untested-branch","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the status == open check had no test; revision 2 adds one"},
{"file":"crates/metaharness-aep-eval/src/lib.rs","line":null,"category":"credential-detection","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"Proxy-Authorization, Cookie and x-goog-api-key were not detected; revision 2 detects them"},
{"file":"crates/metaharness-aep-eval/src/lib.rs","line":null,"category":"bounded-wait","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"the preflight's harness spawn has no wall-clock bound; not fixed"},
{"file":"crates/metaharness-aep-eval/src/lib.rs","line":null,"category":"invariant-3","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"the preflight's harness JSONL record is not read; not fixed"},
{"file":".engineering/planning/story/call-seam-for-harness.md","line":null,"category":"stale-doc","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"named the removed native arm; body updated"}
]
```
