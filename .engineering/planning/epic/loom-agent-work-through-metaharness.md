---
format: aep.planning-md/3
id: epic:loom-agent-work-through-metaharness
kind: epic
status: draft
title: Loom governs agent work through metaharness per tool call
summary: The six per-call governance gaps Atlas ADR 0098 lists for this repository, and the disposition of metaharness aep drive.
owner: metaharness
tags:
- governance
refs:
- provider: atlas
  reference: adr-0098
revision: 2
---
## Status

Draft, recorded 2026-10-20 from Atlas ADR 0098 (Loom calls llm for its decisions and metaharness for
agent work) and its Consequences for this repository. Owner's instruction of 2026-10-20: the gap
stories are drafted now and are not in this wave.

## Outcome

Loom's `AgentExecutor` (ADR 0098 point 2–3; built first in takt, ADR 0099 point 3) runs an agent
session through metaharness and Commission governs it per tool call on every kind, with the effect
performed by Commission and its result handed back to the harness. Until the gaps below close, the
unit of governance for agent work is the session and its proposed change (ADR 0098 Consequences),
and this repository says so rather than implying per-call governance it cannot deliver.

## Context, read from this repository at 0.9.3

- A caller can `allow`, `deny`, `replace` the input of, or `abstain` on a tool call
  (`crates/metaharness-protocol/src/command.rs:14-58`, `Decision`). No variant carries a result.
- The owned tool surface (`ToolSurface::Owned`, `crates/metaharness-protocol/src/spec.rs:136-146`) is
  served over MCP by `metaharness run --tool-surface owned` from `metaharness_tools::{Catalogue,
  LocalOperations, Server}` (`crates/metaharness-cli/src/lib.rs:67-69,300`): the three verbs
  `tool_search`, `describe`, `invoke` over operations metaharness itself performs. Nothing lets a
  caller supply the catalogue or perform the invocation.
- The b10x adapter runs in `DecisionMode::Observe` only and asserts it (`crates/metaharness-b10x/src/lib.rs:1-25`;
  AGENTS.md invariant 9); the loop's `--hooks` file is "the only content-level refusal this arm has"
  (`spec.rs:225-229`; `launch.rs:558-560`).
- One process runs one prompt; no adapter, the core or the CLI mentions resume
  (`grep -i resume` over `metaharness-claude/src/launch.rs`, `metaharness-codex/src/launch.rs`,
  `metaharness-cli/src/lib.rs`, `metaharness/src/run.rs`: no match, 2026-10-20).
- Vendor kinds get no confinement beyond their own (`spec.rs:425`); sandbox inversion was rejected
  for production on 2026-09-01 (`docs/ROADMAP.md` § 4, `docs/design/sandbox-inversion-v0.1.md`).
- `metaharness aep drive` (`crates/metaharness-aep`) is a governed-run driver inside this repository
  and overlaps Commission (ADR 0098 Consequences). Its future is `decision-blocker:aep-drive-future`.

## Parts

Six gap stories, one per gap ADR 0098 lists, one disposition story, and one statement:
`story:caller-supplied-catalogue-on-every-kind`, `story:decision-returns-caller-produced-result`,
`story:catalogue-changes-within-a-session`, `story:continuity-across-executor-calls`,
`story:call-seam-for-harness`, `story:confinement-of-vendor-kinds`, `story:aep-drive-disposition`,
`story:governance-unit-stated`. The last one is the sentence "until these gaps close, the unit of
governance for agent work is the session and its proposed change" written into AGENTS.md § What this
repository owns, so the repository says it rather than this epic alone.

## Order

The gap stories share `crates/metaharness/src/run.rs`, `crates/metaharness-protocol/src/spec.rs`,
the launch files, `crates/metaharness-tools/src/server.rs` and the protocol design page, so they are
serialized by `depends_on` edges in this order: caller-supplied catalogue (after
`story:metaharness-tools-on-tools-crates`) → decision returns result → catalogue changes within a
session → continuity → confinement; call seam after the b10x pin move and the catalogue; the
disposition story last, after `story:trace-ir-domain` and the call seam. Each story takes the next
free amendment number of `docs/design/metaharness-protocol-v0.1.md` at its own dispatch, never in
its draft.

## Boundaries

- metaharness never depends on loom-sdk; the session executor lives in Loom (ADR 0098 point 3).
- The hermetic floor, per-call decisions through the protocol, the preserved `metaharness.frame/1`
  reader and the credential rules (AGENTS.md § Safety envelope) are unchanged by every story here;
  a story that needs one changed amends its design document first (invariant 8).
- A story that changes AGENTS.md invariant 9 (`story:call-seam-for-harness`) is a design change
  and says so.

## Done when

Each gap story is implemented or rejected with its reason recorded, and `decision-blocker:aep-drive-future`
is cleared by the operator's answer.
