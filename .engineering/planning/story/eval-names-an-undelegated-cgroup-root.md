---
format: aep.planning-md/3
id: story:eval-names-an-undelegated-cgroup-root
kind: story
status: active
title: aep-eval names a cgroup root that holds it but is not delegated
summary: A non-delegated root that holds the eval fails later as a missing run tool
tags:
- from-finding
relations:
- derived_from: review-result:wave-0300-u1-adversary-1
- decomposes: epic:github-zero-2026-11-09
- depends_on: story:eval-cgroup-root-made-absolute
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: crates/metaharness-aep-eval/src/lib.rs
- confidence: inferred
  path: crates/metaharness-aep-eval/tests/adversary_u1_cgroup_root.rs
- confidence: inferred
  path: crates/metaharness-aep-eval/tests/adversary_w8u7.rs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T22:08:15Z", actor: "human:timo", revision: 8}
- {from: "proposed", to: "active", at: "2026-11-09T22:08:15Z", actor: "human:timo", revision: 9}
---
## Scheduling

Draft, filed 2026-11-05 from the out-of-scope finding of `review-result:wave-0300-u1-adversary-1`. The finding
predates the 0.30.0 wave and is not part of it.

## Outcome

When the cgroup root holds the eval's process but is not delegated, `aep-eval` refuses before the run and
names the root. It no longer fails later as "no `run` tool".

## Context

- `crates/metaharness-aep-eval/src/lib.rs:1330`: the eval copies only the prefix step of Substrate's
  `probe_cgroup`.
- With a root such as `user-1000.slice`, which holds the eval but is not delegated, the eval skips the scope.
  The harness probe then refuses confinement.
- Inferred from reading. Not driven.

## Acceptance

- expected_before: preflight with a non-delegated root that holds the eval fails with a missing `run` tool.
- After: it fails before the harness starts, naming the root and the missing delegation.

## Scope

Derived 2026-11-09 by `aep:story-scoper` at `bff6a66`; confidence medium.

- cited: `crates/metaharness-aep-eval/src/lib.rs:1330` in `ensure_cgroup_scope` (`:1323-1345`),
  `below_cgroup_root` (`:1348-1356`), `cgroup_below_root` (`:1369-1382`, the copied prefix step).
- cited: Substrate's private `probe_cgroup` (`substrate-host/src/probe.rs`); the steps the eval does
  not copy are a writable `cgroup.procs`, `cpu memory pids` in `cgroup.controllers`, an empty
  `cgroup.procs`, `cgroup.subtree_control` and a probe child cgroup.
- inferred: `crates/metaharness-aep-eval/tests/adversary_w8u7.rs:439-440` passes `/sys/fs/cgroup`
  and expects the preflight to pass; `tests/adversary_u1_cgroup_root.rs:105-123` passes the parent of
  the test's cgroup and expects a later failure. Both may change with the new refusal.
- inferred: `CHANGELOG.md`, `AGENTS.md:244-250`.
- inferred risk: the probe's later steps write to the root; a full copy in the eval would mutate it.
- Not decided anywhere: which probe steps count as "delegated" for the refusal.
- Runs after `story:eval-cgroup-root-made-absolute` (shared file).
