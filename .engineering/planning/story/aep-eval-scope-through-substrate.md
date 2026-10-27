---
format: aep.planning-md/3
id: story:aep-eval-scope-through-substrate
kind: story
status: draft
title: The AEP eval takes its delegated scope from Substrate's systemd adapter, not its own systemd-run
summary: 'ADR 0109: drop the eval''s systemd-run re-exec for the confinement contract''s systemd adapter'
tags:
- adr-0109
revision: 2
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
