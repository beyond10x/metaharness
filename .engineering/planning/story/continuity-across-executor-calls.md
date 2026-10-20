---
format: aep.planning-md/3
id: story:continuity-across-executor-calls
kind: story
status: draft
title: Continuity across executor calls
summary: A second AgentExecutor call continues the same agent session in the same scratch home instead of starting a process that knows nothing of the first.
owner: metaharness
tags:
- governance
refs:
- provider: atlas
  reference: adr-0098
relations:
- decomposes: epic:loom-agent-work-through-metaharness
- depends_on: story:catalogue-changes-within-a-session
scope:
- confidence: inferred
  path: crates/metaharness-claude/src/launch.rs
- confidence: inferred
  path: crates/metaharness-cli/src/lib.rs
- confidence: inferred
  path: crates/metaharness-codex/src/launch.rs
- confidence: inferred
  path: crates/metaharness-protocol/src/spec.rs
- confidence: inferred
  path: crates/metaharness/src/run.rs
- confidence: inferred
  path: docs/design/metaharness-protocol-v0.1.md
revision: 9
---
# Story: continuity across executor calls

## Scheduling

Draft. Not in the wave of 2026-10-20 (owner's instruction). Gap 4 of Atlas ADR 0098 Consequences.

## Outcome

Commission's `AgentExecutor::run` is called once per action (`loom-commission/src/ports/executor.rs:15`,
ADR 0098 Context). A second call continues the same agent session, with its context, instead of
starting a process that knows nothing of the first, so a multi-action case does not re-explain the
repository to the model at every step.

## Context

- One process runs one prompt: `metaharness-claude/src/launch.rs` builds `-p <prompt> --output-format
  stream-json --verbose ...` (lines 1043-1064) and nothing in the Claude or Codex launch, the core
  run loop or the CLI mentions resume (grep, 2026-10-20; ADR 0098 cites `launch.rs:1051-1056`).
- `metaharness aep drive resume` resumes the *driver's* run from `launch.json`, not the vendor session.
- Vendor claims, unverified here: Claude Code offers `--resume <session-id>` and `--continue`; Codex
  offers `codex exec resume`. Each must be driven against the pinned version before it is claimed
  (invariant 4), and the scratch config home has to survive between calls for either to work (the
  hermetic floor is a scratch home per run today).

## Acceptance

On Claude Code and on Codex, `metaharness run <kind> --continue <session reference>` starts a
process that continues the named session inside the same scratch home with the same attestation,
`session.started` names the resumed session id and the continuation, and the model's first turn
answers a question about the earlier turn's content, pinned by a recorded vector per kind against
the pinned vendor version; a vendor whose resume surface does not do this is a finding, not a pass.
Only `b10x` may refuse by name before spawn.

## Order

Depends on `story:catalogue-changes-within-a-session`, the last of the catalogue stories on
`crates/metaharness/src/run.rs`; this story and `story:caller-supplied-catalogue-on-every-kind` both
change `spec.rs`, both `launch.rs` files, `run.rs` and `crates/metaharness-cli/src/lib.rs`, so they
are never in one wave. `story:confinement-of-vendor-kinds` follows this story on the same files.

## Out of scope

- Keeping a vendor process alive between executor calls (a daemon). This story continues a session;
  it does not hold a process.

## Scope

- inferred: `crates/metaharness-protocol/src/spec.rs`, `crates/metaharness-claude/src/launch.rs`,
  `crates/metaharness-codex/src/launch.rs`, `crates/metaharness/src/run.rs`,
  `crates/metaharness-cli/src/lib.rs`, `docs/design/metaharness-protocol-v0.1.md` § 8.1 (the
  scratch home's lifetime).

## Ambiguities

- `requires-stakeholder-input` — who owns the scratch home between calls (the caller names it, or
  metaharness keeps it under a run directory). Default: the caller names it, so the hermetic claim
  stays per launch.
