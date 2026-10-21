---
format: aep.planning-md/3
id: review-result:adr0096-w4-codex-adversary-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:codex-strict-record-rows: four notes, pass-1 findings resolved'
relations:
- reviews: story:codex-strict-record-rows
revision: 1
---
unit: story:codex-strict-record-rows, pass 2 (the last) — correction round 1, uncommitted tree of impl/codex-strict-record-rows on cfad31d (wt-1308592cf275)
verdict: CONFIRMED (two notes); all six pass-1 findings resolved; one new red case, pre-existing and INFEASIBLE
cases: executed 945→946, red 1
origin: introduced 2 / pre-existing 2 / undecided 0
wrote-outside-worktree: 5 paths (metaharness-wave-adr0096/w4/u3-adv2 under the operator's cache directory)
needs-coordinator: none (cut once by HTTP 429, resumed)

## Ledger, pass 1 → pass 2

| pass-1 | status |
|---|---|
| F1 `run.rs:1266` | resolved (pass-1 case green; the bridge holds nothing; the reader holds only `turn.started` and `opaque`) |
| F2 `bridge.rs:176` | resolved |
| F3 `audit.rs:530` | resolved for file layers; cloud layers carried as N2 |
| F4 `launch.rs:1250` | resolved (H2 attested unavailable when layers exist; advisory `gap` on the floor) |
| F5, F6 | resolved |

Carried 0, new 4, resolved 6.

## Case added (untracked, `crates/metaharness-codex/tests/adversary_codex_strict_record_rows_pass2.rs`)

`the_opening_record_is_first_even_when_a_hook_is_read_before_session_meta`: red, and red the same way at
`cfad31d` (pre-existing): "the merged stream opens with ["tool.requested", "session.started", "turn.started", "opaque"]".
Suite with it: 945 passed, 1 failed, 45 ignored.

## Attacked and not broken

Release on hook (two hooks; a `turn_context` after release completes nothing; order follows arrival; `finish`
cannot return a call). The layer walk against the binary: 21 credential-free 0.153.4 probe runs over 15 layouts;
codex reads from the cwd up to the repository root (a real repo or a worktree gitfile), nothing above, the cwd
alone without a repo; follows a symlinked `.codex`; reads the operator's codex config when HOME is a repo, which
the walk counts too; the walk only over-reports. The walk adds no refusal path: H1b goes `unk`, strict on Codex
already exits 3, `--hermetic on` runs no audit, H2 is advisory.

## Findings

```findings
- file: crates/metaharness-codex/src/bridge.rs
  line: 34
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: the module says the opening record is first in the merged stream, but a hook request read before session_meta is emitted ahead of session.started
- file: crates/metaharness-codex/src/launch.rs
  line: 1366
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: H1b and H2 claim no config layer outside the scratch home exists, while the binary names cloud_requirements and cloud_managed_config layer sources that no file check detects under a ChatGPT login
- file: docs/design/metaharness-protocol-v0.1.md
  line: 2820
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a34 says an untrusted project's .codex/config.toml is not read, but under the workspace-write sandbox every --cwd launch writes, 0.153.4 reads it with no trust entry
- file: crates/metaharness-codex/tests/native_fixture.rs
  line: 711
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the five-second halt limit is timed from before the driver spawn, so load-driven startup (session_meta at 4.73 s and 13.68 s) fails it while halt-to-exit stays at or under 0.091 s
```

Agent cost: 349,531 tokens, 59 tool uses, 1,843 s on the final resume.
