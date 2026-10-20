---
format: aep.planning-md/3
id: review-result:adr0096-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, round 1 (ADR 0096-0098 drafts)
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
story:caller-supplied-catalogue-on-every-kind — lands on `crates/metaharness/src/run.rs`, `crates/metaharness-tools/src/server.rs` and the protocol design doc with story:decision-returns-caller-produced-result. It also lands on `spec.rs`, both `launch.rs` files, `run.rs` and `cli/src/lib.rs` with story:continuity-across-executor-calls. It shares `b10x/src/launch.rs` with story:call-seam-for-harness and `metaharness-tools/src/lib.rs` with story:metaharness-tools-on-tools-crates. Only the edge to story:catalogue-changes-within-a-session exists, and no body admits these files (inferred, prose Scope). Remedy: an ordering edge per pair naming the shared file, or split the surface. — .engineering/planning/story/caller-supplied-catalogue-on-every-kind.md:60
story:decision-returns-caller-produced-result — shares `crates/metaharness-tools/src/server.rs`, `crates/metaharness/src/run.rs`, `crates/metaharness-protocol/src/event.rs` and the protocol doc with story:catalogue-changes-within-a-session. Neither body mentions the other, and the only edge in that area is C→A (inferred, prose Scope). Remedy: an ordering edge naming the shared files, or split the surface. — .engineering/planning/story/decision-returns-caller-produced-result.md:57
story:continuity-across-executor-calls — shares `crates/metaharness/src/run.rs` with story:confinement-of-vendor-kinds and story:decision-returns-caller-produced-result. It shares `spec.rs`, both `launch.rs` files and `cli/src/lib.rs` with story:caller-supplied-catalogue-on-every-kind. No body admits any of it (inferred, prose Scope). Remedy: ordering edges naming the shared files, or split the surface. — .engineering/planning/story/continuity-across-executor-calls.md:54
story:confinement-of-vendor-kinds — shares `crates/metaharness/src/run.rs`, `metaharness-claude/src/launch.rs`, `metaharness-codex/src/launch.rs` and the protocol doc with story:caller-supplied-catalogue-on-every-kind and story:continuity-across-executor-calls. None of the bodies says so (inferred, prose Scope). Remedy: ordering edges naming the shared files, or split the surface. — .engineering/planning/story/confinement-of-vendor-kinds.md:60
story:call-seam-for-harness — shares `b10x/src/lib.rs`, `seam.rs` and `vectors.rs` with story:b10x-adapter-on-harness-over-llm. It shares `b10x/src/launch.rs` with story:caller-supplied-catalogue-on-every-kind and `AGENTS.md` with story:aep-drive-disposition. Its "vector against the pinned harness release" reads the pin that story:b10x-adapter-on-harness-over-llm moves, with no `depends_on` edge. Nothing is named (inferred, prose Scope). Remedy: an ordering edge from this story to the pin story naming the files, or split the surface. — .engineering/planning/story/call-seam-for-harness.md:46, .engineering/planning/story/call-seam-for-harness.md:59
story:b10x-adapter-on-harness-over-llm — re-pins `b10x-harness-tools` and `b10x-harness-loop` in `crates/metaharness-b10x/Cargo.toml:14-19`, and story:metaharness-tools-on-tools-crates removes that same dependency from the same file. Both also edit `Cargo.lock`, `CHANGELOG.md` and `b10x/src/seam.rs` (cited, both Scope sections). Each body lists the other only under Out of scope and no edge orders them. This story re-records the golden loop vector that the other story's acceptance reads ("the recorded vectors already assert"), so a reader sits beside its producer. Remedy: a `depends_on` edge from the tools story to this one, or merge the two. — .engineering/planning/story/b10x-adapter-on-harness-over-llm.md:63, .engineering/planning/story/metaharness-tools-on-tools-crates.md:43
story:metaharness-tools-on-tools-crates — lands on `crates/metaharness-tools/src/lib.rs` and `crates/metaharness-cli/src/lib.rs:300` with story:caller-supplied-catalogue-on-every-kind, which edits the same server construction and `Catalogue` re-exports, and neither says so (cited and inferred). Remedy: an ordering edge naming the files, or split the surface. — .engineering/planning/story/metaharness-tools-on-tools-crates.md:57
story:aep-drive-disposition — claims all of `crates/metaharness-aep`, which holds `drive/ess_conformance.rs` and `Cargo.toml`, both cited by story:trace-ir-domain. It also claims `cli/src/lib.rs` (shared with story:project-reads-vendor-transcripts), `AGENTS.md` (shared with story:call-seam-for-harness), `Cargo.lock` and `crates/metaharness-aep-eval` (shared with story:b10x-adapter-on-harness-over-llm). Its "move to Loom" acceptance reads the "45-plus" suite that story:trace-ir-domain extends, with no `depends_on` edge. The whole-crate scope also collides with everything touching the aep crate (inferred, prose Scope). Remedy: an edge to story:trace-ir-domain and a narrowed scope, or merge. — .engineering/planning/story/aep-drive-disposition.md:42, .engineering/planning/story/aep-drive-disposition.md:54
story:trace-ir-domain — its acceptance reads the b10x golden loop record (`crates/metaharness-b10x/fixtures/golden/loop.jsonl`, "10 events") through `B10xSeams`, which story:b10x-adapter-on-harness-over-llm re-records (cited, both). No `depends_on` edge runs from the reader to the producer. Remedy: add the edge, or state that the fixture is frozen. — .engineering/planning/story/trace-ir-domain.md:122, .engineering/planning/story/b10x-adapter-on-harness-over-llm.md:48
story:trace-ir-domain — hard-codes "amendment a26" in `docs/design/metaharness-protocol-v0.1.md` and edits `CHANGELOG.md` (cited). story:caller-supplied-catalogue-on-every-kind, story:decision-returns-caller-produced-result, story:catalogue-changes-within-a-session, story:continuity-across-executor-calls, story:call-seam-for-harness and story:confinement-of-vendor-kinds each amend that doc, and story:metaharness-tools-on-tools-crates and story:b10x-adapter-on-harness-over-llm edit `CHANGELOG.md`. The amendment number, and `Cargo.lock` (inferred), are shared and unnamed. Remedy: ordering edges naming the shared files, or have each story claim its amendment number. — .engineering/planning/story/trace-ir-domain.md:122
story:project-reads-vendor-transcripts — its `cli/src/lib.rs` (`project` verb, line 236) is also claimed by story:caller-supplied-catalogue-on-every-kind, story:continuity-across-executor-calls and story:aep-drive-disposition, and the body names none of them (inferred, prose Scope). Remedy: ordering edges naming the file, or split the surface. Its edge to story:trace-ir-domain already covers `trace.yaml`, `ess_conformance.rs` and the design page. — .engineering/planning/story/project-reads-vendor-transcripts.md:61

