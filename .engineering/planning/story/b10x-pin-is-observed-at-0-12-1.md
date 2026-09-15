---
format: aep.planning-md/1
id: story:b10x-pin-is-observed-at-0-12-1
kind: story
status: implemented
title: The b10x adapter's provenance pair names harness 0.12.1, observed
summary: Move HARNESS_REVISION and PINNED_VERSIONS from 0.10.2/c1493a7 to 0.12.1/90f10a4 on the evidence of one paid metaharness run b10x, and carry what that run found.
owner: metaharness
tags:
- adapter
- eval
relations:
- decomposes: epic:runs-side-by-side
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/metaharness-b10x/src/lib.rs
- confidence: cited
  path: crates/metaharness-b10x/src/seam.rs
- confidence: cited
  path: crates/metaharness/fixtures/golden/contract-result-b10x.json
- confidence: cited
  path: docs/research/2026-09-15-b10x-harness-0.12.1-adapter-surface.md
- confidence: cited
  path: evals/aep/README.md
revision: 7
---
# Story: the b10x adapter's provenance pair names harness 0.12.1, observed

## Outcome

`metaharness-b10x::PINNED_VERSIONS` reads `0.12.1` and `HARNESS_REVISION` reads
`90f10a4314c1c630691c85e812bd8d5d23d73fcc`, and a dated observation of that binary is what they
rest on rather than an edit.

## Context

The pair is the AEP eval's provenance check for an *installed* `b10x-harness`
(`crates/metaharness-aep-eval/src/lib.rs`, `validate_native_pin`), not the Cargo build pin. When the
Cargo pin moved `0.11.1 → 0.12.1` on 2026-09-15 this pair was deliberately left at `0.10.2`, because
moving it without re-observing would have made invariant 4 a claim about a binary nobody had read,
and re-observing costs a paid run. Org-state review 2026-09-15, decision 13, chose the run.

## Acceptance

- `metaharness doctor b10x` against an installed `0.12.1` exits 0 on both rows, including that
  every flag the adapter emits is one the binary declares.
- One paid `metaharness run b10x --strict-version` against a `0.12.1` binary built from
  `90f10a4` completes: `--strict-version` refuses before the spawn when the resolved binary is off
  the pin, so exit 0 is the provenance rather than a report about it.
- The stream carries **zero `opaque`** lines — every record kind that run emitted is one the
  adapter reads — and the terminal event is the one the adapter documents.
- What the run found is carried, not just its verdict: `usage.cache_creation_input_tokens` is read
  instead of hardcoded `None`, and the retired standing fact that this harness has no MCP client is
  corrected where it was asserted.
- What the run did **not** establish is labelled unverified in
  `docs/research/2026-09-15-b10x-harness-0.12.1-adapter-surface.md` — the golden sample is still
  0.9.1's bytes, and no confinement, withholding, skills or MCP mapping was exercised.
- `task check` green.
