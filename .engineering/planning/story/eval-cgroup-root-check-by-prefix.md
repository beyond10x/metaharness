---
format: aep.planning-md/3
id: story:eval-cgroup-root-check-by-prefix
kind: story
status: implemented
title: The AEP eval judges below-the-cgroup-root by canonical prefix, as Substrate's probe does
summary: 'From the U4 adversary pass of the 0.29.0 wave: below_cgroup_root is a substring match'
tags:
- from-finding
relations:
- derived_from: review-result:wave-0290-u4-adversary-1
scope:
- confidence: inferred
  path: crates/metaharness-aep-eval/src/lib.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-05T11:09:21Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-05T11:09:21Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-05T11:56:19Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Scheduling

Draft, filed 2026-11-05 from `review-result:wave-0290-u4-adversary-1`, from its pre-existing finding. Not part
of the 0.29.0 wave.

## Outcome

The AEP eval decides "below the cgroup root" the way Substrate's probe does: by canonical path prefix.
So the eval never runs a confined arm that the probe would then report as having no exec facts, and never
refuses one the probe would accept.

## Context

- `below_cgroup_root` in `crates/metaharness-aep-eval/src/lib.rs` (about line 1358 at `75594a5`) looks for
  the root's path anywhere in `/proc/self/cgroup`. Substrate's `probe_cgroup` requires a canonical prefix.
- A root of `/sys/fs/cgroup`, a path with `..`, or a symlink makes the two disagree. Nothing found reaches
  that outside the `adversary_w8u7` tests. Inferred from reading.

## Acceptance

- expected_before: a root given as a path with `..` or through a symlink is judged by substring.
- After: the eval canonicalises the root and the process's cgroup path and compares by component prefix. A
  test covers `..`, a symlink and a sibling whose name starts with the root's name.
