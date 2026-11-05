---
format: aep.planning-md/3
id: story:eval-names-an-undelegated-cgroup-root
kind: story
status: draft
title: aep-eval names a cgroup root that holds it but is not delegated
summary: A non-delegated root that holds the eval fails later as a missing run tool
tags:
- from-finding
relations:
- derived_from: review-result:wave-0300-u1-adversary-1
revision: 1
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
