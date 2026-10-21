---
format: aep.planning-md/3
id: review-result:adr0096-w5-session-close-adversary-1
kind: review-result
status: active
title: Adversary, wave 5 unit 3 (session close decides absent fields), pass 1
owner: metaharness
refs:
- provider: atlas
  reference: adr-0096
relations:
- reviews: story:session-close-decides-absent-fields
revision: 1
---
unit: story:session-close-decides-absent-fields, pass 1 — uncommitted tree wt-f135aa0bd1ba (wave 5 unit 3, base 4a72d8c)
verdict: nothing found
cases: executed 1014→1017, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 9 paths (metaharness-wave-adr0096/w5/u3-adv1 under the operator's cache directory)
needs-coordinator: none

## Cases added (kept, `crates/metaharness-aep/tests/adversary_session_close_absent_fields.rs`)

| case | asserts | result |
|---|---|---|
| `every_shape_with_absent_keys_omitted_is_decided_as_production_closes_it` | with absent keys left out (as a vendor record does), not sent as `null`, ESS decides all 216 shapes as production closes them | green |
| `a_wrong_expectation_is_never_passed_for_any_shape` | each shape sent with a wrong outcome is passed by none | green |
| `exchanging_the_two_accepting_branches_is_caught` | a spec copy with `observed-budget-stop` and `positive-terminal-record` swapped answers differently from the real spec for `is_error: false` with `stop_reason: budget-exhausted` | green |

Suite: 82 test binaries, 1017 passed, 0 failed, 48 ignored.

## Attacked and not broken

Grid coverage against how production reads each field (`scripted.rs:477-480`); the expected outcomes come
from production's own close (`Run::reason_from_record`), sharing no code with ESS; the `coalesce` defaults
match `run.rs:1240-1267` in every combination; dropping or swapping an accepting branch is refused by `ess
specify validate` (`ESS-AUTHOR-041`, exit 1); the format bump to `ess/29` matches CI's ESS pin (0.67.0); a
re-run mutation audit: 104 mutants, 103 killed, 0 survived, 1 inconclusive, all 14 `CloseUnsteeredStream`
mutants killed.

## Disposition (coordinator)

- Kept as adversary tests: the three cases above.
- Nothing to fix. The independent review later found the second case has no positive control (s10-2), now
  `story:adversary-tests-have-positive-controls`.

## Findings

```findings
[]
```
