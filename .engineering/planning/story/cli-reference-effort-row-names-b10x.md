---
format: aep.planning-md/3
id: story:cli-reference-effort-row-names-b10x
kind: story
status: active
title: The CLI reference's --effort row says b10x refuses it
tags:
- from-finding
relations:
- derived_from: review-result:wave-1104-u3-adversary-1
scope:
- confidence: inferred
  path: crates/metaharness-cli/tests/website_claims.rs
- confidence: cited
  path: website/docs/reference/cli.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-05T01:50:52Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-05T01:50:53Z", actor: "human:timo", revision: 3}
---
## Context

`website/docs/reference/cli.md` about line 100 says `--effort` is "passed through", but a b10x run
refuses it since amendment a64. Found out of scope by the adversary pass on
`story:actor-reaches-every-launch` (review-result:wave-1104-u3-adversary-1).

## Outcome

The CLI reference's `--effort` row names the kinds that take it and says b10x refuses it, as the
`--prompt-cache-ttl` row does.

## Acceptance

1. The `--effort` row of `website/docs/reference/cli.md` names b10x as refusing it, and a test that
   reads the row against `check_spec`'s refusal fails if the two disagree.
