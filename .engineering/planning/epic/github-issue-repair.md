---
format: aep.planning-md/3
id: epic:github-issue-repair
kind: epic
status: draft
title: Resolve Metaharness GitHub issues 10 through 15
owner: metaharness
refs:
- provider: github
  reference: beyond10x/metaharness#10
- provider: github
  reference: beyond10x/metaharness#11
- provider: github
  reference: beyond10x/metaharness#12
- provider: github
  reference: beyond10x/metaharness#13
- provider: github
  reference: beyond10x/metaharness#14
- provider: github
  reference: beyond10x/metaharness#15
relations:
- informed_by: epic:runs-side-by-side
revision: 4
---
## Requested outcome

Resolve the six open GitHub issues inventoried on 2026-10-03 against remote main 13a8378697fec1462b1014b04e5eb556bcba77cd, using the latest planning store schema. This is an interactive operator session. No implementation has been dispatched.

## Inventory

- #10: refresh the AEP dependency and verify live-eval version admission; crates/metaharness-aep/Cargo.toml still pins AEP 0.55.0 while installed AEP is 0.68.0.
- #11: preserve explicit Codex task_complete failure through normalized terminal events and CLI exit status, with synthetic regression vectors and bounded live failure evidence; crates/metaharness-codex/src/rollout.rs currently leaves error fields absent in finish.
- #12: explicitly authorized mapping of managed wt-* workspaces without weakening confinement or worktree ownership; crates/metaharness-aep/src/drive.rs and crates/metaharness/src/builder.rs check ws_ basenames.
- #13: governed Codex executor through the existing adapter, including genuine per-call decisions, unknown cost and resume semantics; depends on #11. Current Harness::named in crates/metaharness-aep/src/drive.rs admits Claude and b10x only.
- #14: explicit uncapped budget policy with durable authorization, resume consistency and honest accounting; coordinate AEP types as required, retain finite defaults and refusals.
- #15: verified adapter compatibility evidence for intended Claude, Codex and b10x releases before advancing pins; real transcripts remain private and native observation is not governed-drive evidence.

## Completion

Each issue needs its acceptance evidence, repository gates, bot publication and verified resulting issue state. Planning is not completion. Preserve unknown and unverified labels. Live work must remain bounded and paid evals stay outside task check.

## Current state

The AEP 0.68.0 migration to aep.project/5 verified all 26 pre-existing artifacts, 38 transitions and 19 evidence records. Its commit is refused by common Gates because two imported historical evidence references contain personal paths. AEP exposes no evidence-redaction command; evidence is append-only. No gate bypass, exception or direct evidence edit has been performed. Resolve this publication blocker before dispatching implementation branches from the migrated plan.

## Integration instruction

Operator instruction, 2026-10-03: collect all implementation on integration/metaharness-issue-repair and submit only one pull request later. No pull request is authorized at this stage. Internal unit branches, if needed for isolated implementation, merge into this integration branch; do not submit per-issue pull requests.

## Additional source findings

For #14, current AEP delegates dollar-budget admission to the concrete ExecutionHost and persists host fields. An explicit uncapped USD policy is locally feasible; do not report the finite Money parser as a blanket blocker. Entirely unlimited inner loops are different: upstream DriverOptions.max_iterations and StateSteps.visit_budget are finite integers, with omitted visit budget defaulting to three. The issue requests uncapped spending; do not silently expand its scope into unlimited iteration/retry semantics.

The ESS inventory identified session terminal evidence, workspace admission and governed invocation/accounting as the smallest relevant domains. Preserve dependency-owned Workspace/Lease and workflow types rather than creating competing lifecycle owners. Codex Bash hook naming has live evidence; apply_patch and some allow behavior remain explicitly unverified. Full governed Codex support requires evidence beyond an enum and argv change.

The issue repair is partially decomposed into three source-scoped stories. Implementation dispatch has not started; one isolated test-only worker reproduces #11. A complete decomposition review and implementation review remain due before delivery.

## Paused handoff

All requested plan changes and the #11 regression patch are collected in the integration worktree on integration/metaharness-issue-repair, based on 13a8378697fec1462b1014b04e5eb556bcba77cd. No new commit, push, PR or issue closure has succeeded. The primary checkout remains unchanged. The test-only worker is complete; its patch is also retained in scratch and copied to integration. These tests intentionally fail until #11 is fixed; no green gate is claimed.

The operator has been asked for a narrow exception to redact only the personal home-directory prefix in two immutable imported evidence references, preserving originals privately. The answer is pending. Do not infer approval from silence. After that decision, make only the permitted evidence correction (or wait for supported AEP redaction), rerun validation and bot commit, finish design/spec preparation and independent plan review, implement and independently review the issues, and run the complete repository gate. Submit no PR until the operator's later submission instruction; the eventual PR collects all work.
