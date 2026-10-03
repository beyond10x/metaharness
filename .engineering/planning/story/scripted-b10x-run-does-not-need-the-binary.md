---
format: aep.planning-md/3
id: story:scripted-b10x-run-does-not-need-the-binary
kind: story
status: implemented
title: A scripted b10x run does not require b10x-harness on the PATH
summary: Metaharness::start with a ScriptedRunner refuses to start a Kind::B10x run when b10x-harness is absent from the PATH (Launch refusal at crates/metaharness/src/builder.rs:547), although the scripted runner starts no process; the Gate workflow works around it by installing the binary. Observed 2026-09-15 in Gate run 34911935585.
refs:
- provider: github
  reference: beyond10x/metaharness#16
relations:
- decomposes: epic:github-issue-repair
- depends_on: story:codex-terminal-failure
- depends_on: story:managed-workspace-admission
scope:
- confidence: cited
  path: .github/workflows/gate.yml
- confidence: cited
  path: crates/metaharness/src/builder.rs
- confidence: cited
  path: crates/metaharness/src/process.rs
- confidence: cited
  path: crates/metaharness/src/scripted.rs
- confidence: cited
  path: crates/metaharness/tests/stream_closed.rs
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T07:24:43Z", actor: "human:timo", revision: 9}
- {from: "proposed", to: "active", at: "2026-10-03T07:24:43Z", actor: "human:timo", revision: 10}
- {from: "active", to: "implemented", at: "2026-10-03T08:45:34Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"test_result":2,"verification":1}}, executor: "agent:codex-metaharness", correlation: "issue-repair-wave"}
---
## Context

GitHub #16 corroborates the existing story's summary: start_b10x resolves a real binary and probes its version before invoking a supplied ScriptedRunner, even though that runner starts no vendor process. Source: crates/metaharness/src/builder.rs:544 and :707; ScriptedRunner in crates/metaharness/src/process.rs; every_harness_kind_closes_its_stream in crates/metaharness/tests/stream_closed.rs; CI installation accommodation in .github/workflows/gate.yml. The historical CI run in the original summary has not been re-inspected.

## Acceptance

An isolated scripted b10x execution with no b10x-harness executable completes its synthetic stream without spawning a vendor process, while a real execution with the same unavailable binary is still refused before launch and real version validation remains enforced.

## Scope

Cited: crates/metaharness/src/builder.rs, crates/metaharness/src/process.rs, crates/metaharness/tests/stream_closed.rs and .github/workflows/gate.yml. Inferable: a runner capability method or explicit launch contract separates process validation from synthetic execution; decide at the runner seam rather than by a test-only environment bypass. Committed executable test helpers must be Rust.

## Integration order

After story:codex-terminal-failure (stream_closed.rs collision) and story:managed-workspace-admission (builder.rs collision). Record a red executable-free regression first. Remove the CI binary installation only after full task check proves it unnecessary. Never make real launch/version probes optional or fake their evidence.

## Out of scope

Changing real b10x compatibility pins, claiming live compatibility, or changing terminal semantics owned by #11.
