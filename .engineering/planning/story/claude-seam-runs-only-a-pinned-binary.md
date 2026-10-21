---
format: aep.planning-md/3
id: story:claude-seam-runs-only-a-pinned-binary
kind: story
status: draft
title: The Claude seam runs only a binary whose onFailure reading is recorded
summary: With the decision hook installed, an off-pin Claude binary is refused by name before the spawn.
owner: metaharness
refs:
- provider: atlas
  reference: adr-0096
relations:
- informed_by: review-result:adr0096-w6u1-review
- informed_by: story:claude-seam-fails-closed
revision: 1
---
# Story: the Claude seam runs only a binary whose onFailure reading is recorded

## Scheduling

Draft, filed 2026-10-21 after release 0.12.0 from `review-result:adr0096-w6u1-review` (s6-2).

## Outcome

A Claude run that installs the decision hook launches only a binary whose version is one amendment a41
records an `onFailure` reading for, and refuses any other binary by name before the spawn.

## Context

- Refusing a call on hook start failure, timeout or an unreadable answer rests on the vendor honouring
  `onFailure: "block"` (`crates/metaharness-claude/src/launch.rs:1638-1646`, at `cacb923`).
- `strict_version` defaults to false (`crates/metaharness-claude/src/builder.rs:2261-2263`, `:888-894`;
  `crates/metaharness-protocol/src/spec.rs:600`, at `cacb923`); nothing else checks the binary against the
  pins, so a binary off the pins may ignore the key (a41 § 4).
- Verified on the metaharness side.

## Acceptance

- expected_before: today a Claude run with the hook installed spawns whatever binary resolves, on or off the
  pins, unless `--strict-version` is passed. After: with the hook installed, an off-pin binary is refused
  before the spawn with a message naming the resolved version and the pinned ones; a pinned binary runs as
  before; a test drives both cases through the builder.

## Scope

Inferred: `crates/metaharness-claude/src/builder.rs`, `crates/metaharness-claude/src/launch.rs`,
`crates/metaharness-protocol/src/spec.rs`, `docs/design/metaharness-protocol-v0.1.md`, `CHANGELOG.md`.
