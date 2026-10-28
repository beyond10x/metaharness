---
format: aep.planning-md/3
id: story:actor-reaches-every-launch
kind: story
status: draft
title: Every launch carries --actor, or check_spec refuses it on the kinds that do not
summary: Codex and b10x accept --actor and drop it; aep drive passes it to Codex
relations:
- decomposes: epic:loom-agent-work-through-metaharness
revision: 1
---
## Outcome

`--actor` is either carried by every kind's launch or refused by `check_spec` on the kinds that do
not carry it; it is never accepted and dropped in silence.

## Context

- Adversary pass 1 on story:run-spec-document (review-result:d170-u1-adversary-1, finding 3):
  `spec.actor` is read only by the Claude launch (`crates/metaharness-claude/src/launch.rs`), so
  Codex and b10x accept `--actor` and drop it.
- `metaharness aep drive` passes `--actor` to Codex (`crates/metaharness-aep/src/drive.rs`,
  `codex_argv`), so refusing it there breaks that caller; the Codex launch carrying it is the likely
  answer. b10x has no in-code caller passing it.
- Until then `crates/metaharness/tests/adversary_check_spec_pass1.rs` asserts that `--actor` stays
  admitted on Codex and b10x, under this story's id.

## Acceptance

1. A Codex run with `--actor agent:x` sets the actor where the Claude launch sets it, observed in the
   launched environment or attestation, or `check_spec` refuses it naming `--actor`; the same for
   b10x.

## Scope

- `crates/metaharness-codex/src/launch.rs`, `crates/metaharness/src/builder.rs` (`b10x_launch`,
  `dropped_by_kind`).
