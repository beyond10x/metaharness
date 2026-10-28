---
format: aep.planning-md/3
id: story:version-probe-everywhere-bounded
kind: story
status: draft
title: doctor and the b10x launch read --version under the probe deadline
summary: 'Pre-existing finding of D170 adversary pass 1: two --version reads have no deadline'
revision: 1
---
## Outcome

Every `--version` metaharness runs is bounded: `doctor` and the b10x launch read the version through
the same deadline and drain grace as `--strict-version` (design amendment a65), so a binary that
hangs or a leftover process holding stdout cannot block them without bound.

## Context

- Adversary pass 1 on story:version-probe-under-load (review-result:d170-u2-adversary-1, finding 4,
  pre-existing): `doctor` (`crates/metaharness/src/doctor.rs:99`) and the b10x launch's
  `b10x_version` (`crates/metaharness/src/builder.rs:862`) run `--version` through `.output()`
  with no deadline.
- What reaches it: `metaharness doctor` and every `metaharness run b10x --strict-version`. No hang was
  observed; the adversary read it from the code.

## Acceptance

1. A stub that never exits makes `doctor` and a strict b10x launch return within the deadline with a
   named refusal or report, where today they block.

## Scope

- `crates/metaharness/src/doctor.rs`, `crates/metaharness/src/builder.rs` (`b10x_version`).
