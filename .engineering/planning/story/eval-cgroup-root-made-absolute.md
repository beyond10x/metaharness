---
format: aep.planning-md/3
id: story:eval-cgroup-root-made-absolute
kind: story
status: active
title: aep-eval makes the cgroup root absolute once
summary: A relative --cgroup-root is read from two working directories in the driven arm
tags:
- from-finding
relations:
- derived_from: review-result:wave-0300-u1-adversary-1
- decomposes: epic:github-zero-2026-11-09
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: crates/metaharness-aep-eval/src/lib.rs
- confidence: inferred
  path: crates/metaharness-aep-eval/tests/cgroup_root_absolute.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T22:08:15Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-09T22:08:15Z", actor: "human:timo", revision: 7}
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

## Scope

Derived 2026-11-09 by `aep:story-scoper` at `bff6a66`; confidence high.

- cited: `crates/metaharness-aep-eval/src/lib.rs:378-382` (`resolve` takes the raw `--cgroup-root`),
  `:280-281` (the driven arm passes it as `--b10x-cgroup-root`); `canonical_existing` (`:396-399`) is
  what `resolve` uses for every other path.
- cited, read only: `crates/metaharness/src/spawn.rs:340` (`current_dir(plan.cwd)`).
- inferred: a new `crates/metaharness-aep-eval/tests/cgroup_root_absolute.rs`; `CHANGELOG.md`.
- inferred risk: `resolve` also runs for `driven --arm claude` (`lib.rs:194`), which never reads the
  root, so a refusal in `resolve` would newly refuse that arm where the default root does not exist.
- Stays out of `ensure_cgroup_scope` (`:1323-1345`), which is
  `story:eval-names-an-undelegated-cgroup-root`'s; that story runs after this one.
