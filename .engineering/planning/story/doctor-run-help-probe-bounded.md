---
format: aep.planning-md/3
id: story:doctor-run-help-probe-bounded
kind: story
status: active
title: doctor bounds its run --help probe as it bounds --version
summary: 'From U2: doctor.rs run --help runs through .output() with no deadline'
tags:
- from-finding
relations:
- decomposes: epic:loom-agent-work-through-metaharness
scope:
- confidence: cited
  path: crates/metaharness/src/builder.rs
- confidence: cited
  path: crates/metaharness/src/doctor.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-05T01:50:52Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-05T01:50:52Z", actor: "human:timo", revision: 3}
---
## Scheduling

Draft, filed 2026-11-04 from unit U2 of the wave of 2026-11-04 (story:version-probe-everywhere-bounded),
implementor's report, item "not done". Pre-existing; not in that wave.

## Outcome

`metaharness doctor b10x` bounds every child it runs, not only `--version`: the `run --help` probe returns
within a deadline with a named report when the binary hangs.

## Context

- `crates/metaharness/src/doctor.rs` (about line 211 at the wave's base `2dc4d26`) runs `<binary> run --help`
  through `.output()` with no deadline. Read from the code; no hang was observed.
- The bounded runner `vendor_version` (`crates/metaharness/src/builder.rs`, amendment a65) hard-codes
  `--version`, so bounding this probe means a bounded runner that takes any argv.
- What reaches it: every `metaharness doctor b10x`.

## Acceptance

1. A stand-in that answers `--version` and never exits on `run --help` makes `metaharness doctor b10x` return
   within the probe deadline with a report naming the `run --help` probe, where today it blocks.

## Scope

- inferred: `crates/metaharness/src/doctor.rs`, `crates/metaharness/src/builder.rs` (`vendor_version`).
