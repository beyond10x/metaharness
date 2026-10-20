---
format: aep.planning-md/3
id: review-result:adr0096-w3-contract-names-adversary-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:metaharness-tools-adopts-tool-contract: fixes hold, one pre-existing b10x warning'
relations:
- reviews: story:metaharness-tools-adopts-tool-contract
revision: 1
---
unit: story:metaharness-tools-adopts-tool-contract, pass 2 of 2 — uncommitted tree wt-c6ce0a7152d6 (base 34c427b, after correction round 1)
verdict: CONFIRMED
cases: executed 894→896, red 1
origin: introduced 0 / pre-existing 1 / undecided 0
wrote-outside-worktree: 5 paths (metaharness-wave-adr0096/w3/u3-adv2 under the operator's cache directory)
needs-coordinator: P2-1 is outside this unit (the b10x record path), so it needs its own story or an explicit decision

## Cases added (untracked, `crates/metaharness/tests/adversary_contract_names_b10x_pass2.rs`)

Both replay the recorded `b10x-harness 0.15.1` golden through the real b10x adapter.

| case | asserts | now |
|---|---|---|
| `a_contract_name_on_a_b10x_record_read_through_the_b10x_seam_names_no_effect` | `tool_invoke {name: "fs.read"}` on a b10x record leaves `operations` and `subjects` empty | green (F2 holds) |
| `the_file_a_flat_b10x_call_read_is_on_metaharness_record` | the loop's `file_read {"path":"README.md"}`, recorded by the loop with `subjects: ["file:README.md"]`, keeps that subject in metaharness's `tool.requested` | red |

```
assertion `left == right` failed: the loop recorded `subjects: ["file:README.md"]` for this call; metaharness's `tool.requested` for it names none
  left: []
 right: ["file:README.md"]
test result: FAILED. 1 passed; 1 failed
```

Suite with the cases: 895 passed, 1 failed (the red case), 37 ignored.

## Ledger, pass 1 → pass 2

F1 resolved (pass-1 cases green, file unedited; the refusal keys on a translated name plus the field, any
value, before anything runs, `tool_invoke` only); F2 resolved (green through the real b10x adapter; the
pre-existing call sites only gained a surface argument, assertions unchanged). New: P2-1, pre-existing.
Carried 0, new 1, resolved 2.

## Attacked and not broken

Surfaces cannot mix within a run (owned surface is Claude-only, b10x is always HarnessLoop); a prefixed
name on a b10x record is not reachable (no metaharness MCP server attached); refusal-key variants
(nested, renamed, string-encoded arguments) run nothing; `tool_describe` with the field runs nothing; the
bare route refuses `code.write`; a32 point 5's claims match source.

## Findings

```findings
- file: crates/metaharness/src/run.rs
  line: 880
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: a b10x run is read as publishing the three verbs while the loop metaharness launches publishes flat by default, so subjects_of_verb answers empty for every b10x call and the record drops the subject the loop itself reported
```

Agent cost: 218,350 tokens, 37 tool uses, 344 s.
