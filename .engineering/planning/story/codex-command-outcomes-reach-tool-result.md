---
format: aep.planning-md/3
id: story:codex-command-outcomes-reach-tool-result
kind: story
status: implemented
title: Codex command outcomes reach the portable tool.result
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/metaharness-codex/src/conformance.rs
- confidence: cited
  path: crates/metaharness-codex/src/observations.rs
- confidence: cited
  path: crates/metaharness-codex/src/vectors.rs
- confidence: cited
  path: crates/metaharness-codex/tests/observations.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: cited
  path: spec/conformance.json
- confidence: cited
  path: spec/domains/observations.yaml
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-22T00:51:01Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-22T00:51:01Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-22T07:46:30Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
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

Derived 2026-10-22 by `story-scoper` at `a34551e` (release 0.13.0); decision D13 by the coordinator.

- Primary surface: `crates/metaharness-codex/src/observations.rs`: `Observations::call` keys calls by
  `call_id` (`:67`, `:85-95`); `Observations::structured` (`:115-209`) rejects `completed` with a nonzero
  exit (`:139-146`) and emits `TOOL_OUTCOME_UNVERIFIED` for an unmatched completion (`:183`, `:202-206`).
- Tests: `crates/metaharness-codex/tests/observations.rs:154-175` (correlated), `:177-197` (unmatched; gains
  an assertion on the warning), `:243-256` (pins `completed` + exit 7 to null; changes only if the source
  read below shows that shape).
- Vectors: `crates/metaharness-codex/src/vectors.rs:62-63` gains a sanitized 0.153.4 `item_completed` pair
  (exit 0 and exit 7); `crates/metaharness-codex/src/conformance.rs:60-95` a case for it.
- ESS: `spec/domains/observations.yaml:43-45`, `:70-80` and `crates/metaharness-aep/src/drive/ess_conformance.rs:876-906`
  (`observed_codex`) only if a new evidence variant is needed; a named conformance scenario for the
  correlated exit either way.
- Design: amendment a47 to `docs/design/metaharness-protocol-v0.1.md`, changing the a22 `tool.result`
  paragraph (`:2027-2041`) and the "namespaces are not joined" statement (`:1151`, `bridge.rs:17-27`).
- `CHANGELOG.md` under `[Unreleased]`.

Decision:

- D13 the join is decided from Codex source at the pinned tag (`rust-v0.153.4`), cited as file:line, before
  any code: which id the `item_completed` record of a mediated exec carries (`call_*` or `exec-*`), which id
  the hook `tool_use_id` carries, and which `status` a nonzero exit gets. The amendment records the rule.
  If the source shows no key that joins the two without order, text or output, the story ends declined
  with that finding recorded, and nothing is built (invariant 3).
