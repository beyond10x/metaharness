---
format: aep.planning-md/3
id: story:managed-workspace-admission
kind: story
status: draft
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
  path: crates/metaharness-aep/src/drive.rs
- confidence: cited
  path: crates/metaharness-aep/src/drive_tests.rs
- confidence: inferred
  path: crates/metaharness-b10x
- confidence: cited
  path: crates/metaharness/src/builder.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
revision: 7
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
