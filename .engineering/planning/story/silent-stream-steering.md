---
format: aep.planning-md/3
id: story:silent-stream-steering
kind: story
status: active
title: Deliver steering while native output is quiet
relations:
- decomposes: epic:github-issue-repair
- informed_by: story:current-adapter-compatibility
scope:
- confidence: cited
  path: crates/metaharness-aep/src/drive/ess_conformance.rs
- confidence: cited
  path: crates/metaharness-cli/src/lib.rs
- confidence: cited
  path: crates/metaharness-cli/tests
- confidence: cited
  path: crates/metaharness/src
- confidence: cited
  path: crates/metaharness/tests
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: cited
  path: spec/domains/session.yaml
- confidence: cited
  path: spec/generated
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T10:57:59Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1}}}
- {from: "proposed", to: "active", at: "2026-10-03T10:57:59Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Reproduction and acceptance

Issue15 native cancellation qualification found a production liveness gap. The credential-free Codex fixture holds an in-progress Responses stream open for10seconds, sends the existing halt command over the actual Metaharness CLI stdin after the model request arrives, and requires termination within5seconds. cancel-timing.log records exit101, one failed native test: steering must not wait for the provider's ten-second stream to close. Actual command.result ultimately reports ok and stream.closed preserves steer-halt plus native signal9, so this is delayed delivery rather than absent kill semantics. Initial wire-shape assertions were corrected from the declared protocol before this timing run.

Existing Command/Halt/Interrupt and stream termination entities own the behavior; no new wire entity is proposed. The named acceptance scenarios are quiet-native-halt, quiet-native-interrupt, pending-decision-steering and quiet-output-is-not-eof. Add a normal, deterministic regression at the production run/CLI seam; the native fixture must turn green without waiting for provider output. Preserve tool-decision deadlines, exactly one final closure, command result identities and native process outcome. Make any required specification/design amendment before implementation. Integrate the named scenario into the production ESS target where that bounded model can truthfully exercise it; do not equate a synthetic model with native timing.

## Ranked hypotheses

1. Blocking next_line/next_event prevents CLI stdin servicing until a vendor event arrives. Predict: a quiet-stream regression fails, while yielding a polling idle state permits prompt command delivery without changing the vendor protocol.
2. The fixture's stdin write is buffered or malformed. Predict: explicit flush or an already available event will leave delivery delayed if the reader is responsible; the observed command.result id and ok outcome already disfavor malformed input.
3. Native kill/wait is slow. Predict: command acceptance is prompt but closure late; compare command-delivery timestamps against native process closure before changing kill behavior.

## Scope and verification

Core process polling and run loop, CLI drive loop, their regression tests, binding protocol design and corresponding existing ESS session domain/target. Native test extension in crates/metaharness-codex/tests/native_fixture.rs is coordinator-owned and must not be edited by the implementation worker. Other current vendor fixtures run in independent units; no shared checkout. Rust only. Source changes require full task check, independent review and the original native red loop rerun before integration; no pin change or paid request.
