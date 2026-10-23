---
format: aep.planning-md/3
id: story:aep-pin-0-76-0
kind: story
status: draft
title: metaharness aep drive reads AEP 0.76.0 driver-steps/2 maps
summary: 0.14.0 pins AEP 2481b483 (0.68.1) and cannot drive a store adopted with AEP 0.76.0; shipped maps need a b10x endpoint or --allow-evidence-gap
revision: 1
---
## Problem

Reported by session `ess-ui-templates` on 2026-10-23 (verbatim, abridged to its two findings):

1. "metaharness 0.14.0 pins AEP 2481b483 (0.68.1, crates/metaharness-aep/Cargo.toml:14). Its aep reads
   aep.driver-steps/1 only; the 0.76.0 protocol tree (4d22c903) ships driver-steps/2 maps with
   `judgement:`/`expect:` fields → `aep doctor`: "unknown field `judgement`" on
   drivers/development/{default,checks}.yaml, "[unsupported_format_version]" on drivers/releases/source.yaml."
   Workaround there: `project.yaml` `protocols:` pinned to 2481b483.
2. "Under 2481b483: development/default needs --b10x-endpoint (6 llm steps harness b10x);
   development/checks pre-flight refuses: contract_result, property_test_result, verification,
   specification "no step of the map declares it", blocks adversarial_verify -> review."
   Workaround there: the checks map with `--allow-evidence-gap`.

Repro (theirs): `~/.local/state/worktree/trees/b10x/ess-ui-templates/esui-aep-adopt`, branch
`plan/adopt-aep` at `02faeb7`.

Verified here: `crates/metaharness-aep/Cargo.toml:14-17` pin `2481b48395afbaea9ad9b6165dbc0a80b80e1c01`;
AEP's newest release is 0.76.0 (2026-10-21, `gh release list --repo beyond10x/aep`). Finding 2 is not
re-run here.

## Acceptance

- `metaharness aep drive run` drives a store adopted with AEP 0.76.0 whose protocol tree ships
  `aep.driver-steps/2` maps, without a `protocols:` pin to an older tree.
- Whether `development/checks` should complete without `--allow-evidence-gap` under a map with no b10x
  endpoint is decided (AEP's maps or metaharness's pre-flight) and recorded.

Not in 0.15.0 (the Claude 2.1.309 / Codex 0.160.0 pin release); a dependency upgrade under its own go.
