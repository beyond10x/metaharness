---
format: aep.planning-md/3
id: review-result:wave-0290-u2-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on story:catalogue-road-leftovers
summary: An entry no catalogue server performs was asked and allowed; fixed in correction 1
relations:
- reviews: story:catalogue-road-leftovers
revision: 1
---
unit: story:catalogue-road-leftovers, impl/catalogue-remainder at 14315b6 (base 52bc3a8, merged d196e98 into integration/wave-0-29-0); cases 544b9a8
verdict: NEEDS-CHANGE (strongest)
cases: executed 118→122, red 2
origin: introduced 1 / pre-existing 1 / undecided 1
wrote-outside-worktree: wave scratch only

The red cases are in `crates/metaharness/tests/adversary_u2_catalogue_road_pass1.rs`:

- An entry that maps to harness `run` (a `process.run` entry) counted as performed, although no catalogue
  server serves `run`. On Codex under ask it was put to the embedder, and its allow was recorded for a call
  the server refused (F1, origin undecided).
- The server's refusal for that entry came from the verb layer and named harness `run`, not the catalogue
  (F2, pre-existing).

Note: a socket swapped in was refused with `ENXIO` wording rather than by its kind (F3, introduced).

## Outcome

Correction 1 (`17a82fc`, merged with the cases at `3fd5666`):
- An entry counts as performed only if the run's server performs it. This holds under ask and under a
  frame; a new case covers frame.
- The server refuses such an entry naming the catalogue.
- A socket swapped in is refused as no longer a regular file.
- a40 now says `O_NONBLOCK` stays set.

All cases are green.

The story's `rev` pin of tool-contract was withdrawn by the wave lead. Harness pulls `beyond10x/tools` by the
same tag, so a `rev` pin adds a second `b10x-tool-contract` to the lock, and cargo refuses a patch to the same
source. The lock already fixes commit `99e36ed`.

```findings
[
{"file":"crates/metaharness/src/run.rs","line":1182,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"undecided","message":"outside_catalogue counted an entry as performed whenever harness maps its contract name, so on Codex under ask a process.run entry the server never serves was put to the embedder and its allow recorded."},
{"file":"crates/metaharness-tools/src/server.rs","line":166,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"For a mapped entry nothing performs, the server's refusal came from the verb layer and named harness run instead of the catalogue."},
{"file":"crates/metaharness-tools/src/caller.rs","line":766,"category":"judgement","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"A socket swapped in after the metadata check failed open(2) with ENXIO and was refused as No such device or address rather than by its kind."}
]
```
