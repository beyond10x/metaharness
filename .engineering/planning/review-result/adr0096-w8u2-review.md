---
format: aep.planning-md/3
id: review-result:adr0096-w8u2-review
kind: review-result
status: active
title: Wave 8 unit 2 adversary review
relations:
- reviews: story:narrowed-cap-refused-before-persist
revision: 1
---
# Review: wave 8 unit 2, a resumed cap below the assumed charge is refused before it persists

`aep:adversary` pass 1 over the uncommitted unit 2 change (base `8444f4c`), 2026-10-22. Seven new cases in
`crates/metaharness-cli/tests/adversary_w8u2.rs`, two red. The code was not broken: boundaries at one
millionth, zero, a raise, legacy terms and the order of refusal before lock and write all held. Of five
mutants of the floor, the unit's own suite missed one (cap and charge swapped in the message) and caught two
others only through ESS; the adversary's cases kill all five.

Findings and what the coordinator decided:

- Warning: `README.md:119` and `website/docs/quickstart.md:52` let a resumed cap narrow without naming the
  floor. D17: both state that the cap narrows down to the remembered assumed charge and not below it.
- Note: `metaharness aep drive resume --help` carries the pinned AEP text, which names no floor. D18: the
  hosted command overrides the `--budget-usd` help with clap `mut_arg`; a test fails by name if the upstream
  argument is renamed; a46 records the override.
- Warning: the unit's checks tested only that each amount appears somewhere, so the swapped-message mutant
  survived them. Kept the adversary's labelled check; no further change.
- Note: the `!transcripts.exists()` assertions in `aep_resume.rs` could not fail. D19: removed; the
  byte-identical tree comparison shows nothing launched.
- Note: a46 named no exit code. D20: a46 section 2 states exit 1.
- D33: a clippy `format_push_string` fix in the adversary file, changing no expectation.

Revision 1 by `aep:implementor`: the two red cases green; gate `w8u2r2` exit 0 on every step, 1221 passed,
0 failed, 49 ignored; ESS conformance 111/111 on the unit, 112/112 after the merge into integration.

```findings
[
{"file":"README.md","line":119,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"README said a resumed finite cap may only narrow without the a46 floor at the remembered assumed charge; revision 1 states the floor"},
{"file":"website/docs/quickstart.md","line":52,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"the quickstart said a resume may narrow its recorded budget without the floor; revision 1 states it"},
{"file":"crates/metaharness-cli/src/lib.rs","line":null,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"resume --help carried the pinned AEP text with no floor; revision 1 overrides the budget-usd help with clap mut_arg and a test fails if the upstream argument is renamed"},
{"file":"crates/metaharness-cli/tests/aep_resume.rs","line":333,"category":"test-coverage","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"the unit's checks only tested that each amount appears, so swapping cap and charge in the message survived; the adversary's labelled case kills it and is kept"},
{"file":"crates/metaharness-cli/tests/aep_resume.rs","line":341,"category":"test-coverage","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the transcripts-absent assertions could not fail; revision 1 removes them and the byte-identical tree comparison remains the proof"},
{"file":"docs/design/metaharness-protocol-v0.1.md","line":null,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"a46 named no exit code for the refusal; revision 1 states exit 1"}
]
```
