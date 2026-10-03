---
format: aep.planning-md/3
id: epic:github-issue-repair
kind: epic
status: draft
title: Repair Metaharness GitHub issues 10 through 20 and harden its ESS contract
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
- provider: github
  reference: beyond10x/metaharness#16
relations:
- informed_by: epic:runs-side-by-side
revision: 10
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

## Approved decomposition, current

The operator approved the wave on 2026-10-03. Planning evidence redaction is resolved and the schema-5 migration is committed; the earlier pause text is historical. Issues #10-16 now all map to stories: current-aep-runtime, codex-terminal-failure, managed-workspace-admission, governed-codex, uncapped-spending-policy, current-adapter-compatibility and scripted-b10x-run-does-not-need-the-binary. ESS adoption/hardening is the requested additional ticket. Current partial-order delivery: #10/#11, then #12, then #13 and #16 on disjoint surfaces after checking scopes, then #14/#15 as their shared edits allow, then final real-target ESS reconciliation and integration gate. All source outcomes remain local to one integration branch for one later PR.

## Open-issue inventory extension

A final read-only GitHub refresh on 2026-10-03 found new issues #17–20. The original #10–16 inventory was complete when taken; the user's all-issues scope now includes native process termination, authoritative final answer, observed model identity and tool outcome evidence. These four are recorded as stories and serialize after integration f3180f93 on the same branch. No second PR is created. The installed Codex 0.153.4 official source tag rust-v0.153.4 resolves to 042fb41b7c813ac7999105e886b2b7aa715b5081; source inspection is not a native execution claim. New native observations remain subject to the unanswered live budget.

## Integration checkpoint and handoff

All source changes are on integration/metaharness-issue-repair, source commit 3385656e50f3a8694d31896e4a2e80ad3247f7c0, managed tree wt-8e8abedfd902. No push, PR, main merge, release or paid probe has occurred. Seven stories are implemented: #10, #12, #13, #14, #16, #17 and ESS adoption/hardening. #11 and #18–20 remain active; #15 remains draft because current native qualification is unobserved. No issue closure is inferred from those statuses.

The final offline gate passed 761 tests, 13 ignored, and all 36 ESS scenarios. Hardening killed 18/18 guard mutants and caught the planted final-answer omission. Coordinator review is explicit; independent workers exhausted quota. The native budget decision remains open, and #20 additionally needs a verified observation path for legacy commands. docs/research/2026-10-03-codex-observations.md records that source gap.

Next owner: the coordinator continuing this approved wave after the operator resolves native probe spending. Use worktree inspect for wt-8e8abedfd902, acquire a new session lease, inspect exact Git/AEP status, and retain the one-PR integration boundary. Private evidence and worktree recovery inventory live under ~/.cache/metaharness-issue-repair. A single older evidence import remains uncommitted because report_input contains a local absolute path; it has not been edited or deleted.

## Source correction: persistent Codex exec history

Read-only inspection of official Codex rust-v0.153.4 exec/src/lib.rs at 042fb41b7c813ac7999105e886b2b7aa715b5081 changes the earlier inference: thread_start_params_from_config requests Paginated history when not ephemeral. Metaharness refuses --ephemeral. start_thread falls back to unspecified/default legacy only on the explicit server error that paginated threads require listing support. The enum default is therefore not proof that this selected exec path normally uses legacy history. Existing support for retained CommandExecution items can cover the normal persistent path; native qualification must observe which path was taken. Legacy/resumed/fallback records that omit status remain unknown. No additional history flag or alternate vendor parser is currently justified.
