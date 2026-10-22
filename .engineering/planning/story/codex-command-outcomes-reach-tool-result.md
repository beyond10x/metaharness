---
format: aep.planning-md/3
id: story:codex-command-outcomes-reach-tool-result
kind: story
status: draft
title: Codex command outcomes reach the portable tool.result
revision: 1
---
# Story: Codex command outcomes reach the portable tool.result

## Scheduling

Draft for wave 8, from the operator's maintainer campaign on release 0.12.0 with Codex 0.153.4 on
2026-10-21 (finding F1; the evidence is local to the operator and not in this repository). Follow-up
to https://github.com/beyond10x/metaharness/issues/20.

## Outcome

A Codex shell call whose exit the vendor record states reaches the `metaharness.event/1`
`tool.result` with `exit_code`, `is_error` and `outcome_source` set, on the pinned Codex 0.153.4.

## Context

- Observed: two allowed calls exited 0 and 7 (`tool_use_result.exit_code` in the vendor record); both
  portable `tool.result` records carried null `exit_code`, `is_error` and `outcome_source`, each
  followed by `TOOL_OUTCOME_UNVERIFIED` "completion has no matching observed call".
- The outer exec call carries a `call_*` identifier and the mediated command an `exec-*` identifier;
  the unmatched branch is `crates/metaharness-codex/src/observations.rs:204`.
- A completion that cannot be correlated, or that contradicts the call, stays unknown (invariant 3).

## Acceptance

- expected_before: a recorded 0.153.4 rollout with a mediated exec of exit 0 and one of exit 7 yields
  two `tool.result` records with null `exit_code` and two `TOOL_OUTCOME_UNVERIFIED` warnings. After:
  `exit_code` 0 and 7, `is_error` false and true, `outcome_source` set, and no such warning.
- A rollout whose completion names no observed call still yields null and the warning.
- The recorded vectors are sanitized fixtures, never a retained real transcript.

## Scope

`crates/metaharness-codex/src/observations.rs`, the Codex recorded vectors, the design document's
Codex outcome section, `CHANGELOG.md` (to be confirmed by the story scoper).
