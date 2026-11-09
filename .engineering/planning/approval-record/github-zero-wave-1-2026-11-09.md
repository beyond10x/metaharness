---
format: aep.planning-md/3
id: approval-record:github-zero-wave-1-2026-11-09
kind: approval-record
status: approved
title: Wave 1 of github-zero proposed and run with no operator present
tags:
- non-interactive
relations:
- decides: story:eval-cgroup-root-made-absolute
- decides: story:claude-memory-canary-deeper-ancestors
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-11-09T22:08:47Z", actor: "human:timo", revision: 2}
---
## Which stop

The stage-1 proposal of wave 1 of 4 of `release-plan:wave-github-zero-2026-11-09`.

## Why non-interactive

No operator turn until the goal is met (Stop hook). Goal, verbatim: "zero remaining issues on
github, all bugs fixed from planning store". The brief carrying it states the goal is the approval
of each wave.

## What was decided in the operator's absence

Wave 1 runs: `story:eval-cgroup-root-made-absolute` (unit `mh-gh-zero-u1`) and
`story:claude-memory-canary-deeper-ancestors` (unit `mh-gh-zero-u2`), one at a time, each with one
`aep:implementor` and one `aep:adversary` on model opus. Check path: `aep plan artifact waves`
(aep 0.79.0) placed both in wave 1; they collide on nothing. Scope: the eval story is cited, the
canary story inferred. Pre-flight: 71G free on `/`, load about 38; every cargo call through the
shared build slot (3 slots, 20G floor). The native lane uses the three pinned Claude binaries,
digest-checked 2026-11-09, from the coordinator's scratch, in a loopback-only network namespace,
with no credential and no paid call.

Commits this bypass authorises: per unit its commits, the adversary's test commit and the merge into
`epic/github-zero`; the wave's closing store commit. No push except the one pull request at the end
of the plan; no tag, no release.

## What the operator would have been asked

Whether to run wave 1 as proposed.
