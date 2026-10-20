---
format: aep.planning-md/3
id: story:b10x-adapter-on-harness-over-llm
kind: story
status: draft
title: The b10x adapter pins the harness release that builds on llm
summary: HARNESS_REVISION, PINNED_VERSIONS and the three git pins move to the llm-based harness release, qualified with the credential-free native fixture and a research page.
owner: metaharness
tags:
- pins
refs:
- provider: atlas
  reference: adr-0097
relations:
- decomposes: epic:layer-pins-follow-their-owners
- depends_on: story:metaharness-tools-on-tools-crates
- depends_on: story:trace-ir-domain
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: crates/metaharness-aep-eval/src/lib.rs
- confidence: inferred
  path: crates/metaharness-b10x/Cargo.toml
- confidence: inferred
  path: crates/metaharness-b10x/fixtures
- confidence: inferred
  path: crates/metaharness-b10x/src/lib.rs
- confidence: inferred
  path: crates/metaharness-b10x/src/seam.rs
- confidence: inferred
  path: crates/metaharness-b10x/src/vectors.rs
- confidence: inferred
  path: docs/research
revision: 11
---
# Story: the b10x adapter pins the harness release that builds on llm

## Scheduling

Draft. Blocked by `dependency-blocker:harness-llm-release`. Not in the wave of 2026-10-20.

## Outcome

`metaharness-b10x` observes the in-house harness at the release where it builds on llm (ADR 0097
point 2), and every claim the adapter makes about that binary is pinned to that version and verified
against it or labelled unverified (invariant 4).

## Context

- The pin today: `PINNED_VERSIONS = ["0.13.3"]`, `HARNESS_REVISION = "798325f03cf5a18df8fadb346d31b314826136ec"`
  (`crates/metaharness-b10x/src/lib.rs:80,87`); two git pins in `crates/metaharness-b10x/Cargo.toml:14-19`,
  `b10x-harness-tools` and `b10x-harness-loop`, at the same rev; `b10x-harness-loop` is taken for
  `micro_usd_as_decimal` so the cost figure on the wire is rendered by the loop's own function. The
  `b10x-harness-tools` pin leaves with `story:metaharness-tools-on-tools-crates`; this story moves
  the loop pin.
- What changes when harness builds on llm: the wire and credential crates go (`harness-messages`,
  `harness-responses`, `harness-http`, `harness-credential`, ADR 0097 Context), so
  `credential_source`, the `usage` shape (llm's `cache_creation_input_tokens` and siblings) and cost
  rendering may move; each is read from the new record, not assumed (the 0.12.1 pin move found two
  claims that reading the code did not show: AGENTS.md § Live-evaluating, last bullet).
- The pin-move protocol is AGENTS.md § Live-evaluating: build the binary at the pinned revision,
  `--strict-version` refuses before spawn when the resolved binary is off the pin, and the research
  page records the surface (`docs/research/2026-09-15-b10x-harness-0.12.1-adapter-surface.md`).
  Since 2026-10-03 the credential-free native fixture (`task:b10x-native-fixture`, `docs/research/
  2026-10-03-native-codex-fixture.md` pattern) qualifies a pin without a paid run; a paid probe is
  outside the standing approval for this work and would need its own spend authorization.

## Acceptance

`task check` exits 0 with `HARNESS_REVISION`, `PINNED_VERSIONS` and the `b10x-harness-loop` git pin
in `crates/metaharness-b10x/Cargo.toml` naming one harness release that builds on llm and is
reachable from harness main; the recorded golden loop and version-pair vectors are re-recorded
against that binary and the `metaharness.trace` normalize scenario over the b10x golden record still
passes; the credential-free native fixture passes against it; `docs/research/<date>-b10x-harness-
<version>-adapter-surface.md` lists every adapter claim as verified or unverified against that
release; `CHANGELOG.md` names the pin move and the research page.

## Order

Depends on `story:metaharness-tools-on-tools-crates`, which removes `b10x-harness-tools` from
`crates/metaharness-b10x/Cargo.toml` first, so this story moves one pin (the loop) and never leaves
two harness revisions in `Cargo.lock`; depends on `story:trace-ir-domain`, whose scenarios read the
golden record this story re-records. `story:call-seam-for-harness` follows this story on
`crates/metaharness-b10x/src/{lib,seam,vectors}.rs`.

## Out of scope

- The tool crates pin (`story:metaharness-tools-on-tools-crates`).
- Any paid run. A probe that spends requires `METAHARNESS_LIVE=1`, an explicit budget and the
  operator's spend authorization (invariant 5; standing approval excludes spending).

## Scope

- cited: `crates/metaharness-b10x/Cargo.toml`, `crates/metaharness-b10x/src/lib.rs`,
  `crates/metaharness-b10x/src/vectors.rs`, `crates/metaharness-b10x/fixtures/`, `Cargo.lock`,
  `CHANGELOG.md`, `docs/research/`.
- inferred: `crates/metaharness-b10x/src/seam.rs` (usage and cost mapping), `crates/metaharness-aep-eval/src/lib.rs`
  (the release identity the eval requires before trusting the binary).

## External prerequisite

A harness release tag that builds on llm 0.7.0 (ADR 0097 point 2; the spike is `8989d138` on
`spike/builds-on-llm-0.1.7`, unmerged). The blocker carries the reference in prose.
