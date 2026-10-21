---
format: aep.planning-md/3
id: review-result:adr0096-w7u1-review
kind: review-result
status: active
title: 'Wave 7 unit 1 adversary review: trace-ir-summary/1'
relations:
- reviews: story:retire-trace-formats-beside-trace-ir
revision: 1
---
# Review: wave 7 unit 1, trace-ir-summary/1

`aep:adversary` pass 1 over the uncommitted unit 1 change (base `00cd0f8`), 2026-10-21. Ten findings and
two red test files (`crates/metaharness-protocol/tests/adversary_w7u1.rs`,
`crates/metaharness-cli/tests/adversary_w7u1.rs`), kept in the change.

Revision 1 by `aep:implementor` resolved every finding as the coordinator decided: `tokens_basis` and
`models` added to the summary, the per-vendor `input_tokens` semantics stated in amendment a42 (harness-json
labelled unverified), `render` refuses a non-finite cost, `float_roundtrip` enabled on `serde_json` in
`metaharness-protocol`, a one-stream refusal for `--to trace-ir-summary`, the a42 wording corrected, and the
conformance target reads every payload value from the rendered bytes. One adversary expectation changed to
the new refusal (`two_streams_for_one_summary_are_refused_in_words_that_fit`); no case deleted.

After revision 1: conformance 107/107; 17 planted mutants of `trace_summary.rs` each caught by at least one
layer; M3b, M4b, M4c, M6, M7, M8 and M14 by the protocol tests only.

```findings
[
{"file":"docs/design/metaharness-protocol-v0.1.md","line":3801,"category":"acceptance","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"a harness-json summary reads null for every token count although the b10x record states usage per request, so takt's native-arm rows lose their tokens; fixed by tokens_basis sum-of-requests"},
{"file":"crates/metaharness-protocol/src/trace_summary.rs","line":157,"category":"acceptance","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"model is the opening record's only, dropping haiku on Claude and reading null on Codex where observed_models states gpt-5.6-sol; fixed by the models list"},
{"file":"docs/design/metaharness-protocol-v0.1.md","line":3802,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"Claude terminal usage covers opus only while cost_usd includes haiku; fixed by per-model entries and the a42 wording"},
{"file":"crates/metaharness-codex/src/rollout.rs","line":493,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"input_tokens has vendor-dependent cache semantics; a42 now states them per vendor, unverified where no pinned source says so; no normalization in this unit"},
{"file":"crates/metaharness-protocol/src/trace_summary.rs","line":86,"category":"contract-drift","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"render's doc promises an error for a non-finite cost but serde_json writes null; fixed, render refuses"},
{"file":"Cargo.toml","line":44,"category":"boundary","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"without float_roundtrip a protocol-only build parses a 17-digit vendor cost 1 ULP off; fixed by the serde_json feature on metaharness-protocol"},
{"file":"crates/metaharness-cli/src/lib.rs","line":313,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"two streams under --to trace-ir-summary got the viewer refusal; fixed with a one-stream refusal"},
{"file":"docs/design/metaharness-protocol-v0.1.md","line":3819,"category":"contract-drift","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a42 claimed the unknown --to refusal kept its old words; corrected"},
{"file":"crates/metaharness-aep/src/drive/ess_conformance.rs","line":1685,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"SummarizeRecord re-serialised typed fields; fixed, the payload is read from the rendered bytes"},
{"file":"crates/metaharness-protocol/src/trace_summary.rs","line":111,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"terminal-node and opening-node choice survived the suite; fixed by keeping the adversary's two tests"}
]
```
