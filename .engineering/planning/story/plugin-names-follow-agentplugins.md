---
format: aep.planning-md/1
id: story:plugin-names-follow-agentplugins
kind: story
status: implemented
title: This repository names the renamed plugins
summary: Rename aep-planning, adp and ess-schema references to aep-plan, aep-drive and ess-specify in the live fixture path, authored expectations and checks; recorded runs and dated prose stay.
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/metaharness-aep-eval/src/lib.rs
- confidence: cited
  path: crates/metaharness-claude
- confidence: cited
  path: crates/metaharness/tests/audit.rs
- confidence: cited
  path: evals/aep/README.md
- confidence: cited
  path: evals/aep/checks/contracts/trace-expectations.txt
- confidence: cited
  path: evals/aep/checks/lib.sh
- confidence: cited
  path: evals/aep/expectations.driven-step.trace.yaml
- confidence: cited
  path: evals/aep/expectations.projection.trace.yaml
- confidence: cited
  path: evals/aep/expectations.trace.yaml
- confidence: cited
  path: evals/aep/prompt.md
revision: 5
---
# Story: This repository names the renamed plugins

## Context

`agentplugins` renames `aep-planning` to `aep-plan`, `adp` to `aep-drive` and `ess-schema` to
`ess-specify` (skill `specify`); agent ids follow (`aep-plan:decomposer`,
`aep-plan:plan-reviewer`, `aep-drive:implementor`, …). Sibling record: agentplugins
`epic:plugins-named-by-product-and-verb`. Reference sites here (`rg` on 2026-09-03, `!target
!CHANGELOG.md`):

| site | hits | class |
|---|---|---|
| `crates/metaharness-aep-eval/src/lib.rs:694` | 1 | **live code**: copies `plugins/aep-planning` into the fixture |
| `crates/metaharness-claude/tests/marketplace_plugin.rs`, `src/marketplace.rs`, `src/vectors.rs` | 23 | marketplace-format fixtures naming `beyond10x/agentplugins@aep-planning@0.4.0` and its install path |
| `crates/metaharness/tests/audit.rs` | 4 | test data |
| `evals/aep/expectations*.trace.yaml`, `evals/aep/checks/lib.sh`, `checks/contracts/trace-expectations.txt`, `prompt.md`, `README.md` | 17 | authored expectations and instructions |
| `evals/aep/runs/*`, `evals/aep/checks/transcripts/*.jsonl`, `runs/side-by-side.html` | 24 | recorded runs: evidence, not rewritten |
| `docs/research/*.md`, `docs/design/*.md` | 4 | dated prose: not rewritten |

## Acceptance

`task check` (fmt, clippy, test) exits 0; the live path at `lib.rs:694` and every authored
expectation, check and instruction name the new ids; recorded runs and dated prose are unchanged;
a marketplace fixture that pins a released version (`@0.4.0`) may keep the name that version
shipped under only if the test asserts parsing rather than a current install, and the report says
which fixtures were kept for that reason.

## Notes

Cross-repository dependency on the agentplugins rename commit, recorded here in prose. The
`agentplugins` story keeps five golden-path expectation rows under the recorded names for the
same reason as the recorded runs here.
