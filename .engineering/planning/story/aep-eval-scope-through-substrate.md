---
format: aep.planning-md/3
id: story:aep-eval-scope-through-substrate
kind: story
status: implemented
title: The AEP eval takes its delegated scope from Substrate's systemd adapter, not its own systemd-run
summary: 'ADR 0109: drop the eval''s systemd-run re-exec for the confinement contract''s systemd adapter'
tags:
- adr-0109
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: crates/metaharness-aep-eval/Cargo.toml
- confidence: inferred
  path: crates/metaharness-aep-eval/src/lib.rs
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-05T03:32:36Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-05T03:32:36Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-05T05:45:04Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}}
---
## Outcome

`metaharness-aep-eval` gets the delegated cgroup its Substrate probe needs from Substrate's
`systemd` confinement adapter (`substrate_host::confinement`, ADR 0109), not by re-executing
itself under its own `systemd-run --user --scope`. A host where that adapter cannot serve a
scope is refused naming the missing family.

## Why

ADR 0109 decision 6 makes Substrate's confinement contract the only confinement code in its
consumers. The eval still builds `systemd-run --user --scope --quiet --setenv … -- <itself>` and
re-executes (`crates/metaharness-aep-eval/src/lib.rs`, the function around `Command::new("systemd-run")`).
Found while metaharness moved to Harness 0.21.0 on Substrate 0.13.0
(story:b10x-harness-pin-substrate-0-12), and left out of that pin-only hop.

## Acceptance

1. No `systemd-run` invocation is left in `crates/metaharness-aep-eval/src`; the scope comes from
   the `systemd` adapter of the pinned Substrate release.
2. A host without a user manager is refused with the adapter's family name, and a test pins the
   refusal.
3. `task check` passes.
