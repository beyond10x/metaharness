---
format: aep.planning-md/3
id: story:b10x-harness-pin-0-20-0
kind: story
status: implemented
title: metaharness admits b10x-harness 0.20.0
summary: Harness 0.20.0 adds run --approve caller; move the pin, re-record the golden from the released archive
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-26T12:38:24Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"test_result":1,"approval":1,"verification":1}}}
- {from: "proposed", to: "active", at: "2026-10-26T12:38:24Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1,"approval":1,"verification":1}}}
- {from: "active", to: "implemented", at: "2026-10-26T12:38:24Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"approval":1,"verification":1}}}
---
# Story: b10x-harness-pin-0-20-0

## Outcome

`metaharness run b10x --strict-version` admits an installed `b10x-harness` 0.20.0, the first release
with `run --approve caller`.

## Context

Harness 0.20.0 (`684c99f5`, https://github.com/beyond10x/harness/releases/tag/0.20.0) pins llm 0.15.0
and ESS 0.69.2 and adds `run --approve caller` for atlas ADR 0104. `harness-wire` and `harness-loop`
have no diff from 0.19.0. `b10x-chef-orchestrator` approved the admission on 2026-10-26 under the
grant (dispatch-queue #8): "Admit 0.20.0 in metaharness the way you described (steps 1-5 of your
report, amendment a60; the golden re-recorded from the checksum-verified 0.20.0 archive)". Design:
amendment a60.

Choice recorded: 0.19.0 leaves the pin, on a59 § 1's reasoning.

## Acceptance

- Both manifests, the lock, `PINNED_VERSIONS` and `HARNESS_REVISION` name 0.20.0 / `684c99f5`, and
  `task check` exits 0.
- The golden loop record is captured again from the released, checksum-verified 0.20.0 archive
  against harness's emulator, and the replay's expected stream moves only in `harness_version`.
- A credential-free `metaharness run b10x --strict-version --decisions observe` against the
  emulator exits 0 on 0.20.0 with this change and exits 2 before the spawn with metaharness 0.18.0.

## Out of Scope

- A paid one-turn run on 0.20.0.
- Using `--approve caller`; that is `story:b10x-caller-approval`.

## Ambiguities

None.

## Open Questions

None.
