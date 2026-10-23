---
format: aep.planning-md/3
id: story:claude-dir-list-search-unregistered
kind: story
status: implemented
title: Claude adapter renders dir.list and search as Glob and Grep, which 2.1.305 and 2.1.309 do not register
summary: Frames admitting dir.list/search without shell give the model no working tool on the current Claude pins
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-23T17:41:00Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"test_result":1,"verification":1}}}
- {from: "proposed", to: "active", at: "2026-10-23T17:41:01Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1,"verification":1}}}
- {from: "active", to: "implemented", at: "2026-10-23T17:41:01Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Problem

The Claude adapter renders `dir.list` as `Glob` and `search` as `Grep`
(`crates/metaharness-claude/src/seam.rs:128` and the `Search` arm beside it). Claude Code 2.1.305 and
2.1.309 do not register either tool.

Evidence:

- Reported by session `ess-ui-templates` on 2026-10-23, run `ess-ui-ess-068-crates/1`
  (metaharness 0.14.0, claude 2.1.309). The `specify` frame admits
  `dir.list, file.edit, file.read, file.write, search, skill.load`. `session.started.offered_tools`
  lists neither `Glob` nor `Grep`. Both calls return
  `Error: No such tool available: Glob. Glob is not available in this session — find files with
  `find` via the Bash tool instead.` (`specify-0-1.jsonl` seq 17/19, and the same for Grep at seq 46).
- The 2026-10-23 credential-free qualification records (`~/.cache/metaharness-qualify-2026-10-23/claude/`)
  contain 42 `session.started` events on 2.1.305 and 47 on 2.1.309. None of them offers `Glob` or `Grep`.

So this predates 2.1.309. A frame that admits `dir.list`/`search` without `shell` leaves the model
no way to list or search; the only route the vendor offers is `Bash`, which the frame refuses.

Not this story: whether an AEP map's `specify`/`decompose`/`implement` steps should admit `shell`.
The map declares the operations and metaharness enforces them (`specify-0-1.refused.json`,
`refused-shell`). That question belongs to AEP's maps.

## Acceptance

- On every pinned Claude Code version, a frame admitting `dir.list` or `search` either reaches a
  tool the vendor registers that performs that operation, or is refused before the spawn by name.
  The decision is recorded as a design amendment.
- The native fixture asserts, per pinned version, that each operation the adapter renders is in
  `offered_tools`.
