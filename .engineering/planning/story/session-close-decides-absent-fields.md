---
format: aep.planning-md/3
id: story:session-close-decides-absent-fields
kind: story
status: implemented
title: The session domain decides terminal records whose optional fields are absent
refs:
- provider: atlas
  reference: adr-0096
relations:
- decomposes: epic:session-format
- informed_by: review-result:adr0096-w4-ess-pin-adversary-1
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T12:00:25Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-21T12:00:25Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-21T18:43:46Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"ess_conformance_coverage_v1":1}}}
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

## Delivered

Commit `ae0b45f`, merged `bc3f398` into `integration/adr-0096-wave-5`; one `aep:adversary` pass
(`review-result:adr0096-w5-session-close-adversary-1`, nothing found, three cases kept in
`crates/metaharness-aep/tests/adversary_session_close_absent_fields.rs`), coordinator gate 82 test
binaries, 1017 passed, 0 failed, 48 ignored, conformance 103/103. The guards of
`metaharness.session.CloseUnsteeredStream` use `coalesce(x, d) == v` (ESS 0.67.0, issue 620), so ESS's
interpreted model reads an absent `stop_reason`, `terminal_reason` or `is_error` the way
`reason_from_record` does; `spec/system.yaml` moves from `ess/15` to `ess/29`. The `none:` budget clause on
`positive-terminal-record` is dropped. A grid test in `ess_conformance.rs` runs all 216 terminal-record
shapes through production's close and ESS's model and requires the same outcome for each; the acceptance
case `the_specification_decides_the_success_record_the_repository_writes` is no longer ignored. No
amendment was needed (a38 unused). The coordinator applied one doc-comment patch to `run.rs`.

Whole gate at the wave-6 head: task check, one step at a time, on `integration/adr-0096-wave-6` at
`5ddebbd`: 6 steps exit 0, 96 test binaries, 1128 passed, 0 failed, 49 ignored, conformance 103/103 on ESS
0.67.0.

Deviations: none in scope. The independent review notes the adversary file's last case has no positive
control and ignores the `ess` exit status (`story:adversary-tests-have-positive-controls`, draft), and that
the comment at `spec/domains/session.yaml:144-159` still says an absent `is_error` is "neither verdict"
(open note, no story).
