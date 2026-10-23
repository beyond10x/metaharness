---
format: aep.planning-md/3
id: story:aep-pin-0-77-0
kind: story
status: implemented
title: metaharness aep drive links AEP 0.77.0 and renders a step's declared commands
summary: 'AEP 0.77.0 adds per-step commands: to llm steps; the vendor prompt told implement ''not cargo'' while development/checks admits cargo test'
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-23T18:46:47Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1,"approval":1,"verification":1}}}
- {from: "proposed", to: "active", at: "2026-10-23T18:46:47Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"approval":1,"verification":1}}}
- {from: "active", to: "implemented", at: "2026-10-23T18:46:48Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1,"approval":1,"verification":1}}}
---
# Story: aep-pin-0-77-0

## Outcome

A driven step under AEP 0.77.0's `development/checks` map is told the commands its map declares, so
`implement` runs `cargo test` and `task check` instead of being told not to run `cargo`.

## Context

AEP 0.77.0 (`23294974`, https://github.com/beyond10x/aep/releases/tag/0.77.0) adds `commands:` to
`llm` steps (`aep_driver_spec::map::CommandPrefix`), carried to the executor as
`StepContext::commands`. `driven_surface` admits the declared prefixes and the state's readers, and
refuses `cargo --config` and `rg --pre`. The AEP session reported that metaharness's static vendor
prompt line "Not `cargo`" is wrong for `implement` under `development/checks`.
`b10x-chef-orchestrator` approved option A on 2026-10-23: bump, render the per-step commands,
correct the line, and release. Design: amendment a56.

## Acceptance

- The eight AEP crates link `23294974a72bbe7e8651588ec3bfa69e1cb64145`, and `task check` exits 0.
- `a_declared_step_surface_is_rendered_from_the_map_and_admitted_by_the_driver` passes. It reads the
  0.77.0 `development/checks` fixture. Every prefix the vendor prompt names for `implement` is
  admitted by `driven_surface`, the prompt names `cargo --config` and the call refuses it, the prompt
  no longer says "not `cargo`", and the b10x renderings name `{driver} plan artifact …` for `specify`
  and name the `cargo` prefixes as not startable for `implement`.
- A step with no declaration keeps the existing paragraph.

## Out of Scope

- Adding declared programs to the b10x arm's `--allow-program` list. It widens the observed arm's
  sandbox and is its own change under § Safety envelope.
- The evidence-producer gap in `development/checks`, which is AEP's (`story:checks-map-evidence-producers`
  in AEP's store).

## Ambiguities

None.

## Open Questions

None.
