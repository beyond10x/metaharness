---
format: aep.planning-md/3
id: story:b10x-harness-pin-0-19-0
kind: story
status: implemented
title: metaharness admits b10x-harness 0.19.0
summary: A 0.19.0 loop read H9 gap against the 0.18.0 pin; move the pin, re-record the golden, release
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-26T08:51:15Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"test_result":1,"approval":1,"verification":1}}}
- {from: "proposed", to: "active", at: "2026-10-26T08:51:15Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1,"approval":1,"verification":1}}}
- {from: "active", to: "implemented", at: "2026-10-26T08:51:15Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"approval":1,"verification":1}}}
---
# Story: b10x-harness-pin-0-19-0

## Outcome

`metaharness run b10x --strict-version` admits an installed `b10x-harness` 0.19.0, so a trial's
harness arm no longer reads H9 `gap` against the released loop.

## Context

On 2026-10-24 a trial's harness arm failed with H9 `gap`: the vendor was 0.19.0 and the adapter was
pinned to `["0.18.0"]`. Harness 0.19.0 (`13d72c06`, https://github.com/beyond10x/harness/releases/tag/0.19.0)
moves the tools crates to 0.5.0 and changes no crate of `harness-cli`, `harness-llm`, `harness-wire`
or `harness-loop`. `b10x-chef-orchestrator` approved the admission on 2026-10-26 under the grant:
"metaharness admits b10x-harness 0.19.0 ... Admit 0.19.0 (keep or drop 0.18.0 as the repository's
admission rules say; record the choice) ... then release". Design: amendment a59.

Choice recorded: 0.18.0 leaves the pin. `PINNED_VERSIONS` names the release whose library crates the
workspace compiles, and `cargo_resolves_exactly_the_pinned_harness` holds the lock to
`PINNED_VERSIONS[0]`, so a second entry would be a claim about a binary nobody compiled against
(a59 § 1).

## Acceptance

- Both manifests, the lock, `PINNED_VERSIONS` and `HARNESS_REVISION` name 0.19.0 / `13d72c06`, the
  tools crates are at tag 0.5.0, and `task check` exits 0.
- The golden loop record is captured again from the released 0.19.0 archive against harness's
  emulator, and the replay's expected stream moves only in `harness_version`.
- A credential-free `metaharness run b10x --strict-version --decisions observe` against the
  emulator exits 0 on 0.19.0 with this change and exits 2 before the spawn with metaharness 0.17.0.

## Out of Scope

- A paid one-turn run on 0.19.0.
- The next harness release; it is its own admission.

## Ambiguities

None.

## Open Questions

None.
