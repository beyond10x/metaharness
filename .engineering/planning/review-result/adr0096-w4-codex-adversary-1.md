---
format: aep.planning-md/3
id: review-result:adr0096-w4-codex-adversary-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:codex-strict-record-rows: 4 warnings (infeasible on the pin), 2 notes, three red cases'
relations:
- reviews: story:codex-strict-record-rows
revision: 1
---
unit: story:codex-strict-record-rows, pass 1 — uncommitted tree of impl/codex-strict-record-rows on cfad31d (wt-1308592cf275)
verdict: CONFIRMED (two doc notes); the 3 red cases are INFEASIBLE warnings: introduced, but nothing found that reaches them on the pin
cases: executed 938→941, red 3
origin: introduced 5 / pre-existing 1 / undecided 0
wrote-outside-worktree: 12 paths (metaharness-wave-adr0096/w4/u3-adv1 under the operator's cache directory)
needs-coordinator: whether H1b on Codex should cover more than `personality`, and whether H2 should refuse when /etc/codex/* exists

## Cases added (untracked, `crates/metaharness/tests/adversary_codex_strict_record_rows_pass1.rs`)

Each drives `CodexSeams` through the real `Run` with a scripted child that sends `session_meta`, `task_started`,
a hook request for `call-7`, and no `turn_context`. All three red; all three pass on `cfad31d` with only this
file added.

| case | asserts |
|---|---|
| `a_hook_call_released_by_finish_is_decided_like_any_other` | under observe, the held call gets a `tool.decided` when the stream ends |
| `a_halt_while_a_hook_call_is_held_abandons_it_like_any_other` | under ask, a `halt` abandons the held call |
| `a_hook_waiting_behind_the_held_opening_record_does_not_fail_the_run` | under ask, a child `WouldBlock` does not fail the run and the call was shown to the embedder |

```
call-7 was presented on the hook seam and the halt did not abandon it: ["session.started", "turn.started", "tool.requested", "command.result", "stream.closed"]
the run failed while a hook waited behind the held opening record (presented to the embedder: false): the child is waiting on a decision and metaharness holds none pending; neither side can leave this state, so it is reported rather than spun on
call-7 was presented on the hook seam and never decided: ["session.started", "turn.started", "tool.requested", "stream.closed"]
test result: FAILED. 0 passed; 3 failed
```

Suite with the cases: 938 passed, 3 failed (this file), 45 ignored.

## Attacked and not broken

The reader hold over a second `session_meta`, a `turn_context` without personality or with a non-object payload,
non-JSON and blank lines, end of stream; record order on 0.153.4 (three managed configs, an unknown model) and
0.160.0 matches the research page; `viewer.rs` citations caught by `project.rs`; the shared strict-version
guard leaves Claude's arguments unchanged and `codex --version` writes nothing; only golden line 1 moved; the
`personality` line precedes the first table in the c1 fixtures; no live consumer waits on `session.started`
with a timeout. Probes (credential-free, `/etc` on a private tmpfs): a managed `developer_instructions`
appeared in the developer message and a managed `model_instructions_file` replaced
`session_meta.base_instructions`, both with `personality` still "pragmatic".

## Findings

```findings
- file: crates/metaharness/src/run.rs
  line: 1266
  category: contract-drift
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: wind_up puts the hook call that CodexSeam::finish releases from the hold straight into the outbox after abandon_pending and without admit, so it is presented with decision_required true and never decided, abandoned or warned about
- file: crates/metaharness-codex/src/bridge.rs
  line: 176
  category: concurrency
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: a hook call held behind the opening record is invisible to the run's pending set, so CodexProcess's WouldBlock after BLOCK_GRACE_POLLS fails the run with "metaharness holds none pending" where the base presented and deadlined the call
- file: crates/metaharness/src/audit.rs
  line: 530
  category: judgement
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: H1b now reads ok from turn_context.personality alone while a managed layer's developer_instructions or model_instructions_file, observed in the same rollout's developer message and session_meta.base_instructions, leaves personality at "pragmatic"
- file: crates/metaharness-codex/src/launch.rs
  line: 1250
  category: judgement
  severity: warning
  verdict: INFEASIBLE
  origin: pre-existing
  message: H2 is attested imposed with no check for /etc/codex config, managed_config or requirements files, which observably override the scratch config.toml, where unit 2's managed_settings_found precedent would withdraw the claim
- file: docs/design/metaharness-protocol-v0.1.md
  line: 2811
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a34 says a machine carrying managed_config.toml or a trusted project config reads H1b gap, but a managed file that sets other keys records "pragmatic" and H1b reads ok
- file: crates/metaharness-codex/src/launch.rs
  line: 1244
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the H1b attestation says turn_context records the personality in force, but 0.153.4 records "pragmatic" with features.personality = false in a managed config
```

Agent cost: 254,439 tokens, 120 tool uses, 1,585 s.
