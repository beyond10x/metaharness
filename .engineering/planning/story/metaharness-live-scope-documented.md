---
format: aep.planning-md/3
id: story:metaharness-live-scope-documented
kind: story
status: implemented
title: METAHARNESS_LIVE is documented as the guard of the paths that check it
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: cited
  path: website/docs/quickstart.md
- confidence: cited
  path: website/docs/status.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T20:32:26Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-21T20:32:26Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-22T00:04:37Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1}}}
---
# Story: METAHARNESS_LIVE is documented as the guard of the paths that check it

## Scheduling

Wave 7 unit 4 (`approval-record:adr-0096-wave-7`), from the operator's user trial of release 0.12.0 on
2026-10-21 (finding 03; the evidence is local to the operator and not in this repository).

## Outcome

Every document that names `METAHARNESS_LIVE` says which commands check it, and says that a plain
`metaharness run` does not: it spawns the harness with the operator's credentials whatever the variable
holds.

## Context

- `README.md:73`: "The live runs cost money, sit behind `METAHARNESS_LIVE=1`"; `website/docs/status.md:50`
  says the same. Observed on 0.12.0: a plain `run` with the variable unset spawned Codex and reached the
  provider.
- Readers of the variable (grep 2026-10-21): `crates/metaharness-aep/src/eval.rs`,
  `crates/metaharness-aep/src/drive/spending.rs`, `crates/metaharness-aep-eval/src/lib.rs`, the live tests
  under `crates/metaharness/tests/` and `crates/metaharness-cli/tests/`, `spec/domains/spending.yaml`.

## Decision (coordinator, 2026-10-21)

Scope the documents; do not enforce the variable on `run`. `run` is the embedder's launch command: a
required variable would break every caller that drives it today, and spending on `run` is the embedder's
decision through its own credentials.

## Acceptance

- expected_before: `README.md:73` and `website/docs/status.md:50` state the variable as the guard of live
  runs without naming the commands. After: each names the commands that check it (from the readers above,
  verified by a test or a cited line) and states that plain `run` does not.
- A test asserts that `README.md` and `website/docs/status.md` name `metaharness run` beside
  `METAHARNESS_LIVE`, so the scoped wording cannot drift back.
- `CHANGELOG.md` `[Unreleased]` one line under `### Changed`.

## Scope

`README.md`, `website/docs/status.md`, `website/docs/quickstart.md` where it names the variable, one test
file, `CHANGELOG.md`.
