---
format: aep.planning-md/3
id: story:eval-cgroup-root-made-absolute
kind: story
status: draft
title: aep-eval makes the cgroup root absolute once
summary: A relative --cgroup-root is read from two working directories in the driven arm
tags:
- from-finding
relations:
- derived_from: review-result:wave-0300-u1-adversary-1
revision: 1
---
## Scheduling

Draft, filed 2026-11-05 from `review-result:wave-0300-u1-adversary-1`. The finding predates the 0.30.0 wave
and is not part of it.

## Outcome

`aep-eval` resolves `--cgroup-root` to an absolute, canonical path once, in `resolve`, so the eval and the
harness it starts check the same directory.

## Context

- `crates/metaharness-aep-eval/src/lib.rs:378` and `:280`: `resolve` passes the raw root on.
- In the paid driven arm, the harness starts with `current_dir(plan.cwd)`
  (`crates/metaharness/src/spawn.rs:340`). A relative root is therefore read from two different working
  directories.
- In preflight both share a working directory, and a relative root passed there.
- Nobody is shown passing a relative root. Inferred from reading; the paid path was not run.

## Acceptance

- expected_before: a relative `--cgroup-root` reaches the harness argv unchanged.
- After: the argv carries the canonical absolute root, or the eval refuses a root that does not resolve,
  naming it.
