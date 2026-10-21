---
format: aep.planning-md/3
id: review-result:adr0096-w4-ess-pin-adversary-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:ess-pin-0-66-0: 2 warnings, 3 notes, two red cases'
relations:
- reviews: story:ess-pin-0-66-0
revision: 1
---
unit: story:ess-pin-0-66-0, pass 1 — uncommitted tree wt-402b59d5b2a3 (impl/ess-pin-0-66-0, base 8c553e1)
verdict: CONFIRMED, highest severity warning, no blocker
cases: executed 895→897, red 2
origin: introduced 2 / pre-existing 3 / undecided 0
wrote-outside-worktree: 1 directory (metaharness-wave-adr0096/w4/u1-adv1 under the operator's cache directory)
needs-coordinator: which fix to apply for F1, and whether F2 gets its own story (cut once by HTTP 429, resumed)

## Cases added (untracked, `crates/metaharness-aep/tests/adversary_ess_pin_0_66_0.rs`)

- `every_authored_scenario_is_decided_by_the_specifications_reference_model`: runs ESS's own model
  (`ess verify conform run --target interpreted`) over the committed suite and asserts each authored
  scenario passed. Red: `close-unsteered-absent-verdict-is-error` and `close-unsteered-absent-verdict-legacy-success`
  are `unsupported` ("the guard of `…/positive-terminal-record` is Unknown over this input").
- `the_specification_decides_the_success_record_the_repository_writes`: production closes
  `{"emit":"session.ended","is_error":false,"subtype":"success"}` as `completed` (passes); the same input as a
  scenario through ESS's model is `unsupported` (`observed-budget-stop` Unknown). Red.

Suite with the cases: 74 binaries, 895 passed, 2 failed (this file), 38 ignored; the production conformance
report passed, 91/91, 0 unsupported.

## Attacked and not broken

The session guard change is equivalent under ESS's logic (same status per scenario on a 216-shape grid; no
input ESS decides disagrees with production `run.rs:1226-1236`); under a plain reading old and new differ
only on an absent `is_error` with subtype `success` (F1). `valid-finite-terms`: `cap > 0` follows from the
other two for required Integers; all `CheckFiniteTerms` mutants killed; production `drive.rs:1711-1723`
agrees. The 85 other scenarios are JSON-identical to base; the two changed ones are explained. Suite
`ess-conformance/35`, `synthesize --check` exit 0, both manifests validate strict. CI pin: tag 0.66.0 → tag
object `55d301e9` → commit `145a1b9d`, `crates/edge/ess-cli` present with `Cargo.lock`. Remaining `0.56`
mentions are history. Mutation audit (guard-negate, connective, boundary, `--scenarios spec`, ESS's model):
93 mutants, 91 killed, 0 survived, 2 inconclusive. Upstream observation for ESS: 0.66.0 refused the third
`ESS-SYNTH-022` even though an authored input satisfies its formula, against its own help text.

## Findings

```findings
- file: crates/metaharness-aep/tests/adversary_ess_pin_0_66_0.rs
  line: 89
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: two authored scenarios assert CloseUnsteeredStream outcomes on an absent is_error that the specification's guard does not decide (ESS reference model unsupported), and legacy-success asserts positive-terminal-record where its guard is_error == false holds under no logic
- file: crates/metaharness-aep/tests/adversary_ess_pin_0_66_0.rs
  line: 156
  category: boundary
  severity: warning
  verdict: INFEASIBLE
  origin: pre-existing
  message: the specification does not decide the success record the repository's vectors write (is_error false, subtype success, no stop or terminal reason), nor 30 of 216 terminal-record shapes, identically under the old and new guards
- file: spec/domains/session.yaml
  line: 154
  category: judgement
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the source-mapping comment still names the legacy subtype=success rule beside a guard that no longer carries it, and does not say the rule now lives only in authored scenarios
- file: spec/domains/session.yaml
  line: 154
  category: judgement
  severity: note
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: the comment cites run.rs:1140-1164, where reason_from_record actually sits at run.rs:1210-1237
- file: spec/domains/session.yaml
  line: 146
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: ESS's guard mutation audit leaves the or-to-and mutant of positive-terminal-record's none clause inconclusive because no scenario sends is_error false with a budget word
```

Agent cost: 187,893 tokens, 31 tool uses, 400 s on the final resume.
