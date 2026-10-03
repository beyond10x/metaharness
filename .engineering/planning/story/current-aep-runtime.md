---
format: aep.planning-md/3
id: story:current-aep-runtime
kind: story
status: draft
title: Link the current AEP release and admit its matching eval executable
owner: metaharness
refs:
- provider: github
  reference: beyond10x/metaharness#10
relations:
- decomposes: epic:github-issue-repair
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: crates/metaharness-aep
- confidence: cited
  path: crates/metaharness-aep-eval/src/lib.rs
- confidence: inferred
  path: evals
revision: 7
---
## Context

crates/metaharness-aep/Cargo.toml pins eight AEP packages to 28abe09bb6e5b0a6b4db839f6bf5693957d39324 (0.55.0). The installed and newest release is AEP 0.68.0, tag peeled to 6d7a44d3607d2d9a6ffdf0a165993c546c43d0db. GitHub #10 reports EVAL-RUN-017 refusing the newer installed executable. crates/metaharness-aep/src/eval.rs:165 compares against linked aep_cli::VERSION, so preserve the exact equality requirement. Scope investigation found current upstream command admission recognizes aep rather than retired protocol spellings; migrate affected fixtures and prompts rather than widening admission.

## Acceptance

The locked workspace links released AEP 0.68.0, the eval preflight accepts the matching child-PATH executable and still refuses a different version, and offline drive/eval contract suites and task check pass without weakening version or capability admission.

## Scope

Cited: crates/metaharness-aep/Cargo.toml, Cargo.lock, crates/metaharness-aep/src/drive.rs, drive_tests.rs, eval.rs and eval_tests.rs; inferred: retired command spellings in eval fixtures and documentation. No AEP source change, public dependency path override or paid run is required to prove preflight compatibility. Actual paid evaluation remains separately declared.

## Evidence

Record the old-pin version-admission refusal and matching-new-pin success with the same synthetic binary fixture; record nonmatching-version refusal. Exact version identity is the contract, not a test changed to accept any banner.

## Scope investigation

Read-only comparison against released AEP 0.68.0 found the scratch project generator in crates/metaharness-aep-eval/src/lib.rs:713 still writes aep.project/1. Update that fixture to aep.project/5 with store.git as part of the schema requirement. ExecutionHost and PreparedExecution signatures are unchanged. Upstream renamed PROTOCOL_BINARY to AEP_BINARY; local protocol command fixtures at drive_tests.rs:631-700 must use aep. No compilation was performed by the scoper, so source compatibility remains to be tested.
