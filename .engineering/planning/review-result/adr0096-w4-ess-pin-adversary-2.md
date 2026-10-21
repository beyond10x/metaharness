---
format: aep.planning-md/3
id: review-result:adr0096-w4-ess-pin-adversary-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:ess-pin-0-66-0: two wording notes, F2 carried to its story'
relations:
- reviews: story:ess-pin-0-66-0
revision: 1
---
unit: story:ess-pin-0-66-0, pass 2 (the last) — correction round 1, uncommitted tree wt-402b59d5b2a3 over base 8c553e1
verdict: NEEDS-CHANGE on two introduced notes; no red case; F2 carried as INFEASIBLE warning, pre-existing
cases: executed 898→898, red 0
origin: introduced 2 / pre-existing 1 / undecided 0
wrote-outside-worktree: 1 directory (metaharness-wave-adr0096/w4/u1-adv2 under the operator's cache directory)
needs-coordinator: none

## Ledger, pass 1 → pass 2

| finding | status | evidence |
|---|---|---|
| F1 authored scenarios the spec does not decide | resolved | no `is_error: null` input remains; four planted production mutants (absent verdict completes; legacy success removed; budget read after verdicts; any non-true verdict succeeds) each caught by the two new `run.rs` tests |
| F2 the spec does not decide the success record | carried | `adversary_ess_pin_0_66_0.rs:175`, ignored, naming `story:session-close-decides-absent-fields`; still red with `--ignored` |
| F3 stale legacy-rule comment | resolved | `session.yaml:142-146`, `:159-163` |
| F4 wrong citation | resolved | `run.rs:1210-1237` |
| F5 budget precedence not held | resolved | the new scenario passes in ESS's model; audit 93 / 91 killed / 1 survived / 1 inconclusive |

Carried 1, new 2, resolved 4. Suite: 74 binaries, 898 passed, 0 failed, 39 ignored; conformance 90/90.

## Equivalent-mutant argument

Holds in the current declaration order: across six spec variants the three authored scenarios pass
(current, mutant, clause dropped, branch declared first); with the branch declared first plus the mutant
or with the clause dropped, validation refuses the budget-word scenario (`ESS-AUTHOR-041`). So the clause
matters only if the order changes, and the new scenario catches that. `--class precedence-swap`: 36
mutants, 35 equivalent, 1 survived (the swap of `observed-budget-stop` and `positive-terminal-record`).

## Findings

```findings
- file: crates/metaharness-aep/tests/adversary_ess_pin_0_66_0.rs
  line: 175
  category: boundary
  severity: warning
  verdict: INFEASIBLE
  origin: pre-existing
  message: carried from pass 1 (was line 156), ignored pending story:session-close-decides-absent-fields; ESS's model of the spec still reports the repository's success record unsupported because the observed-budget-stop guard is Unknown over absent stop_reason and terminal_reason
- file: spec/coverage.md
  line: 377
  category: mutant
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the equivalent-mutant rationale says the or-to-and mutant changes positive-terminal-record only on budget-word records, but under ESS's three-valued logic it also flips the success record from Unknown to completed, hidden only by the declaration order and by F2
- file: crates/metaharness/src/run.rs
  line: 1702
  category: judgement
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the test doc says an absent-is_error record closes error whatever stop_reason or terminal_reason say, but a budget word in either closes budget, and session.yaml:144 leaves budget out the same way
```

Agent cost: 240,976 tokens, 37 tool uses, 429 s.
