---
format: aep.planning-md/3
id: story:ess-specification-and-hardening
kind: story
status: draft
title: Specify Metaharness in ESS and harden its conformance contract
owner: metaharness
relations:
- informed_by: epic:runs-side-by-side
- informed_by: epic:github-issue-repair
- depends_on: story:uncapped-spending-policy
- depends_on: story:current-adapter-compatibility
scope:
- confidence: cited
  path: Taskfile.yml
- confidence: inferred
  path: crates/metaharness-aep/src/drive_tests.rs
- confidence: inferred
  path: crates/metaharness/tests/ess_conformance.rs
- confidence: inferred
  path: spec
revision: 7
---
## Context

The operator requested an AEP ticket on 2026-10-03 to use ESS for Metaharness, improve an existing specification if present, and then harden it. Inventory at source 13a8378697fec1462b1014b04e5eb556bcba77cd found no system.yaml or ess-inputs.yaml. Existing authority is AGENTS.md (Invariants and Safety envelope), docs/design/metaharness-protocol-v0.1.md, and the protocol, core, adapter and AEP executor crates. This is a retrofit of existing behavior, not authorization to invent new runtime entities.

## Outcome

Metaharness has an executable specification derived from its actual contracts and code, and evidence that its conformance checks detect broken declared rules. This supports organization objectives O3 (observed and comparable harnesses) and O6 (measured improvement).

## Acceptance

The repository's task check changes from having no ESS gate to validating its specification and executing real-target conformance with at least one executed passing named scenario for each required obligation—terminal outcomes, unknown evidence, frame integrity, confinement, tool decisions and accounting—with no required scenario skipped, and every hardening technique reported as successful has a recorded planted-defect failure followed by a restored green run.

## Procedure

1. Recheck for existing ESS inputs before starting; improve them if another change has added them, otherwise apply ess:retrofitting with ess:specifying, citing source locations and recording every unresolved mapping.
2. Model existing session, normalized terminal evidence, sealed frame, workspace eligibility and governed invocation behavior before planning additions around those entities. Preserve the frame/1 wire contract and adapter/core boundary. Distinguish observed behavior from desired fixes in GitHub issues 10 through 15.
3. Use ess:testing-conformance to connect synthesized named scenarios to the real implementation. Rust is required for every committed executable, with clap derive for CLIs. Do not claim a reference-model-only run proves Metaharness conformance. Record passed, failed, skipped and excluded counts.
4. After the suite is green, apply ess:hardening: early design-to-spec review, breaking-change classification, mutation audit and guard analysis, then bounded sequence, determinism, metamorphic and caller replay checks where relevant. Each technique must catch a planted defect before its green result counts. Report techniques not run and why.
5. Record spec gaps, implementation defects and technique false positives separately in AEP. Keep paid evaluation out of task check and private transcripts out of source. Preserve all safety-envelope refusals.

## Scope

Inferred: new ESS inputs, Rust conformance integration and Taskfile.yml; existing authority: docs/design/metaharness-protocol-v0.1.md, crates/metaharness-protocol/src, crates/metaharness/src, crates/metaharness-aep/src and adapter crates. The implementation owner must refine exact paths before concurrent scheduling.

## Evidence required

ESS validation and synthesized suite identity; real-target conformance report; named planted-defect and restoration results; design-review findings and dispositions; complete task check exit status. A validated draft alone does not close this ticket.

## Draft contract and integration order

The initial isolated draft declares metaharness.session, metaharness.workspace and metaharness.spending plus an exact separately validated monetary projection. ESS 0.51.0 cannot synthesize Binary64 codecs, so optional f64 observations remain typed and explicitly outside generated executable coverage. No decimal/string substitute or fabricated zero is authorized. Generated scenarios and production tests must distinguish absent error from explicit error:null; the initial Optional-only projection was corrected before integration. Final conformance wiring follows the runtime stories to avoid drive_tests.rs and adapter collisions; pure specification drafting precedes them. Frame/tool obligations remain required by acceptance and may be authored real-target scenarios rather than invented duplicate models.
