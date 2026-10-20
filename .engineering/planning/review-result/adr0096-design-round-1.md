---
format: aep.planning-md/3
id: review-result:adr0096-design-round-1
kind: review-result
status: active
title: Design critic, round 1 (ADR 0096-0098 drafts)
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
story:b10x-adapter-on-harness-over-llm — it moves the `b10x-harness-tools` pin in `crates/metaharness-b10x/Cargo.toml`, the same dependency story:metaharness-tools-on-tools-crates deletes, and neither an edge nor the narrowed scope says which lands first; narrow it to the loop pin plus an explicit `depends_on` (or `blocks`-by-order) between the two stories — .engineering/planning/story/b10x-adapter-on-harness-over-llm.md:50, .engineering/planning/story/metaharness-tools-on-tools-crates.md:45
story:call-seam-for-harness — its acceptance records vectors against "the pinned harness release" and its default seam is "the wire when harness-over-llm publishes one", both of which story:b10x-adapter-on-harness-over-llm moves and re-records in the same `metaharness-b10x` files (`lib.rs`, `vectors.rs`), and no `depends_on` edge records that order — .engineering/planning/story/call-seam-for-harness.md:52, .engineering/planning/story/call-seam-for-harness.md:68
story:caller-supplied-catalogue-on-every-kind — it extends the `Catalogue` that `metaharness-tools` re-exports from harness, while story:metaharness-tools-on-tools-crates replaces that re-export from `tools`, and its own format ambiguity ("`tool-contract/1` once `tools` publishes it; until then ... interim") admits the dependency without an edge; either order has a cost (edge behind an unreleased crate, or an interim shape the tools story must migrate), so the body should pick one and name it — .engineering/planning/story/caller-supplied-catalogue-on-every-kind.md:70, .engineering/planning/story/metaharness-tools-on-tools-crates.md:45
story:trace-ir-domain — it types "the four source kinds a document is normalized from" beside the header field `metaharness.source`, which code only ever writes as `metaharness.event/1`, while story:project-reads-vendor-transcripts decides in an open ambiguity whether that field takes the vendor names; the domain should keep the four kinds as inputs of the normalize commands and mark the document-field mapping `UNMAPPED:` until the second story decides — .engineering/planning/story/trace-ir-domain.md:100, .engineering/planning/story/project-reads-vendor-transcripts.md:69
story:aep-drive-disposition — its "move" option re-homes the conformance target and requires "the 45-plus scenarios", the very suite and `ess_conformance.rs` count gate that story:trace-ir-domain grows in the current wave, and no `depends_on` edge says so; add `depends_on story:trace-ir-domain` so the figure and the re-homed scenarios include `metaharness.trace` — .engineering/planning/story/aep-drive-disposition.md:46, .engineering/planning/story/trace-ir-domain.md:126

What I read: 14 artifacts with `aep plan artifact show` and the full `aep plan artifact graph --format json`. I walked 18 declared edges in the set: 11 `decomposes`, 2 `depends_on`, 2 `informed_by` and 3 `blocks`. I also followed the edges out to `story:trace-ir-reader` and `story:ess-specification-and-hardening` and ran `aep plan artifact relations`. I found no cycle. `aep plan artifact validate` printed `valid`, plus overdue review-outcome warnings on old review-results, which are not mine. I traced 4 acceptance reads to a producer in the set, and an edge records 2 of them. The recorded two are `project-reads-vendor-transcripts` reading `trace.yaml` from `trace-ir-domain`, and `catalogue-changes-within-a-session` reading the catalogue from `caller-supplied-catalogue`. The unrecorded two are `call-seam-for-harness` reading the pin from `b10x-adapter-on-harness-over-llm`, and `aep-drive-disposition` reading the conformance suite from `trace-ir-domain`. I also read ADRs 0097 and 0098 for the gap list and the tools move.

Could not establish:
- Whether the llm-based harness release still ships `b10x-harness-tools`. ADR 0097 says harness "adopts `tools` once T1 ships" and gives no ordering, so finding 1's order is open.
- Out of my lane, so it does not set the verdict: most of the loom stories edit `crates/metaharness/src/run.rs`, `spec.rs`, `docs/design/metaharness-protocol-v0.1.md` and `server.rs`. Every story that regenerates `spec/conformance.json` also touches the count gate. Parallel-safety owns that.
- Out of my lane, acceptance: the "three git pins" in the llm story cite `Cargo.toml:14-19`, which holds two b10x pins.

```findings
[
  {"file": ".engineering/planning/story/b10x-adapter-on-harness-over-llm.md", "line": 50, "category": "design", "severity": "blocker", "verdict": "needs-revision", "origin": "introduced", "message": "it moves the b10x-harness-tools pin in crates/metaharness-b10x/Cargo.toml that story:metaharness-tools-on-tools-crates deletes (acceptance line 45), and no edge or narrowed scope says which lands first; narrow it to the loop pin and add the depends_on between the two stories"},
  {"file": ".engineering/planning/story/call-seam-for-harness.md", "line": 52, "category": "design", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "its acceptance pins vectors to the harness release and its default seam is the wire from harness-over-llm (line 68), both moved and re-recorded by story:b10x-adapter-on-harness-over-llm in the same metaharness-b10x files, and no depends_on edge records that order"},
  {"file": ".engineering/planning/story/caller-supplied-catalogue-on-every-kind.md", "line": 70, "category": "design", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "it extends the Catalogue that metaharness-tools re-exports from harness while story:metaharness-tools-on-tools-crates replaces that re-export from tools, and its format ambiguity admits the dependency without an edge; the body should choose edge-behind-tools or interim-shape-the-tools-story-migrates"},
  {"file": ".engineering/planning/story/trace-ir-domain.md", "line": 100, "category": "design", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "it types four source kinds beside the header field metaharness.source, which code only writes as metaharness.event/1, while story:project-reads-vendor-transcripts (line 69) leaves open whether that field takes vendor names; keep the kinds as normalize-command inputs and mark the document-field mapping UNMAPPED until decided"},
  {"file": ".engineering/planning/story/aep-drive-disposition.md", "line": 46, "category": "design", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "its move option re-homes the conformance target and requires the 45-plus scenarios, the suite and count gate story:trace-ir-domain grows in the current wave, and no depends_on edge says so; add depends_on story:trace-ir-domain"}
]
```
