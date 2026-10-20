---
format: aep.planning-md/3
id: review-result:adr0096-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic, round 2 (ADR 0096-0098 drafts)
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
story:metaharness-tools-on-tools-crates — shares `CHANGELOG.md` (cited in trace-ir-domain, inferred here) and `Cargo.lock` (inferred) with story:trace-ir-domain, and changes `crates/metaharness-b10x/src/seam.rs`, which holds the `B10xSeams` reader trace-ir-domain's scenarios drive; no `depends_on` edge and neither body names the other. Add an ordering edge recording the shared files, or split the surface — .engineering/planning/story/metaharness-tools-on-tools-crates.md:78
story:caller-supplied-catalogue-on-every-kind — shares `docs/design/metaharness-protocol-v0.1.md` with story:trace-ir-domain (cited there as amendment a26, inferred here), and its Order names only the `tools` story. An edge to trace-ir-domain would also cover the gap-story chain after it, which reaches trace-ir-domain only through this story — .engineering/planning/story/caller-supplied-catalogue-on-every-kind.md:74
story:call-seam-for-harness — shares the protocol design page (inferred) with story:decision-returns-caller-produced-result, story:catalogue-changes-within-a-session, story:continuity-across-executor-calls and story:confinement-of-vendor-kinds, but its only gap-chain edge is to caller-supplied; the epic says those stories are "serialized by `depends_on` edges" and orders call-seam only "after the catalogue". Add the missing edges or split the surface by section — .engineering/planning/story/call-seam-for-harness.md:69
story:aep-drive-disposition — shares `crates/metaharness-aep/src/drive/ess_conformance.rs` and `crates/metaharness-cli/src/lib.rs` with story:project-reads-vendor-transcripts (inferred), and `crates/metaharness-cli/src/lib.rs` with story:continuity-across-executor-calls, with no edge to either (both reach it only through trace-ir-domain or call-seam); its Order names only trace-ir-domain and call-seam — .engineering/planning/story/aep-drive-disposition.md:86
story:aep-drive-disposition — its acceptance requires the ESS count gate to equal "the count `story:trace-ir-domain` left", but story:project-reads-vendor-transcripts adds `metaharness.trace` scenarios to the same suite and no edge runs from this story to it; the waves output already places project-reads first. Add the edge to project-reads, or restate the expected count — .engineering/planning/story/aep-drive-disposition.md:70
story:governance-unit-stated — claims the other stories "edit other sections" of `AGENTS.md`, but story:aep-drive-disposition's acceptance cites the decision in the same § What this repository owns, and "lands before both" is prose with no edge — .engineering/planning/story/governance-unit-stated.md:56
story:project-reads-vendor-transcripts — its "Shared file, split by symbol" list for `crates/metaharness-cli/src/lib.rs` omits story:metaharness-tools-on-tools-crates, which touches the same file at the server construction (line 300); the waves output lists the collision and no edge or named split covers it — .engineering/planning/story/project-reads-vendor-transcripts.md:77

Can story:trace-ir-domain run alone now: yes. It has no `depends_on` and no open blocker. Ten of its eleven scope entries are cited, and the one inferred entry is `Cargo.lock`. A wave of one has no concurrent peer inside this set. The seven findings concern the later waves, and they are warnings because none changes trace-ir-domain itself. Its own pair edges (project-reads on `spec/domains/trace.yaml` and `docs/design/runs-side-by-side-v0.1.md`, aep-drive-disposition and b10x-adapter on `ess_conformance.rs` and the golden record) are recorded.

Later waves:
- **Safely ordered:** the gap-story chain (caller-supplied → decision-returns → catalogue-changes → continuity → confinement), tools → b10x-adapter → call-seam, and trace-ir-domain → project-reads and → aep-drive-disposition.
- **Not recorded:** the pairs in the findings above.
- **Why the order holds anyway:** `aep plan artifact waves` serializes colliding scopes, so it separates these pairs. That order is a tie-break, not a recorded reason. It is also computed from scopes that are almost all inferred.

