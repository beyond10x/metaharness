---
format: aep.planning-md/1
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
