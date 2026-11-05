---
format: aep.planning-md/3
id: story:claude-attestation-names-actor
kind: story
status: active
title: The Claude attestation names the declared AEP_ACTOR
tags:
- from-finding
relations:
- derived_from: review-result:wave-1104-u3-adversary-1
scope:
- confidence: cited
  path: crates/metaharness-claude/src/launch.rs
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-05T01:50:52Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-05T01:50:52Z", actor: "human:timo", revision: 3}
---
## Context

`RunSpec::actor` (`crates/metaharness-protocol/src/spec.rs` about line 251) says the attestation
records the declared actor as imposed. The Claude launch sets `AEP_ACTOR` in the child from it
(`crates/metaharness-claude/src/launch.rs` about line 1522), but its H3 attestation row (about line
1927) names only the allowlisted keys. Found by the adversary pass on
`story:actor-reaches-every-launch` (review-result:wave-1104-u3-adversary-1); the Codex row is fixed
in that story, the Claude row's gap predates it.

## Outcome

A Claude run started with `--actor` has an imposed control in its attestation that names `AEP_ACTOR`.

## Acceptance

1. A Claude plan for a spec with `actor: agent:x` carries an H3 imposed control whose text names
   `AEP_ACTOR`; a plan without an actor does not.
