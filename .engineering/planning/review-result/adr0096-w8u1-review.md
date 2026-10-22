---
format: aep.planning-md/3
id: review-result:adr0096-w8u1-review
kind: review-result
status: active
title: Wave 8 unit 1 adversary review
relations:
- reviews: story:mcp-arguments-checked-against-declared-types
revision: 1
---
# Review: wave 8 unit 1, catalogue call arguments checked against their declared types

`aep:adversary` pass 1 over the uncommitted unit 1 change (base `8444f4c`), 2026-10-22. New cases in
`crates/metaharness-tools/tests/adversary_w8u1.rs`, three red. Of twelve mutants of the type walk, the unit's
own suite caught seven; with the adversary file all twelve are caught.

Findings and what the coordinator decided:

- Warning: `expected_sha256: null` on `code.write` and `code.edit` was refused by the digest guard as asking
  for a check nothing here runs. D21: null on a field that is not required reads as absent, so the call is
  performed without the field; a45 section 2 amends a32 section 5.
- Note: a required field given as null was refused without naming its declared type. D22: a required field
  left out or given as null is named with its declared type at every depth.
- Note: `-0` was described as a number with a fraction or an exponent. D23: `-0.0` is refused and named as
  negative zero.

Revision 1 by `aep:implementor`: the three red cases green. It found that the workspace build turns on
serde_json `arbitrary_precision` through the AEP crates, so the `metaharness` binary holds `-0` as the
integer 0 and admits it, while the crate built alone refuses it. D39: accepted as stated in a45 and the
CHANGELOG; refusing `-0` in the binary needs the raw request text and is left open in a45 section 2. The
walk test asserts which way the build holds `-0`. Gate `w8u1r2` exit 0 on every step, 1223 passed,
0 failed, 49 ignored; ESS conformance 107/107 on the unit.

```findings
[
{"file":"crates/metaharness-tools/src/server.rs","line":null,"category":"correctness","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"expected_sha256 null on code.write and code.edit was refused by the digest guard; revision 1 reads null on a field that is not required as absent and performs the call"},
{"file":"crates/metaharness-tools/src/caller.rs","line":null,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"a required field given as null was refused without naming its declared type; revision 1 names the type for a required field left out or given as null at every depth"},
{"file":"crates/metaharness-tools/src/caller.rs","line":null,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"-0 was described as a number with a fraction or an exponent; revision 1 names -0.0 as negative zero, and the workspace build holds -0 as the integer 0 (a45 section 2, open)"},
{"file":"crates/metaharness-tools/tests/adversary_w8u1.rs","line":null,"category":"test-coverage","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the unit suite caught 7 of 12 mutants of the type walk; the adversary's cases catch all 12 and are kept"}
]
```
