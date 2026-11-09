---
format: aep.planning-md/3
id: release-plan:wave-github-zero-2026-11-09
kind: release-plan
status: draft
title: 'Wave plan of epic github-zero: four waves, one pull request, no release'
summary: Six stories in four waves on epic/github-zero; no version bump, no tag
revision: 1
---
## What this page is

The wave plan and wave page of `epic:github-zero-2026-11-09`. It is a plan for one pull request and
**no release**: no version bump, no tag, no published release. Acceptance:
`approval-record:github-zero-acceptance-2026-11-09` (`non-interactive`). Base: `main` at `bff6a66`
(0.30.0). Integration branch `epic/github-zero`, tree `mh-gh-zero-int`. Skill: AEP implementing 0.26.0.

## Plan

Derived with `aep plan artifact waves --kind story --status active --format json` on 2026-11-09
(aep 0.79.0), in the verb's order. Every story the verb placed is in this epic.

| Wave | Story | Objective |
| --- | --- | --- |
| 1 | `story:claude-memory-canary-deeper-ancestors` | O3 |
| 1 | `story:eval-cgroup-root-made-absolute` | O3, O6 |
| 2 | `story:loom-run-adapter` (issue 28, adapter half) | O3 |
| 3 | `story:eval-names-an-undelegated-cgroup-root` | O3, O6 |
| 3 | `story:h11-not-ok-on-unprobed-memory` | O3 |
| 4 | `story:claude-task-text-mentions` | O3 |

Collisions printed (the reason the waves are four): `crates/metaharness-aep-eval/src/lib.rs` (cited)
between the two eval stories; `crates/metaharness-claude/src/lib.rs`,
`crates/metaharness-claude/tests/native_fixture.rs`, `crates/metaharness/src/builder.rs` and
`docs/design/metaharness-protocol-v0.1.md` among the Claude stories and the Loom adapter;
`CHANGELOG.md` among all. Unassessed: none. Cycles: none.

Not in the plan: `story:loom-native-default` (issue 28, default half), held by
`decision-blocker:loom-native-default`.

## Units

One unit at a time; at most one sub-agent runs at once. Each unit is a managed worktree under the
worktree tool's tree root (`metaharness/<tree>`), forked from `epic/github-zero`, building into its
own `target/`; every cargo call goes through the shared build slot. Scratch is outside the
repository in the coordinator's scratch directory.

| Wave | Story | Branch | Tree | Head | Stage |
| --- | --- | --- | --- | --- | --- |
| 1 | `story:eval-cgroup-root-made-absolute` | `impl/eval-cgroup-root-made-absolute` | `mh-gh-zero-u1` | | planned |
| 1 | `story:claude-memory-canary-deeper-ancestors` | `impl/claude-memory-canary-deeper-ancestors` | `mh-gh-zero-u2` | | planned |
| 2 | `story:loom-run-adapter` | `impl/loom-run-adapter` | `mh-gh-zero-u3` | | planned |
| 3 | `story:eval-names-an-undelegated-cgroup-root` | `impl/eval-names-an-undelegated-cgroup-root` | `mh-gh-zero-u4` | | planned |
| 3 | `story:h11-not-ok-on-unprobed-memory` | `impl/h11-not-ok-on-unprobed-memory` | `mh-gh-zero-u5` | | planned |
| 4 | `story:claude-task-text-mentions` | `impl/claude-task-text-mentions` | `mh-gh-zero-u6` | | planned |

## Commits the plan makes

The opening store commit; per unit its commits and the adversary's test commit and the merge into
`epic/github-zero`; per wave a closing store commit; the squash merge of the one pull request into
`main` through the bot. No version bump, no tag, no release.
