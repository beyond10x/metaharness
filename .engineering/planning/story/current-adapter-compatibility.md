---
format: aep.planning-md/3
id: story:current-adapter-compatibility
kind: story
status: draft
title: Verify installed adapter releases before advancing pins
owner: metaharness
refs:
- provider: github
  reference: beyond10x/metaharness#15
relations:
- decomposes: epic:github-issue-repair
- depends_on: story:governed-codex
- depends_on: story:scripted-b10x-run-does-not-need-the-binary
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/metaharness-b10x
- confidence: inferred
  path: crates/metaharness-claude
- confidence: inferred
  path: crates/metaharness-codex
- confidence: inferred
  path: docs/research
revision: 4
---
## Context

Installed banners are Claude 2.1.288, Codex 0.153.4 and b10x-harness 0.13.3; current pins are older. Issue #15 calls existing success observations insufficient for general compatibility. Each version claim must be supported by the corresponding observed surface; retained private transcripts never enter Git.

## Acceptance

For each installed release explicitly named here—Claude 2.1.288, Codex 0.153.4 and b10x-harness 0.13.3—the compatibility-success, compatibility-terminal-failure, compatibility-tool-decision, compatibility-cancellation, compatibility-model-usage and compatibility-declared-controls matrix records a disposition backed by sanitized observed evidence and synthetic regression vectors, keeps untested claims explicitly unverified, and advances a pin only when its required evidence is complete; an empty proposed-pin set does not satisfy this acceptance.

## Scope and sequence

Adapter crates metaharness-claude, metaharness-codex and metaharness-b10x own claims and synthetic vectors. Core spawn vector tests may change after scope confirmation. docs/research holds sanitized method, binary versions and measured conclusions; CHANGELOG records any pin change, Cargo.lock only if a deliberate source revision is required. Do not change pins to suppress drift warnings. Native b10x evidence does not establish governed execution. Runs after #11/#13 and scripted launch repairs to avoid concurrent adapter/core edits.

## Verification and resources

Offline suites first. Short live probes run sequentially in private scratch with a shared explicit operator spending limit, no private repository task, and no unbounded model loop. Budget requested while offline work proceeds; a missing answer does not authorize spending. Record each command, version, time bound, cost if observed and exact unsupported scope. If a required surface cannot be exercised, retain its older pin/unverified label and record the unresolved issue honestly.
