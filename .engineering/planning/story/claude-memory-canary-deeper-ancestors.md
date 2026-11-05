---
format: aep.planning-md/3
id: story:claude-memory-canary-deeper-ancestors
kind: story
status: proposed
title: The Claude memory canary measures ancestors above the cwd's parent
summary: Native lane plants the six memory shapes at deeper ancestor levels, including a home-shaped .claude/CLAUDE.md, so H11 reads them from a measurement
tags:
- from-finding
relations:
- informed_by: story:h11-not-ok-on-unprobed-memory
scope:
- confidence: inferred
  path: crates/metaharness-claude/src/lib.rs
- confidence: inferred
  path: crates/metaharness-claude/tests/native_fixture.rs
- confidence: inferred
  path: docs/design/metaharness-protocol-v0.1.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-05T11:38:12Z", actor: "human:timo", revision: 5}
---
## Scheduling

Filed on 2026-11-05 while the 0.30.0 wave was running, and accepted for the next metaharness wave. It comes
from the U2 run of `story:h11-not-ok-on-unprobed-memory`, which was held out of 0.30.0. That story depends on
this one and lands after it.

## Outcome

The Claude memory canary measures memory files in every ancestor level that a scratch cwd has on a real
machine, not only in the cwd and its parent. That includes a home-shaped `.claude/CLAUDE.md` several levels
above the cwd, which is what the operator's own `~/.claude/CLAUDE.md` looks like when `TMPDIR` sits under
`HOME`. H11 can then read `ok` for those surfaces from a measurement, not from inference.

## Context

Measured in the held U2 tree (`impl/h11-unprobed-memory`, patch kept in the wave's scratch):

- `TMPDIR` is under `HOME`, so the walk finds `~/.claude/CLAUDE.md` as a `.claude/CLAUDE.md` four levels
  above every scratch cwd.
- `native_memory_canaries_in_the_cwd_and_an_ancestor_match_the_recorded_row`
  (`crates/metaharness-claude/tests/native_fixture.rs`) plants canaries only in the cwd and one ancestor. It
  also asserts that no memory file sits above the evidence root.
- So the surfaces above the parent are unmeasured. Reading them `unk` (the held route) turns every strict
  Claude run on such a machine into exit 3, which is what a36 § 4 reverted.

## Acceptance

- expected_before: `MEMORY_CANARIES` rows record canaries in the cwd and its parent only. The native lane
  refuses a world with memory files above the evidence root.
- After:
  - The native lane plants the six shapes at several ancestor depths above the parent, including a
    `.claude/CLAUDE.md` at a home-shaped level.
  - It runs with the real pinned `claude`, in a loopback-only network namespace, against the stub provider,
    with no credential.
  - The row of each pinned version records the planted count, what the control sent, and what reached a
    request through the launch, per depth.
  - The floor reads those depths from the row.
  - A design amendment states the measured depths and what stays unprobed.
- No paid run, no real login, no network beyond loopback.
