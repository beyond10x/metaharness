---
format: aep.planning-md/3
id: release-plan:release-0-9-0
kind: release-plan
status: active
title: Publish Metaharness 0.9.0 after issue repair and native qualification
relations:
- delivers: epic:github-issue-repair
revision: 2
transitions:
- {from: "draft", to: "active", at: "2026-10-03T12:02:42Z", actor: "human:timo", revision: 2}
---
## Objective and authorization

The active operator goal is a new published Metaharness release with no open GitHub issues left to fix. Preserve one integration branch and one eventual PR. The release request authorizes the repository's bot-authenticated push, PR, merge, tag and release operations after their prerequisites pass; it does not supply a live-model spending limit. Version 0.9.0 is the proposed next minor release after verified published 0.8.0, reflecting the additive protocol and governed executor capabilities.

## Current evidence

As of 2026-10-03, remote main is 13a8378697fec1462b1014b04e5eb556bcba77cd; no open PR exists. All eleven GitHub issues #10–20 remain open. Integration source 3385656e with planning 19b788d6 passed 761 tests, 13 ignored and 36 ESS scenarios; 18 guard mutants were killed. This is not native adapter qualification or release evidence. The planning schema is aep.project/5.

## Completion requirements

1. Meet each issue's actual acceptance using the production implementation and bounded native observations where requested. Preserve unknowns and unsupported claims. Record source, child versions, command, terminal result, process status, hook decisions and costs including absence. Close issues through the bot only after their fixes and evidence ship; do not close them merely to empty the issue list.
2. Qualify Claude 2.1.288, Codex 0.153.4 and b10x 0.13.3 against the complete issue #15 matrix. Advance only claims actually observed. A missing spending authorization remains an external blocker.
3. Use the single integration PR, reconcile it with current remote main, obtain repository-required reviews/checks, and verify its exact candidate tree and bot author/committer. No hook, policy, or required check bypass.
4. Cut the changelog and README status together, update workspace version and lockfile, and run the full gate at the release candidate. Verify current required checks from GitHub branch rules rather than relying only on old documentation.
5. Merge the green reviewed candidate, verify exact main and its required checks, then create the annotated bare tag 0.9.0 through the bot route. Verify tag and version agreement.
6. Verify release workflow success, the published GitHub Release, all four archives (Linux and macOS, x86_64 and aarch64), SHA256SUMS, and their identities/content. Existing release workflow uses github.token writes; reconcile this with the current requirement that every GitHub write be b10x-bot[bot] before triggering publication. Publishing permission must not be weakened to work around this.
7. Re-query all open issues and the exact release, including any newly discovered defects in scope. Mark the goal complete only after the published release and zero remaining open fix issues are verified. Documentation publication may remain pending under the repository's source-release completion boundary.

## Open evidence handling

One unchanged uncommitted ESS import has an absolute local report_input. Preserve it and its private archive; no hand-edit, deletion or privacy-gate bypass. Use supported tooling or the pending explicit narrow operator exception before committing it. The later final report uses a relative input and is already committed.

## Execution ownership

Coordinator resumes under the integration worktree wt-8e8abedfd902 and its own lease. Native work is sequential and never part of task check. Private logs live under ~/.cache/metaharness-issue-repair. Completed unit worktrees have archive recovery; source is integrated locally and no branch was published as of this checkpoint.
