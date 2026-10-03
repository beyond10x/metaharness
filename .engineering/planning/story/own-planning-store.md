---
format: aep.planning-md/3
id: story:own-planning-store
kind: story
status: implemented
title: metaharness plans in a store of its own
summary: The .engineering store, pinned to aep 0.42.0; roadmap items become epics here before code.
owner: metaharness
tags:
- store
relations:
- decomposes: epic:runs-side-by-side
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-02T22:54:59Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"test_result":1}}, imported: true}
- {from: "proposed", to: "active", at: "2026-09-02T22:55:00Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1}}, imported: true}
- {from: "active", to: "implemented", at: "2026-09-02T22:55:00Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}, imported: true}
---
# Story: metaharness plans in a store of its own

## Outcome

Anybody working on metaharness finds its plan in `.engineering/planning/`, mutated only through `aep artifact`, pinned to a protocol tree by commit.

## Context

Until 2026-09-02 metaharness had no `.engineering/` directory. `docs/ROADMAP.md` carries operator-scheduled directions and stays; the store holds the work items derived from them and from `beyond10x/bench`.

## Acceptance

- `.engineering/project.yaml` names the `aep` tree by a 40-hex commit and `development.standard`.
- `aep artifact validate` exits 0.
- `AGENTS.md` names the store and the rule that a roadmap item becomes an epic here before code.

## Out of Scope

Converting every roadmap section into an epic. Only the ones with work behind them.

## Ambiguities

- `inferable` — the pin is `a054945cf55229861b7e7b9e83e94343278cbc02`, `aep` tag `0.42.0`.

## Open Questions

None.
