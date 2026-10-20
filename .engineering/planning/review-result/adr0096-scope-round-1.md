---
format: aep.planning-md/3
id: review-result:adr0096-scope-round-1
kind: review-result
status: active
title: Scope critic, round 1 (ADR 0096-0098 drafts)
owner: metaharness
relations:
- reviews: epic:session-format
- reviews: story:trace-ir-domain
- reviews: story:project-reads-vendor-transcripts
- reviews: epic:loom-agent-work-through-metaharness
- reviews: story:caller-supplied-catalogue-on-every-kind
- reviews: story:decision-returns-caller-produced-result
- reviews: story:catalogue-changes-within-a-session
- reviews: story:continuity-across-executor-calls
- reviews: story:call-seam-for-harness
- reviews: story:confinement-of-vendor-kinds
- reviews: story:aep-drive-disposition
- reviews: epic:layer-pins-follow-their-owners
- reviews: story:metaharness-tools-on-tools-crates
- reviews: story:b10x-adapter-on-harness-over-llm
revision: 1
---
needs-revision
epic:loom-agent-work-through-metaharness — the promise "this repository says so rather than implying per-call governance it cannot deliver" has no claimant: no story or task has the outcome of adding a statement to AGENTS.md or README that the unit of governance for agent work is the session and its proposed change until the gaps close. A grep of AGENTS.md, README.md and docs finds no such statement today. Add it to a new small story, or to the acceptance of `story:aep-drive-disposition`. — .engineering/planning/epic/loom-agent-work-through-metaharness.md:26-28
story:metaharness-tools-on-tools-crates — the epic's Done-when promises "each with `task check` green and the research page written", but this story's acceptance ends at `CHANGELOG.md` and names no research page, so the promise is silently narrowed to the b10x story. Either claim the page here, or have the epic say it is the b10x pin's alone. — .engineering/planning/epic/layer-pins-follow-their-owners.md:59 (story acceptance at .engineering/planning/story/metaharness-tools-on-tools-crates.md:43-50)

What I read: 3 epics, 11 stories, 2 decision and dependency blockers (16 artifacts in all). I ran `aep plan artifact show` on each and `aep plan artifact graph` once, and read ADR 0096 (point 7, Consequences), ADR 0097 (point 3, Consequences) and ADR 0098 (Consequences). I extracted 14 promises from the three parents and the owner's four instructions, and traced 12 to an item. The two untraced ones are the findings above.

Checks that came out clean:
- **Session format:** the three outcomes and the Done-when trace to `story:trace-ir-domain` and `story:project-reads-vendor-transcripts`. The epic's exclusions (session mining, which trace formats survive, the family set) are respected.
- **Loom epic:**
  - The six ADR 0098 gaps map one-to-one to six stories, with no overlap and no extra gap.
  - The blocker lists keep, move to Loom and retire with costs, and recommends but does not act.
  - Every gap story is marked not in this wave.
- **Layer pins:**
  - Both stories are blocked by their release blockers.
  - The stories hold no work that is not in the epic.

What I could not establish: none. Two points I judged not to be findings:
- `story:aep-drive-disposition` has the outcome "carried out", while the epic's Done-when needs only the blocker cleared. The epic's Parts names the story and it is blocked, so I treated it as asked for.
- `story:project-reads-vendor-transcripts` adds scenarios to the same domain `story:trace-ir-domain` writes, but for the CLI verb, so I did not count it as double-claiming.

```findings
[
  {
    "file": ".engineering/planning/epic/loom-agent-work-through-metaharness.md",
    "line": 26,
    "category": "scope",
    "severity": "blocker",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "the epic promises that until the gaps close \"this repository says so rather than implying per-call governance it cannot deliver\" and no drafted item claims writing that statement into AGENTS.md or README; add a story, or put it in the acceptance of story:aep-drive-disposition"
  },
  {
    "file": ".engineering/planning/epic/layer-pins-follow-their-owners.md",
    "line": 59,
    "category": "scope",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "the Done-when promises each pin move with task check green and the research page written, but story:metaharness-tools-on-tools-crates names no research page in its acceptance (ends at CHANGELOG.md), so the promise is narrowed to the b10x story with nothing recording it; claim the page in that story or have the epic scope it to the b10x pin"
  }
]
```
