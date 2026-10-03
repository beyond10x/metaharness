---
format: aep.planning-md/3
id: story:ess-specification-and-hardening
kind: story
status: implemented
title: Specify Metaharness in ESS and harden its conformance contract
owner: metaharness
relations:
- informed_by: epic:runs-side-by-side
- informed_by: epic:github-issue-repair
- depends_on: story:uncapped-spending-policy
scope:
- confidence: cited
  path: .github/workflows/gate.yml
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: README.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/metaharness-aep/Cargo.toml
- confidence: cited
  path: crates/metaharness-aep/src/drive.rs
- confidence: cited
  path: crates/metaharness-aep/src/drive/ess_conformance.rs
- confidence: cited
  path: crates/metaharness-aep/src/drive_tests.rs
- confidence: cited
  path: crates/metaharness-codex/Cargo.toml
- confidence: cited
  path: crates/metaharness-codex/src/conformance.rs
- confidence: cited
  path: crates/metaharness-codex/src/lib.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: cited
  path: spec
revision: 19
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T08:02:13Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-03T08:02:13Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-10-03T08:45:35Z", actor: "human:timo", revision: 17, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"ess_conformance_coverage_v1":1}}, executor: "agent:codex-metaharness", correlation: "issue-repair-wave"}
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

Cited from the final diff: spec manifests, domains, coverage and conformance.json; crates/metaharness-aep/Cargo.toml, src/drive.rs, src/drive_tests.rs and src/drive/ess_conformance.rs; crates/metaharness-codex/Cargo.toml, src/lib.rs and src/conformance.rs; Cargo.lock, Taskfile.yml, .github/workflows/gate.yml and README.md. The binding design was amended before implementation. No production protocol types or frame bytes changed.

## Evidence required

ESS validation and synthesized suite identity; real-target conformance report; named planted-defect and restoration results; design-review findings and dispositions; complete task check exit status. A validated draft alone does not close this ticket.

## Draft contract and integration order

The initial isolated draft declares metaharness.session, metaharness.workspace and metaharness.spending plus an exact separately validated monetary projection. ESS 0.51.0 cannot synthesize Binary64 codecs, so optional f64 observations remain typed and explicitly outside generated executable coverage. No decimal/string substitute or fabricated zero is authorized. Generated scenarios and production tests must distinguish absent error from explicit error:null; the initial Optional-only projection was corrected before integration. Final conformance wiring follows the runtime stories to avoid drive_tests.rs and adapter collisions; pure specification drafting precedes them. Frame/tool obligations remain required by acceptance and may be authored real-target scenarios rather than invented duplicate models.

## Conformance implementation

Offline conformance targets the integrated runtime and existing version-bounded adapters; it does not require or advance current vendor pins. Removed the inferred dependency on story:current-adapter-compatibility because paid pin qualification remains separate and unchanged. Final suite wiring follows the implemented spending unit. Generated cases will execute production code in a Rust test target; no built-in ESS reference target is evidence. Existing generated outcome branches with indistinguishable observed results will be consolidated before declaring coverage. Exact monetary projection remains separately validated because ESS 0.51.0 declines Binary64 generation.

## Result

Unit b5ab07275743ed19898ee6a8c4124b212105f5c0 is integrated on integration/metaharness-issue-repair. ESS 0.51.0 validates both manifests; the declared ess-conformance/23 suite executes 24 production-target scenarios, all passed, zero skipped/unsupported/refused. spec_digest 89810481880cbf5c8a54c1f080c8f4e9890cf49e4a165cddf35588dfa89dbf4c; contract_digest c3574dcb450fda2ba697761202f63709cc8aef59d6c377805efcde937f9da72c. Native report/2 producer external-scenario-status/1;runner=metaharness-rust-target@1 is imported as evidence, with its exact suite.

Planted specification error: explicit failure changed to success, caught by FinishCodexCompletion/outcome/explicit-failure. Planted production error: removing uncapped admission persistence, caught by InvocationAdmission/invariant/at/metaharness.spending.RetainedAdmission/admission. Both restored and green. ESS guard-negate audit: 10 killed, 0 surviving, inconclusive, stillborn, unwitnessed or equivalent. Other technique exclusions and the Binary64 limitation are explicit in spec/coverage.md. Core generated Rust compiles (59 capabilities, four obligations, zero refusals); generated behavior is not production evidence.

The integrated task check exited 0: 744 passed and 13 existing ignored live tests. CI has not run. Source review is coordinator fallback, not independent review, because workers exhausted their host quota. Evidence is retained in local cache metaharness-issue-repair/ess-final and final-integration-task-check.log. No new vendor pin or paid run is claimed.

## Expanded final integration evidence

The #17–20 extension is integrated at 3385656e50f3a8694d31896e4a2e80ad3247f7c0. Its task check exited 0 with 761 tests passed and 13 ignored. ESS executes 36 generated scenarios, all passed, no skipped/unsupported/refused obligations. Final model digest d9ddef341a2de70c15e79ac60d68f70c624fe320fbfff69c6124165c2793d64c; suite digest sha256:96fd0a449234fa8475ec55615fb3bd596e5f1474ac65e23521f7ca134aa902ff. The native report is imported through a relative report_input and retained at ~/.cache/metaharness-issue-repair/observations1820/integration-ess-conformance-report.json with its exact suite.

The expanded guard-negate audit kills 18 of 18 mutants, with no survivor, inconclusive, stillborn, unwitnessed or equivalent result. Six mutants gain refusals but each is killed by executed scenarios; the unchanged baseline has zero refusals. A planted production final-answer omission failed ReadFinalAnswer/outcome/authoritative and restoration passed. Evidence: observations1820/{planted-answer.log,mutation-report.json,integration-task-check.log} in the same private cache. Earlier 24-scenario results remain historical evidence rather than being relabelled.

One earlier uncommitted evidence import contains an absolute local report_input. It is preserved unchanged pending the operator decision; the CLI has no redaction operation. Later evidence uses relative input paths. No historical evidence is hand-edited.

## Release candidate 0.9.0

The combined model executes45generated scenarios, allpassed with0skipped/unsupported/refused. Guard-negation audit kills25of25mutants,0survivors/inconclusive/unwitnessed/equivalent. Additional planted production idle/deadline/no-wire-interrupt defects fail named scenarios and are restored. Independent review found a possible integer comparison false-match past2^53; the Rust regression failed before exact decimal normalization and passes afterward. The full taskcheck exits0 onRust1.99:779passed,31ignored. Native opt-in fixtures are run separately and do not convert ignored counts into conformance coverage. The report/2 import uses relative paths and the exact45scenario suite. Prior24/36scenario evidence remains historical.
