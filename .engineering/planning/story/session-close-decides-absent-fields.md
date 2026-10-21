---
format: aep.planning-md/3
id: story:session-close-decides-absent-fields
kind: story
status: active
title: The session domain decides terminal records whose optional fields are absent
refs:
- provider: atlas
  reference: adr-0096
relations:
- decomposes: epic:session-format
- informed_by: review-result:adr0096-w4-ess-pin-adversary-1
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T12:00:25Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-21T12:00:25Z", actor: "human:timo", revision: 4}
---
# Story: the session domain decides terminal records whose optional fields are absent

## Scheduling

Wave 5 unit 3 (`approval-record:adr-0096-wave-5`), filed 2026-10-21 from the adversary's first pass on wave 4
unit 1 (`review-result:adr0096-w4-ess-pin-adversary-1`, finding F2, `pre-existing`). The guards are rewritten
with the presence construct ESS 0.67.0 offers (issue 620, released 2026-10-21).

## Outcome

ESS's own model of `metaharness.session.CloseUnsteeredStream` decides every terminal record production
decides, so the specification, not only the Rust target, states how a session ends for records that omit
`stop_reason`, `terminal_reason` or `is_error`.

## Context

- ESS evaluates a comparison with an absent field as unknown; production reads it as false. Over a
  216-shape grid of terminal records, ESS 0.66.0's interpreted model passes 186 and leaves 30 unsupported,
  identically under the guards before and after wave 4 unit 1; ESS 0.56.0 on the base spec is also
  unsupported on the success record.
- The record the repository's own vectors write, `{"emit":"session.ended","is_error":false,"subtype":"success"}`
  (`vectors.rs:33`, `builder.rs:2013`, `scripted.rs:358`), is closed `completed` by production and is
  undecided by the specification (`observed-budget-stop` Unknown).
- Acceptance case already written: `crates/metaharness-aep/tests/adversary_ess_pin_0_66_0.rs::the_specification_decides_the_success_record_the_repository_writes`,
  ignored with this story's name until it lands.
- Wave 4 unit 1 moved production's handling of an absent `is_error` into a Rust test beside
  `reason_from_record` (`crates/metaharness/src/run.rs:1210-1237`).
- The adversary's probes found no guard written with `defined()`/`missing()` that clears `ESS-SYNTH-022` at
  0.66.0 (two forms tried); this may need an ESS change (presence-aware comparisons), to be asked of the ESS
  owners through `b10x-chef-orchestrator`.

## Acceptance

ESS's interpreted model decides the success record and every terminal-record shape production decides
(the 216-shape grid, or the subset production handles, with the rest named), with the guards rewritten
using whatever presence construct the pinned ESS offers; the ignored acceptance case runs and passes; the
regenerated suite stays green on the Rust target; synthesis has 0 refusals.

## Scope

- inferred: `spec/domains/session.yaml`, `spec/scenarios/`, `spec/conformance.json`,
  `crates/metaharness-aep/src/drive/ess_conformance.rs`, `crates/metaharness-aep/tests/adversary_ess_pin_0_66_0.rs`,
  `spec/coverage.md`, `CHANGELOG.md`.
