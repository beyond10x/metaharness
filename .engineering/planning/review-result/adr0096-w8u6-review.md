---
format: aep.planning-md/3
id: review-result:adr0096-w8u6-review
kind: review-result
status: active
title: 'Wave 8 unit 6 review: API key by secret-store name'
relations:
- reviews: story:b10x-api-key-from-keychain
revision: 1
---
# Review: wave 8 unit 6, the loop's API key named by its secret-store name

Two independent passes over the uncommitted unit 6 change (base `8edb547`), 2026-10-22, run in parallel:
`aep:adversary` (cases in `crates/metaharness/tests/adversary_w8u6.rs`) and an invariant review against the
safety envelope (cases in `crates/metaharness/tests/invariants_w8u6.rs` and
`crates/metaharness-cli/tests/invariants_w8u6.rs`). Nine cases, two red.

Before review, the implementor's first gate was red on five tests from the move to harness 0.18.0. D41: one
pin move, to 0.18.0, instead of 0.17.1. D42: the tools server takes out a null `expected_sha256` before the
call, since harness 0.16.1 refuses an undeclared field. D43: the b10x golden recorded again on the emulator
with the released 0.18.0 binary after its SHA256SUMS check; D44: the trace domain's `transcript_digest`
follows it.

Findings and what the coordinator decided:

- Warning (invariant review F1): the H3 record on the secret route was one fixed sentence, the same whether
  the session-bus variables were forwarded or not. D46: the sentence is built from what was forwarded.
- Warning (adversary): `process.run` with `timeout_ms: null` was refused by harness 0.18.0 as undeclared,
  breaking a45 section 2 at the new pin. D45: every null argument the caller catalogue does not require is
  taken out before the call; a non-null undeclared field stays an open item in a50 section 2.
- Warning (adversary): the aep-eval native arm still calls `workflow`, which 0.18.0 removed. Owned by
  `story:aep-eval-native-arm-removed`, built in parallel.
- Note: the `split_once` to `rsplit_once` mutant of the name check survived the unit's tests; the adversary's
  empty-first-segment case catches it and is kept.
- Note: the name check is shape only; names the loop refuses still spawn. Kept as a50 section 5 decides.
- Note: `XDG_RUNTIME_DIR` is forwarded beside `DBUS_SESSION_BUS_ADDRESS` although the client reads it only
  as a fallback; accepted, one variable.

Checked and not faulted: no secret value reaches an event, output, argv or the child environment;
metaharness opens no secret store; flag-like names stay one argument; one harness and one tools revision in
the lock; nine harness citations at `330e2cde`.

Revision 2 by `aep:implementor`: both red cases green. Gate `w8u6r` exit 0 on every step, 1306 passed,
0 failed, 49 ignored; ESS conformance 116/116.

```findings
[
{"file":"crates/metaharness/src/builder.rs","line":null,"category":"hermetic-attestation","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"the H3 record on the secret route did not say whether the session-bus variables were forwarded; revision 2 builds it from what was forwarded"},
{"file":"crates/metaharness-tools/src/server.rs","line":null,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"a null timeout_ms on process.run was refused by harness 0.18.0 as undeclared; revision 2 takes out every null argument the caller catalogue does not require"},
{"file":"crates/metaharness-aep-eval/src/lib.rs","line":null,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"the native arm calls workflow, which 0.18.0 removed; owned by story:aep-eval-native-arm-removed"},
{"file":"crates/metaharness-b10x/src/lib.rs","line":null,"category":"test-coverage","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the split_once to rsplit_once mutant survived; the adversary's empty-first-segment case catches it"},
{"file":"crates/metaharness-b10x/src/lib.rs","line":null,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"shape-only name validation admits names the loop refuses; kept as a50 section 5 decides"},
{"file":"crates/metaharness/src/builder.rs","line":null,"category":"least-privilege","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"XDG_RUNTIME_DIR is forwarded beside DBUS_SESSION_BUS_ADDRESS; accepted"}
]
```
