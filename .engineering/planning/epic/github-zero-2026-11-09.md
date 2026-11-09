---
format: aep.planning-md/3
id: epic:github-zero-2026-11-09
kind: epic
status: active
title: Zero open GitHub issues and every store defect fixed, in one pull request
summary: 'Operator goal 2026-11-09: issue 28 and the open defect stories, one epic, one PR'
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T22:08:15Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-09T22:08:15Z", actor: "human:timo", revision: 3}
---
## Why

Operator goal, 2026-11-09, verbatim: "zero remaining issues on github, all bugs fixed from planning
store". Method, verbatim: "use /planning and /wave and /worktree:managing-worktrees - but in the end
provide 1 single PR to gh. make sure all are below 1 big epic for integration via bottoken from
./atlas".

## What it holds

Read on 2026-11-09 at `origin/main` `bff6a66`. `gh issue list -R beyond10x/metaharness --state open`
returned one issue, 28. The store's non-terminal stories and open blockers were read for ones that
describe wrong behaviour.

| Item | Story | Route |
| --- | --- | --- |
| Issue 28, the Loom adapter | `story:loom-run-adapter` | implemented in the wave |
| Issue 28, Loom as the native default | `story:loom-native-default` | held by `decision-blocker:loom-native-default`; the issue stays open |
| Defect: a relative `--cgroup-root` is read from two working directories | `story:eval-cgroup-root-made-absolute` | wave |
| Defect: a root that holds the eval but is not delegated fails late as a missing `run` tool | `story:eval-names-an-undelegated-cgroup-root` | wave |
| Defect: memory above the cwd's parent is unmeasured | `story:claude-memory-canary-deeper-ancestors` | wave |
| Defect: H11 reads `ok` over memory in a surface the canary never probed | `story:h11-not-ok-on-unprobed-memory` | wave, after the canary |
| Defect: an `@` mention in a Claude task reads a file no seam sees | `story:claude-task-text-mentions` | wave, after the two above |

## Not in it, and why

Read as work that is not a defect (coordinator's reading, 2026-11-09): the six per-call governance
stories of `epic:loom-agent-work-through-metaharness`, `story:several-tool-contract-families-per-session`,
`story:inner-workings-modelled-in-ess`, `story:selector-metrics`, `story:llm-route-bindings` and
`story:llm-route-evidence` are capabilities that do not exist yet;
`dependency-blocker:llm-foundation-release` waits on another repository's release.

`release-plan:release-0-31-0` on the unpublished local branch `integration/wave-2026-11-06-metaharness`
(another session, 2026-11-06, "Proposed, not yet approved") names four of the defect stories above
and an ESS pin move. That branch and its tree `mh-wave-2026-11-06` are left as they are; the ESS pin
and the 0.31.0 release are not part of this epic.

## Acceptance

Every open GitHub issue of beyond10x/metaharness on 2026-11-09 and every open defect in this store
has a story under this epic, and each story is implemented, or held by an open typed blocker that
names what clears it. One pull request carries every wave. No release is part of it.
