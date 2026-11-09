---
format: aep.planning-md/3
id: approval-record:github-zero-acceptance-2026-11-09
kind: approval-record
status: approved
title: 'Epic github-zero: stories accepted with no operator present'
tags:
- non-interactive
relations:
- decides: epic:github-zero-2026-11-09
- decides: story:eval-cgroup-root-made-absolute
- decides: story:eval-names-an-undelegated-cgroup-root
- decides: story:claude-memory-canary-deeper-ancestors
- decides: story:claude-task-text-mentions
- decides: story:loom-run-adapter
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-11-09T22:08:15Z", actor: "human:timo", revision: 2}
---
## Which stop

Acceptance of the stories under `epic:github-zero-2026-11-09`, and of the epic itself. Accepting a
story is the operator's act (AEP implementing, wave.md); this run has no operator turn.

## Why non-interactive

The operator started the run on 2026-11-09 with a goal that a Stop hook enforces and no operator turn
until it is met. The goal, verbatim: "zero remaining issues on github, all bugs fixed from planning
store". Method, verbatim: "use /planning and /wave and /worktree:managing-worktrees - but in the end
provide 1 single PR to gh. make sure all are below 1 big epic for integration via bottoken from
./atlas". The brief that carries it (rule 3) states that the goal is the acceptance of these stories
and the approval of each wave, and asks that each bypassed stop be recorded here.

## What was decided in the operator's absence

- `epic:github-zero-2026-11-09` moves draft -> proposed -> active.
- Accepted (moved to `active`) for the epic's wave plan: `story:eval-cgroup-root-made-absolute`,
  `story:eval-names-an-undelegated-cgroup-root`, `story:claude-memory-canary-deeper-ancestors`,
  `story:claude-task-text-mentions` and `story:loom-run-adapter` (issue 28's adapter half).
  `story:h11-not-ok-on-unprobed-memory` was already `active` (accepted 2026-11-05).
- The three Claude stories need the native lane's pinned binaries. All three were obtained and
  digest-checked on 2026-11-09 against the research pages: 2.1.304 downloaded from the vendor's
  release host, SHA-256 `ea0f1357…7f912`; 2.1.305 `86cb722a…656ef`; 2.1.309 `4d704a8a…9b658`.
- `story:loom-native-default` (issue 28's "native default" half) is NOT accepted. It stays draft,
  held by `decision-blocker:loom-native-default`, which nobody answered; issue 28 stays open and the
  pull request says `Refs #28`.

## What the operator would have been asked

Whether to accept these five stories into the epic's waves, and whether the choices the Claude
stories' acceptance leaves open (for the mentions story: refuse, neutralise or a vendor switch)
may be taken by the coordinator and recorded in the design amendment.