**What I read:** 16 artifacts (3 epics, 11 stories, plus the epic bodies) with `aep plan artifact show`, `aep plan artifact waves --kind story --status draft --format json`, `aep plan artifact graph`, `aep plan artifact relations`, and `git grep`/`ls` on the tree. Items placed: 3 cited (story:trace-ir-domain, story:metaharness-tools-on-tools-crates, story:b10x-adapter-on-harness-over-llm). 8 inferred (story:project-reads-vendor-transcripts, the five Loom gap stories A to F, story:aep-drive-disposition). 0 unplaceable. Acceptance reads traced to a producer in the set: 5 (project-reads→trace-ir-domain, catalogue-changes→caller-supplied-catalogue, call-seam→b10x-pin, trace-ir-domain→b10x golden, aep-drive→trace-ir-domain suite, tools-on-tools→b10x-pin). The `depends_on` edges record 2 of them.

**What I could not establish:**
- `aep plan artifact waves` reports 10 of the 11 stories as unassessed. Everything except story:trace-ir-domain carries only prose Scope, not typed `scope`. By my ladder each does name an inferred surface, so none is surface-less. The tool cannot compute their collisions, though, and it will not order them until the Scope is typed.
- `story:aep-drive-disposition` claims the whole `crates/metaharness-aep` crate, which is a surface so wide it collides with everything near the aep crate.
- Out of the set, so not a finding. `waves` places story:trace-ir-domain in wave 2 behind story:llm-route-bindings, on an inferred `Cargo.lock` collision; it also reports story:llm-route-evidence and story:selector-metrics. I did not assess them.
- Out of my lane: whether the Loom gap stories are split correctly, and the `requires-stakeholder-input` ambiguities.
- The cited line numbers are heading lines (the Scope or Acceptance heading), since the `## Scope` sections run to a few lines each.

