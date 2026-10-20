---
format: aep.planning-md/3
id: epic:layer-pins-follow-their-owners
kind: epic
status: draft
title: metaharness takes tools from tools and the in-house harness at its llm-based release
summary: Re-pin metaharness-tools onto the tools crates and the b10x adapter onto the harness release that builds on llm, each after its upstream release (Atlas ADR 0097, ADR 0095).
owner: metaharness
tags:
- pins
refs:
- provider: atlas
  reference: adr-0095
- provider: atlas
  reference: adr-0097
revision: 4
---
## Status

Draft, recorded 2026-10-20 from Atlas ADR 0097 (harness is kept, builds on llm and gives its tools to
`tools`) and ADR 0095 (`tools` holds agent tool contracts and their default implementations). Both
stories wait on releases in other repositories; each is blocked by a `dependency-blocker` that names
the release.

## Outcome

metaharness takes its tool layer from `tools` and its in-house harness from the harness release that
builds on llm, so its two cross-repository pins follow the layer owners ADR 0096 names (L1: harness
and tools; L0: llm) instead of the pre-ADR harness that carried both.

## Context

- `metaharness-tools` depends on `b10x-harness-tools` and `b10x-harness-wire` at harness rev
  `798325f03cf5a18df8fadb346d31b314826136ec` (0.13.3) (`crates/metaharness-tools/Cargo.toml:21-22`);
  `metaharness-b10x` depends on `b10x-harness-tools` and `b10x-harness-loop` at the same rev
  (`crates/metaharness-b10x/Cargo.toml:14-19`) and pins `PINNED_VERSIONS = ["0.13.3"]`,
  `HARNESS_REVISION = "798325f…"` (`src/lib.rs:80,87`). Invariant 10: harness is taken by git
  revision, never by `path`.
- ADR 0097 point 3: `harness-tools` and `harness-toolchain` become `tools` crates under the
  `tool-contract/1` profile (ADR 0095 phases T1 and T2), "metaharness's own tool surface, which today
  serves harness's tool code, follows them". `tools` holds no crate yet (ADR 0097 Context).
- ADR 0097 point 2: harness builds on llm 0.7.0; the four wire and credential crates go; the bridge
  from branch `spike/builds-on-llm-0.1.7` (`8989d138`, unmerged) is rebased. Consequences: "a
  consumer moves to the new tags in its own wave".
- The organization-level rule for the pin move is in AGENTS.md § Live-evaluating: build the binary at
  the pinned revision, verify with `--strict-version`, and record what the release changed in a
  research page (`docs/research/2026-09-15-b10x-harness-0.12.1-adapter-surface.md` is the pattern).
  Since 2026-10-03 the credential-free native fixture (`task:b10x-native-fixture`) qualifies a pin
  without a paid run.

## Parts

- `story:metaharness-tools-adopts-tool-contract`: the Apache-2.0 contract crate from tools 0.1.0
  (tag `a9bf6eb6` on `046dfb14`, released 2026-10-20); unblocked.
- `story:metaharness-tools-on-tools-crates`: the implementation crates; `dependency-blocker:tools-first-release`
  cleared 2026-10-20, now blocked by `decision-blocker:tools-implementation-licence` (the three
  implementation crates are `LicenseRef-B10x-Proprietary`; Timo decides).
- `story:b10x-adapter-on-harness-over-llm`: `dependency-blocker:harness-llm-release` cleared
  2026-10-20 (harness 0.14.0, tag `58cc4623` on `c50f4fd5`, llm 0.8.0); waits on the tools
  implementation story and the budget hold.

## Order

`story:metaharness-tools-on-tools-crates` lands first: it removes `b10x-harness-tools` from both
`Cargo.toml` files, so the b10x story moves one pin and `Cargo.lock` never holds two harness
revisions. The b10x story carries a `depends_on` edge to it.

## Done when

Neither `Cargo.toml` in this workspace names `b10x-harness-tools`, with `task check` green and the
`tools` release named in `CHANGELOG.md`; and the b10x adapter's loop pin names a harness release that
builds on llm, with `task check` green and the adapter-surface research page written. The research
page is the b10x story's alone.
