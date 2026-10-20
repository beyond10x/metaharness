---
format: aep.planning-md/3
id: review-result:adr0096-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1 (ADR 0096-0098 drafts)
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
story:trace-ir-domain — the acceptance is one sentence joining seven outcomes with semicolons (gate exit, scenarios executed, zero skipped, count gate and command list, coverage.md, amendment a26 with the runs-side-by-side pointer, CHANGELOG), so one can pass while another fails, and "the new total" is never stated, so the count gate cannot be read off the acceptance — .engineering/planning/story/trace-ir-domain.md:124
story:decision-returns-caller-produced-result — the acceptance expects `tool.result` to carry `decided_by`, but that field is on `tool.decided` and `ToolResult` has none, so the stated check cannot pass as written — .engineering/planning/story/decision-returns-caller-produced-result.md:46 (`crates/metaharness-protocol/src/event.rs:623` and `:632`)
story:decision-returns-caller-produced-result — "the same answer is refused by name at run start" refuses at a moment before the answer exists, since the answer is given mid-run to a `tool.requested`, so the observable (spawn refusal, a `tool.decided`, or an error exit) is unnamed — .engineering/planning/story/decision-returns-caller-produced-result.md:46
story:confinement-of-vendor-kinds — the acceptance names no outer confinement that the recorded vector runs inside, and `session.started.hermetic` has no write-surface or denied-path field today, so with Substrate excluded nobody can reproduce the "attempted write outside the grant appears as a denial" check — .engineering/planning/story/confinement-of-vendor-kinds.md:48
story:aep-drive-disposition — the section holds three alternative acceptance sets, each with several outcomes, and none applies until the blocker picks one; "the 45-plus scenarios still pass" also names no number — .engineering/planning/story/aep-drive-disposition.md:44

What I read: all 11 stories (the 2 in epic:session-format, the 7 in epic:loom-agent-work-through-metaharness, the 2 in epic:layer-pins-follow-their-owners) in full with `aep plan artifact show`, plus `aep plan artifact kinds` and `aep plan artifact lifecycle story`. I spot-checked `offered_tools`, `resolve_verb`, amendments a24 and a25, the `tool.decided` and `tool.result` fields, and the hermetic fields with `git grep`.

What I could not establish:
- The cut-off for "more than one statement". I flagged only trace-ir-domain, which is in the wave. The other stories also chain clauses, but each describes one verifiable outcome set.
- I did not check that the ESS 0.56.0 suite really regenerates the scenario counts the trace-ir-domain context cites.
- Out of my lane, for the design and scope critics: `story:project-reads-vendor-transcripts` asserts a default for an open `requires-stakeholder-input` question, and `story:call-seam-for-harness` amends invariant 9, a design change.

```findings
[
  {"file": ".engineering/planning/story/trace-ir-domain.md", "line": 124, "category": "acceptance", "severity": "blocker", "verdict": "needs-revision", "origin": "introduced", "message": "the acceptance is one sentence joining seven outcomes with semicolons (gate exit, scenarios executed, zero skipped, count gate and command list, coverage.md, amendment a26 with the runs-side-by-side pointer, CHANGELOG), so one can pass while another fails, and 'the new total' is never stated, so the count gate cannot be read off the acceptance"},
  {"file": ".engineering/planning/story/decision-returns-caller-produced-result.md", "line": 46, "category": "acceptance", "severity": "blocker", "verdict": "needs-revision", "origin": "introduced", "message": "the acceptance expects `tool.result` to carry `decided_by`, but that field is on `tool.decided` and `ToolResult` has none (crates/metaharness-protocol/src/event.rs:623, :632), so the stated check cannot pass as written"},
  {"file": ".engineering/planning/story/decision-returns-caller-produced-result.md", "line": 46, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "'the same answer is refused by name at run start' refuses at a moment before the answer exists, since the answer is given mid-run to a tool.requested, so the observable (spawn refusal, a tool.decided, or an error exit) is unnamed"},
  {"file": ".engineering/planning/story/confinement-of-vendor-kinds.md", "line": 48, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the acceptance names no outer confinement that the recorded vector runs inside, and session.started.hermetic has no write-surface or denied-path field today, so with Substrate excluded nobody can reproduce the 'attempted write outside the grant appears as a denial' check"},
  {"file": ".engineering/planning/story/aep-drive-disposition.md", "line": 44, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the section holds three alternative acceptance sets, each with several outcomes, and none applies until the blocker picks one; 'the 45-plus scenarios still pass' also names no number"}
]
```
