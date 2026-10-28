---
format: aep.planning-md/3
id: approval-record:wave-d170
kind: approval-record
status: draft
title: 'Wave D170: RunSpec document, version probe under load, harness 0.22.0 pin'
tags:
- wave
relations:
- decides: story:run-spec-document
- decides: story:version-probe-under-load
- decides: story:b10x-harness-pin-0-22
revision: 1
---
## Approval

Approved by `b10x-chef-orchestrator` under the grant in the workspace instructions (class C: a wave;
commits, pushes, PRs and merges through the bot route; a release through metaharness's own process;
a dependency upgrade). The approval is the dispatch message; the wave skips the stage-1 stop. Quoted
from it, 2026-10-28:

> Item D170: one metaharness wave of 3 units with one integration branch, one PR to main and one
> release (D169, brief.md).
> 1. A RunSpec as a document. The consumer is takt's
> dependency-blocker:metaharness-session-request-document. A session request is parsed from a public
> document format with a public parse function. Unknown keys are refused with a named error, and the
> spec is checked per harness kind (check_spec). Create the story if none exists (search the store
> first), scope it, and move it to active.
> 2. F4 from the takt 0.17.0 trial: the `--version` probe at builder.rs:2429 has a 5 s deadline and
> fails at load 33-63. Make it robust under load: a longer deadline, or a probe that is not on the
> hot path. Start with a failing test, as a story or task.
> 3. Pin hop: move to the newest harness (0.22.0), llm and ESS (0.75.0), and run the newest ESS's
> checks.
> Each unit is test-first with an adversary pass. Merge on green, then release metaharness through
> its own process.

And, from the same coordinator, 2026-10-28: "Run only targeted tests per unit. Run one local full
gate on the integration branch once all 3 units are in. Then open one PR through the bot, set it to
auto-merge, and release."

## Wave

Skill: aep:implementing 0.24.1, wave mode. Base `75a44bcc` (origin/main, tag 0.22.0) plus
`9ff83d98` (`deps: move ESS to 0.75.0`, the D158 sweep's commit, fast-forwarded so the ESS move is
not made twice). Integration tree `mh-d170-int`, branch `wave/d170`.

| unit | story | tree | branch |
|---|---|---|---|
| 1 | story:run-spec-document | mh-d170-u1 | unit/d170-u1-run-spec-document |
| 2 | story:version-probe-under-load | mh-d170-u2 | unit/d170-u2-version-probe |
| 3 | story:b10x-harness-pin-0-22 | mh-d170-u3 | unit/d170-u3-harness-0-22 |

Build directory for every tree: one shared wave target, builds serialised by a lock and a touch of
the tracked files on every tree switch. Scratch: the wave's scratch directory under the session's
scratch root.

Selection: `aep plan artifact waves --kind story --status draft` placed run-spec-document and
b10x-harness-pin-0-22 in wave 1 and version-probe-under-load in wave 2 (collision on
`crates/metaharness/src/builder.rs`). The coordinator keeps all three in this wave as the item
names them, with the file split by function: unit 1 edits `check_spec` only, unit 2 edits
`VERSION_PROBE_DEADLINE` and its tests only; unit 1 takes design amendment a64, unit 2 a65.

Commits this approval authorises: one or more per unit on its branch, the merges into `wave/d170`,
the store commits on `wave/d170`, one PR to `main` merged through the App, the release tag and the
release publication of metaharness's own process.
