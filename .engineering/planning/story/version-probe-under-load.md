---
format: aep.planning-md/3
id: story:version-probe-under-load
kind: story
status: implemented
title: The --version probe reads a binary that answers slowly under load
summary: 'Takt 0.17.0 trial F4: the 5 s VERSION_PROBE_DEADLINE refused pinned binaries at load 33-63; raise it to 60 s'
scope:
- confidence: cited
  path: crates/metaharness/src/builder.rs
- confidence: cited
  path: crates/metaharness/tests/adversary_version_probe_pass1.rs
- confidence: cited
  path: crates/metaharness/tests/qualify_adv1.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-28T21:16:55Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-28T21:16:55Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-28T22:30:52Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}}
---
## Outcome

A vendor binary's `--version` answer that takes longer than five seconds because the machine is
loaded is read, not refused: `--strict-version`, the Codex launch and `qualify` accept a binary that
answers within the new deadline, and a binary that never answers is still killed and refused by name.

## Context

- Finding F4 of the takt 0.17.0 trial: the `--version` probe at
  `crates/metaharness/src/builder.rs:2429` (`VERSION_PROBE_DEADLINE`, 5 s, amendments a33 and a34)
  timed out at load average 33 to 63, so a step refused a pinned binary that was only slow.
- Callers of `vendor_version` with that deadline: `builder.rs:2560` (strict-version and the Codex
  launch), `crates/metaharness/src/qualify.rs:112`.
- Claude Code's `--version` starts a Node runtime; under load its start time is not bounded by the
  binary.

## Decision

- Raise the deadline to 60 s. The probe stays on the launch path because `--strict-version` must read
  the version of the binary it is about to start; a cache keyed on a path would read a binary that may
  have been replaced. A binary that does not answer in 60 s is still killed, reaped and refused.

## Acceptance

1. A stub vendor binary that sleeps 7 s and then prints a pinned version is reported as
   `VersionProbe::Reported`, and a `--strict-version` start on it is not refused, where today it is
   `TimedOut` after 5 s.
2. A stub that never exits is still killed and refused by name within the deadline plus 5 s
   (the existing test, at its deadline).

## Out of Scope

- A configurable deadline.

## Scope

- `crates/metaharness/src/builder.rs` (`VERSION_PROBE_DEADLINE`, its tests), `crates/metaharness/src/qualify.rs`
  if it names the value, `docs/design/metaharness-protocol-v0.1.md` (amendment), `CHANGELOG.md`.
