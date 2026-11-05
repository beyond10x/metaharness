---
format: aep.planning-md/3
id: story:aep-version-reads-bounded
kind: story
status: implemented
title: The AEP crates read --version under the probe deadline
tags:
- from-finding
relations:
- derived_from: review-result:wave-1104-u2-adversary-1
scope:
- confidence: cited
  path: crates/metaharness-aep-eval/src/lib.rs
- confidence: cited
  path: crates/metaharness-aep/src/drive.rs
- confidence: cited
  path: crates/metaharness-aep/src/eval.rs
- confidence: cited
  path: crates/metaharness/src/builder.rs
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-05T01:50:52Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-05T01:50:52Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-05T02:52:37Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}}
---
## Context

`story:version-probe-everywhere-bounded` bounded the `--version` reads of `doctor` and the b10x launch
under the probe deadline of amendment a65. Its adversary pass (review-result:wave-1104-u2-adversary-1)
found three more `--version` reads with no deadline, all unbounded at base 2dc4d26 and outside that
story's scope:

- `crates/metaharness-aep/src/drive.rs` about line 2851 (`metaharness aep drive --aep-binary`)
- `crates/metaharness-aep/src/eval.rs` about line 159
- `crates/metaharness-aep-eval/src/lib.rs` about line 1378

A binary whose `--version` never exits blocks each of them.

## Outcome

Every `--version` read in the AEP crates goes through the same bounded probe, so a binary that never
exits is refused by name within the deadline.

## Acceptance

1. A stub `aep` whose `--version` never exits makes `metaharness aep drive --aep-binary <stub>` and the
   AEP eval's version check return within the deadline with a refusal that names the deadline.

## Out of scope

- The reads already bounded by `story:version-probe-everywhere-bounded`.
