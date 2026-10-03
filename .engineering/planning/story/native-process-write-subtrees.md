---
format: aep.planning-md/3
id: story:native-process-write-subtrees
kind: story
status: implemented
title: Expose explicit confined subprocess write directories
refs:
- provider: github
  reference: beyond10x/metaharness#21
relations:
- decomposes: epic:github-issue-repair
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/metaharness-aep/src/drive/ess_conformance.rs
- confidence: cited
  path: crates/metaharness-b10x
- confidence: cited
  path: crates/metaharness-cli/tests
- confidence: cited
  path: crates/metaharness-protocol/src/spec.rs
- confidence: cited
  path: crates/metaharness/src/builder.rs
- confidence: cited
  path: crates/metaharness/tests
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: cited
  path: docs/research
- confidence: cited
  path: spec/conformance.json
- confidence: cited
  path: spec/domains/workspace.yaml
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T11:11:24Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-03T11:11:24Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-03T12:02:42Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Context and typed home

GitHub21 was discovered on the release refresh after the initial10–20 inventory. The all-open-issues goal includes it. Harness0.13.3 at798325f03cf5a18df8fadb346d31b314826136ec accepts repeated process-write-subtree values, keeping subprocess workspace access ReadOnly by default and independent from file-tool write_scope. The current Metaharness launch/RunSpec does not forward that declaration. This is source-established; the reporter did not observe a confined Cargo failure because process capability was withheld for missing cgroup I/O facts.

Before this story, managed unit wt-eb99f248effb drafted ProcessWriteSubtree(String) and ProcessWriteDeclaration(List<ProcessWriteSubtree>) in the existing ESS workspace domain. ess specify validate --path spec --strict-requires exited0: metaharness v1 — 6 file(s), valid. This is a typed home, not a completed conformance claim.

## Acceptance

Named production-target scenarios process-read-only-default, process-explicit-subtrees, process-invalid-directory and file-scope-is-not-process-scope verify the public CLI/builder declaration and native argv admission. Empty stays read-only. Accept only exact supported workspace-relative directory declarations under the selected native contract, preserve invalid/root/traversal/glob/overlap refusals, and never infer namespace mounts from file globs or silently widen confinement. Preserve behavior for adapters that do not support this control by a named refusal. Record source revision, installed version and pin distinctions; do not claim older binaries support a newly forwarded flag.

Add meaningful real containment regressions: declared build directories may be written while undeclared siblings/outside paths cannot. If this host withholds execution capability, report and preserve that limitation rather than substituting an unconfined write or claiming native success. Dependency-closure/toolchain mounting remains separate. No paid requests, consumer shim or fallback. Rust only.

## Scope and sequence

Protocol RunSpec field/public CLI derive, Metaharness builder/native launch argv and owning tests, binding design, existing ESS workspace domain/production target/generated suite, research and changelog. Amend the binding design and run failing source/argv regressions before production. Serialize integration with the ongoing polling/conformance unit; the generated suite must be rebuilt from the combined model. The coordinator owns this managed unit and may delegate implementation after current native fixtures finish. One existing PR22 receives the eventual changes; no second PR.