```findings
- file: .engineering/planning/story/caller-supplied-catalogue-on-every-kind.md
  line: 60
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "lands on run.rs, tools/server.rs and the protocol doc with story:decision-returns-caller-produced-result, on spec.rs, both launch.rs files, run.rs and cli/src/lib.rs with story:continuity-across-executor-calls, on b10x/src/launch.rs with story:call-seam-for-harness and on metaharness-tools/src/lib.rs with story:metaharness-tools-on-tools-crates; no body names any of it (inferred, prose Scope). Remedy: ordering edges naming the shared file, or split the surface."
- file: .engineering/planning/story/decision-returns-caller-produced-result.md
  line: 57
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "shares tools/server.rs, run.rs, event.rs and the protocol doc with story:catalogue-changes-within-a-session, and neither body mentions the other; no edge orders them (inferred, prose Scope). Remedy: an ordering edge naming the shared files, or split the surface."
- file: .engineering/planning/story/continuity-across-executor-calls.md
  line: 54
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "shares run.rs with story:confinement-of-vendor-kinds and story:decision-returns-caller-produced-result, and spec.rs, both launch.rs files and cli/src/lib.rs with story:caller-supplied-catalogue-on-every-kind; no body admits it (inferred, prose Scope). Remedy: ordering edges naming the shared files, or split the surface."
- file: .engineering/planning/story/confinement-of-vendor-kinds.md
  line: 60
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "shares run.rs, claude/codex launch.rs and the protocol doc with story:caller-supplied-catalogue-on-every-kind and story:continuity-across-executor-calls; none says so (inferred, prose Scope). Remedy: ordering edges naming the shared files, or split the surface."
- file: .engineering/planning/story/call-seam-for-harness.md
  line: 59
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "shares b10x lib.rs, seam.rs, vectors.rs with story:b10x-adapter-on-harness-over-llm, b10x/src/launch.rs with story:caller-supplied-catalogue-on-every-kind and AGENTS.md with story:aep-drive-disposition; its vector against the pinned harness release reads the pin the b10x pin story moves, with no depends_on edge (inferred, prose Scope). Remedy: an ordering edge naming the files, or split the surface."
- file: .engineering/planning/story/b10x-adapter-on-harness-over-llm.md
  line: 63
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "re-pins b10x-harness-tools and b10x-harness-loop in metaharness-b10x/Cargo.toml:14-19 that story:metaharness-tools-on-tools-crates removes from the same file, and both edit Cargo.lock, CHANGELOG.md and b10x/src/seam.rs (cited); each names the other only under Out of scope and no edge orders them; the tools story's acceptance reads the golden loop vector this story re-records. Remedy: a depends_on edge from the tools story to this one, or merge the two."
- file: .engineering/planning/story/metaharness-tools-on-tools-crates.md
  line: 57
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "lands on metaharness-tools/src/lib.rs and cli/src/lib.rs:300 with story:caller-supplied-catalogue-on-every-kind and neither says so (cited and inferred). Remedy: an ordering edge naming the files, or split the surface."
- file: .engineering/planning/story/aep-drive-disposition.md
  line: 54
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "claims all of crates/metaharness-aep (ess_conformance.rs and Cargo.toml, cited by story:trace-ir-domain), plus cli/src/lib.rs, AGENTS.md, Cargo.lock and metaharness-aep-eval; its move-to-Loom acceptance reads the 45-plus suite story:trace-ir-domain extends with no depends_on edge; the whole-crate scope collides with everything near the aep crate (inferred, prose Scope). Remedy: an edge to story:trace-ir-domain and a narrowed scope, or merge."
- file: .engineering/planning/story/trace-ir-domain.md
  line: 122
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its acceptance reads the b10x golden loop record (loop.jsonl, 10 events) through B10xSeams, which story:b10x-adapter-on-harness-over-llm re-records (cited, both); no depends_on edge runs from the reader to the producer. Remedy: add the edge, or state the fixture is frozen."
- file: .engineering/planning/story/trace-ir-domain.md
  line: 122
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "hard-codes amendment a26 in the protocol design doc and edits CHANGELOG.md (cited), both also claimed by six Loom gap stories (doc, inferred) and the two pin stories (CHANGELOG.md); Cargo.lock is shared too (inferred); none names it. Remedy: ordering edges naming the shared files, or have each story claim its own amendment number."
- file: .engineering/planning/story/project-reads-vendor-transcripts.md
  line: 61
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its cli/src/lib.rs (the project verb) is also claimed by story:caller-supplied-catalogue-on-every-kind, story:continuity-across-executor-calls and story:aep-drive-disposition, and the body names none (inferred, prose Scope). Remedy: ordering edges naming the file, or split the surface."
```