What I read: 12 stories, 3 epics and 3 blockers, through `aep plan artifact waves --kind story --status draft --format json`, `aep plan artifact graph --format json`, `aep plan artifact show` on each id, and `git grep`/`ls` on the tree. Surfaces established: 2 cited (trace-ir-domain mostly, governance-unit-stated), 10 inferred, 0 unplaceable. Acceptance reads traced to a producer in the set: 14, of which 13 are recorded by an edge; the unrecorded one is the count gate in aep-drive-disposition.

What I could not establish:
- Whether every inferred scope matches the files the work will touch; the ten stories other than trace-ir-domain and governance-unit-stated rest on inferred surfaces.
- Collisions with items outside the set. `Cargo.lock` is shared with story:llm-route-bindings, story:llm-route-evidence and others, and the waves output puts trace-ir-domain in wave 3 behind them. I did not judge them, and they are blocked anyway. Wave of one is therefore the operator's selection, not the store's output.
- Out of my lane: whether the catalogue → decision → continuity chain is the right split (design critic).

```findings
[
  {"file": ".engineering/planning/story/metaharness-tools-on-tools-crates.md", "line": 78, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "shares CHANGELOG.md (cited in trace-ir-domain, inferred here) and Cargo.lock (inferred) with story:trace-ir-domain and changes crates/metaharness-b10x/src/seam.rs, the B10xSeams reader trace-ir-domain's scenarios drive; no depends_on edge and neither body names the other. Add an ordering edge recording the shared files or split the surface. The Cargo.lock and seam.rs surfaces are inferred."},
  {"file": ".engineering/planning/story/caller-supplied-catalogue-on-every-kind.md", "line": 74, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "shares docs/design/metaharness-protocol-v0.1.md with story:trace-ir-domain (cited in that body as amendment a26, inferred here) and its Order names only the tools story; the gap-story chain after it reaches trace-ir-domain only through this story. Add an ordering edge or split the surface. Surface is inferred on this side."},
  {"file": ".engineering/planning/story/call-seam-for-harness.md", "line": 69, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "shares docs/design/metaharness-protocol-v0.1.md with decision-returns-caller-produced-result, catalogue-changes-within-a-session, continuity-across-executor-calls and confinement-of-vendor-kinds, but its only gap-chain edge is to caller-supplied-catalogue-on-every-kind while the epic says those stories are serialized by depends_on edges. Add the edges or split by section. Surface is inferred."},
  {"file": ".engineering/planning/story/aep-drive-disposition.md", "line": 86, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "shares ess_conformance.rs and crates/metaharness-cli/src/lib.rs with project-reads-vendor-transcripts and crates/metaharness-cli/src/lib.rs with continuity-across-executor-calls with no edge to either; its Order names only trace-ir-domain and call-seam-for-harness. Add an ordering edge or split the surface. Surfaces are inferred."},
  {"file": ".engineering/planning/story/aep-drive-disposition.md", "line": 70, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "acceptance requires the ESS count gate to equal the count story:trace-ir-domain left, but story:project-reads-vendor-transcripts adds metaharness.trace scenarios to the same suite and no depends_on edge runs from this story to it. Add the edge to project-reads-vendor-transcripts or restate the expected count."},
  {"file": ".engineering/planning/story/governance-unit-stated.md", "line": 56, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "states that call-seam-for-harness and aep-drive-disposition edit other sections of AGENTS.md, but aep-drive-disposition's acceptance cites the decision in the same section (What this repository owns); 'lands before both' is prose with no depends_on edge. Record the order as an edge or name the true split."},
  {"file": ".engineering/planning/story/project-reads-vendor-transcripts.md", "line": 77, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the shared-file split list for crates/metaharness-cli/src/lib.rs omits story:metaharness-tools-on-tools-crates, which touches the same file at the server construction; no edge or named split covers it. Surface is inferred."}
]
```
