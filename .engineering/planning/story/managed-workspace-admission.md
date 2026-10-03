---
format: aep.planning-md/3
id: story:managed-workspace-admission
kind: story
status: active
title: Admit explicitly selected managed workspaces under existing confinement
owner: metaharness
refs:
- provider: github
  reference: beyond10x/metaharness#12
relations:
- decomposes: epic:github-issue-repair
- depends_on: story:current-aep-runtime
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: crates/metaharness-aep/Cargo.toml
- confidence: cited
  path: crates/metaharness-aep/src/drive.rs
- confidence: cited
  path: crates/metaharness-aep/src/drive_tests.rs
- confidence: inferred
  path: crates/metaharness-b10x
- confidence: cited
  path: crates/metaharness/src/builder.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T07:16:20Z", actor: "human:timo", revision: 8}
- {from: "proposed", to: "active", at: "2026-10-03T07:16:20Z", actor: "human:timo", revision: 9}
---
## Context

Metaharness duplicates a ws_ prefix eligibility rule in crates/metaharness/src/builder.rs:1784 and crates/metaharness-aep/src/drive.rs:2425. The actual pinned Harness revision 90f10a4314c1c630691c85e812bd8d5d23d73fcc already accepts any valid single component of ASCII alphanumerics, underscore and hyphen, not beginning with a hyphen, in crates/harness-substrate/src/embedded.rs:213-250. Its pinned-root identity and openat2 checks remain authoritative; the prefix was an ID naming convention, not containment. Explicit cwd/project and confinement flags already select the operator-owned directory. Reuse that authorization surface and do not invent a second Workspace lifecycle.

## Acceptance

An explicitly selected real managed wt-* checkout is admitted to the existing confined native launch, relative project dot resolves identically to its canonical directory, invalid or escaping paths remain refused, and the worktree path, identity, lease and lifecycle ownership remain unchanged.

## Scope

Cited: crates/metaharness/src/builder.rs (canonical path resolution and eligibility), crates/metaharness-aep/src/drive.rs (native preflight and launch eligibility), their tests; inferred: shared predicate in crates/metaharness-b10x and design/docs clarification. No worktree rename, copy, lifecycle mutation or substrate confinement weakening. Existing upstream contract is sufficient; a dependency pin change is not required for this rule.

## Evidence

Synthetic red/green admission and invalid-name tests; an actual managed checkout identity/lease before and after a no-model confined probe; independent review of containment assumptions and full task check. A successful argv assertion alone does not establish live confinement.

## Integration order

Wait for story:current-aep-runtime because both edit drive.rs and drive_tests.rs. The coordinator already wrote the shared design amendment before implementation. This order addresses a source collision, not a semantic dependency on a newer Harness pin.

## Local implementation stage

Unit wt-ba0e905c48f6 selected exact integration base 710da207 after worktree create resolved symbolic HEAD against the primary; no primary files changed. Coordinator acquired its own unit lease. Two executable regressions fail on valid managed components/relative project selection; an initial misplaced test doc comment was corrected before counting the red result. The shared component predicate lives in metaharness-b10x and AEP takes that internal crate dependency rather than duplicating the vendor rule; Cargo manifests/lock scope is explicit.

A no-model b10x-harness 0.13.3 tools probe over the actual managed checkout with --substrate-embedded published file.write/file.edit. Directory and .git inode identity were unchanged; before/after managed registry inspections are retained in private workspace12 scratch. This is a bounded observation of installed 0.13.3, not a new compatibility pin or proof of every confinement operation.

## Package result

Two targeted regressions failed before the change and pass after it. Full package suite: 327 executed passing cases, 7 pre-existing ignored cases, exit0; clippy all-targets with denied warnings and formatting pass. The changed preflight explicitly refuses requested confinement over a missing/ineligible directory instead of silently omitting confinement flags. No substrate source or pin changed. The no-model probe and syntax tests are evidence of the bounded admission change, not a new claim about every sandbox operation. A separate agent review could not complete because worker usage was exhausted; coordinator review fallback and full integration gate remain visible obligations.
