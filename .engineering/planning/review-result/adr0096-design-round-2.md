---
format: aep.planning-md/3
id: review-result:adr0096-design-round-2
kind: review-result
status: active
title: Design critic, round 2 (ADR 0096-0098 drafts)
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
needs-revision
story:aep-drive-disposition — its `depends_on story:call-seam-for-harness` is justified only by "both edit AGENTS.md", but call-seam amends invariant 9 and the Live-evaluating section while disposition edits § What this repository owns; the story that edits that section is story:governance-unit-stated, which has no edge, so the edge sits on the wrong pair and drags two external-release blockers and the catalogue chain into a "keep" citation. Either keep the order and say so, or drop the call-seam edge and add `depends_on story:governance-unit-stated`. — .engineering/planning/story/aep-drive-disposition.md:17,72,89; .engineering/planning/story/governance-unit-stated.md:48,56; .engineering/planning/story/call-seam-for-harness.md:38
story:aep-drive-disposition — its acceptance requires the count gate to equal "the count story:trace-ir-domain left", but story:project-reads-vendor-transcripts also adds `metaharness.trace` scenarios to that gate and no edge orders the two, so the check fails if project-reads lands first. Either word the acceptance as the count in the file at dispatch, or add `depends_on story:project-reads-vendor-transcripts`. — .engineering/planning/story/aep-drive-disposition.md:70-71; .engineering/planning/story/project-reads-vendor-transcripts.md:64

What I read: 18 artifacts in full (the 3 epics, 14 stories, `decision-blocker:aep-drive-future`) plus the two dependency-blockers' edges. Commands run: `aep plan artifact show` for each, `relations`, `graph --format json` and `validate`.

Edges walked: 22 `depends_on` and `blocks` edges, extended to the neighbours `story:trace-ir-reader`, `story:ess-specification-and-hardening` and the blockers.
- No cycle. The longest chain is tools-on-tools → catalogue → decision-result → catalogue-changes → continuity → confinement.
- That chain is a shared-file order. Each edge names its files (`run.rs`, `spec.rs`, the launch files, `server.rs`), so I do not report it as serialising.
- Of 4 acceptance reads of a sibling's output, 3 have a recording edge: b10x-adapter reads trace-ir-domain's normalize scenario, project-reads reads trace-ir-domain's domain, and aep-disposition reads trace-ir-domain's count gate. The fourth, aep-disposition's count gate read against project-reads, is the second finding.
- `story:trace-ir-domain`, the one wave unit, has no outgoing `depends_on`.

`validate` reports `valid`. It also lists 15 review-results with no recorded outcome and 1 without a findings block; those are not mine.

What I could not establish:
- Whether `story:decision-returns-caller-produced-result` (`boundaries.yaml`) and `story:confinement-of-vendor-kinds` (`workspace.yaml`) also move the count gate. Neither lists `ess_conformance.rs` in its scope, so I did not cite them.
- Out of my lane (parallel-safety):
  - `story:project-reads-vendor-transcripts` and `story:aep-drive-disposition` both touch `ess_conformance.rs` without an edge.
  - The shared-file note in project-reads (`.engineering/planning/story/project-reads-vendor-transcripts.md`) does not mention that overlap.

```findings
[
  {
    "file": ".engineering/planning/story/aep-drive-disposition.md",
    "line": 17,
    "category": "design",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "its depends_on story:call-seam-for-harness is justified only by 'both edit AGENTS.md', but call-seam amends invariant 9 and the Live-evaluating section while disposition edits § What this repository owns, the section story:governance-unit-stated edits, which has no edge; the edge sits on the wrong pair and drags two external-release blockers and the catalogue chain into a 'keep' citation. Either keep the order and say so, or drop the call-seam edge and add depends_on story:governance-unit-stated."
  },
  {
    "file": ".engineering/planning/story/aep-drive-disposition.md",
    "line": 70,
    "category": "design",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "its acceptance requires the count gate to equal 'the count story:trace-ir-domain left', but story:project-reads-vendor-transcripts also adds metaharness.trace scenarios to that gate and no edge orders the two, so the check fails if project-reads lands first. Either word the acceptance as the count in the file at dispatch, or add depends_on story:project-reads-vendor-transcripts."
  }
]
```
