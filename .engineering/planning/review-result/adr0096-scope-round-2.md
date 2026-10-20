---
format: aep.planning-md/3
id: review-result:adr0096-scope-round-2
kind: review-result
status: active
title: 'Scope critic, round 2 (ADR 0096-0098 drafts): approve'
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
- reviews: story:governance-unit-stated
- reviews: epic:layer-pins-follow-their-owners
- reviews: story:metaharness-tools-on-tools-crates
- reviews: story:b10x-adapter-on-harness-over-llm
revision: 1
---
approve

What I read: 3 epics, 12 stories, 1 decision-blocker and 2 dependency-blockers, whole bodies, plus ADRs 0096, 0097 and 0098. I ran `aep plan artifact show` on each id and `aep plan artifact graph`. I extracted 18 promises from the three parents (4 from session-format, 9 from loom-agent-work, 5 from layer-pins) and traced all 18 to an item. I found no item that reaches past a parent's body or into its exclusions.

- **session-format:** `story:trace-ir-domain` claims the domain, conformance and the executable mapping. `story:project-reads-vendor-transcripts` claims "`metaharness project` reads each of the three vendor records directly". The epic's three exclusions (session mining, which formats survive, the family set and crate move) are not claimed anywhere.
- **loom-agent-work:** the six ADR 0098 Consequences gaps map one to one onto six stories, and the order edges match the epic's Order section. `story:aep-drive-disposition` is blocked by `decision-blocker:aep-drive-future`. `story:governance-unit-stated` claims the AGENTS.md sentence the epic promised. Gap 6 (`story:confinement-of-vendor-kinds`) is narrowed to attestation, and its Out of scope section says so, which the epic's "implemented or rejected with its reason recorded" allows.
- **layer-pins:** each of the two stories has its own blocker. The edge from the b10x story to the tools story is there. The research page and `CHANGELOG.md` outcomes each have one owner.

What I could not establish (all outside my lane, none set the verdict):
- **Design:** `story:confinement-of-vendor-kinds` makes `--hermetic strict` exit non-zero when `denied` is non-empty. Whether that changes "the hermetic floor", which the loom epic's Boundaries section says no story may change, is a design question.
- **Parallel safety:** `story:governance-unit-stated` says it "lands before" `story:call-seam-for-harness` and `story:aep-drive-disposition`, but it has no `depends_on` edge to either.

```findings
[]
```
