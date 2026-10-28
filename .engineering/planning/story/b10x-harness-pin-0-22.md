---
format: aep.planning-md/3
id: story:b10x-harness-pin-0-22
kind: story
status: active
title: metaharness admits b10x-harness 0.22.0 and runs its specification checks under ESS 0.75.0
summary: 'Pin-only hop: harness 3eca074a (0.22.0, llm 0.16.0), ESS 0.75.0'
relations:
- decomposes: epic:layer-pins-follow-their-owners
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: crates/metaharness-b10x/Cargo.toml
- confidence: cited
  path: crates/metaharness-b10x/src/lib.rs
- confidence: cited
  path: crates/metaharness-tools/Cargo.toml
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-28T21:16:55Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-10-28T21:16:56Z", actor: "human:timo", revision: 7}
---
## Outcome

`metaharness run b10x --strict-version` admits `b10x-harness` 0.22.0, the harness crates metaharness
compiles are Harness 0.22.0's (`3eca074aea09574775ed340bb618e5322acaaf22`), which take llm 0.16.0,
and the specification checks run under ESS 0.75.0.

## Context

- Pins today: `HARNESS_REVISION` `ef6df52e` and `PINNED_VERSIONS` `["0.21.0"]`
  (`crates/metaharness-b10x/src/lib.rs:114-121`), the `rev` in `crates/metaharness-tools/Cargo.toml`
  and `crates/metaharness-b10x/Cargo.toml`.
- Harness 0.22.0 (2026-10-28) adds `--substrate-adopt`, moves Substrate to 0.14.0, llm to 0.16.0 and
  ESS to 0.75.0 (harness `CHANGELOG.md`). llm 0.16.0 is llm's newest tag; metaharness takes llm only
  through harness.
- ESS 0.75.0: commit `9ff83d98` (`deps: move ESS to 0.75.0`, the D158 sweep) on this wave's branch.
- The previous hop, story:b10x-harness-pin-substrate-0-12, re-recorded the golden loop record from
  the released archive against harness's emulator and ran a credential-free `--strict-version` run.

## Acceptance

1. `PINNED_VERSIONS` is `["0.22.0"]` and `HARNESS_REVISION` is `3eca074a...`; a 0.21.0 binary reads H9
   `gap` and `--strict-version` refuses it before the spawn.
2. `Cargo.lock` holds harness at `3eca074a` and `llm-*` at `0.16.0`; `cargo tree -i` shows no second
   harness revision.
3. `ess specify validate --path spec --strict-requires` and `--path spec/monetary` exit 0 under ESS
   0.75.0, and `spec/conformance.json` regenerates byte-identical.

## Scope

- `crates/metaharness-b10x/src/lib.rs`, `crates/metaharness-b10x/Cargo.toml`, `crates/metaharness-tools/Cargo.toml`,
  `Cargo.lock`, the b10x golden fixtures if the release changes the record, `CHANGELOG.md`, `README.md`.
