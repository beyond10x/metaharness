---
format: aep.planning-md/3
id: dependency-blocker:harness-llm-release
kind: dependency-blocker
status: cleared
title: No harness release builds on llm yet
owner: metaharness
refs:
- provider: atlas
  reference: adr-0097
relations:
- blocks: story:b10x-adapter-on-harness-over-llm
withholds: test_result
revision: 4
transitions:
- {from: "open", to: "cleared", at: "2026-10-20T06:40:47Z", actor: "human:timo", revision: 3}
---
## Upstream requirement

A harness release (beyond10x/harness) that builds on llm 0.7.0 with its four wire and credential
crates removed (Atlas ADR 0097 point 2), tagged and reachable from harness main (this repository's
invariant 10). Recorded 2026-10-20.

## Why it is open

ADR 0097 Context: harness 0.13.3 "has no llm dependency"; the move exists only on branch
`spike/builds-on-llm-0.1.7` (`8989d138`, not merged), to be rebased onto llm 0.7.0. The current pin is
0.13.3 at `798325f03cf5a18df8fadb346d31b314826136ec` (`crates/metaharness-b10x/src/lib.rs:80,87`).

## What it withholds

A green `task check` and a re-recorded golden loop against the new release (`test_result`) for
`story:b10x-adapter-on-harness-over-llm`.

## Clearing condition

Record here the harness tag, its revision, and the llm version it builds on, then
`aep plan artifact move dependency-blocker:harness-llm-release --to cleared`. Cross-store edges are
not admitted by `aep`; the reference stays in prose.

## Cleared, 2026-10-20

harness 0.14.0 is tagged: annotated tag object `58cc46236bb798f4a956a696d7818a03b7db84d3` peeling to
`c50f4fd597fa08a3e2aa4f224eeb96adadcae800`, reachable from harness `origin/main` (read from the
harness checkout after `git fetch --tags`, 2026-10-20; relayed by session `adr-changes`). At that
commit the workspace depends on `b10x-llm-core`, `b10x-llm-blocking` and `b10x-llm-credentials` at
llm tag `0.8.0`, and `crates/harness-messages`, `harness-responses`, `harness-http` and
`harness-credential` are absent (ADR 0097 point 2 done); `harness-tools`, `harness-toolchain`,
`harness-wire` and `harness-loop` are still present. No GitHub Release object existed for the tag
when this was read (`gh release view 0.14.0`: not found); the pin is by git revision (invariant 10),
so the tag suffices. The adapter move itself (`story:b10x-adapter-on-harness-over-llm`) waits for
`story:metaharness-tools-on-tools-crates` (blocked by `decision-blocker:tools-implementation-licence`)
and for the budget hold to lift.

Later the same day: harness 0.14.1 is tagged, object `9a3735d1079930577fd7afc92dab460c9a463f48` on `f109612eb184f34ce6f7cf72f25b89a928320c39`, reachable from main (verified from the checkout); `adr-changes` relays it as a Substrate 0.7.17 security fix. The adapter pins 0.14.1, not 0.14.0.
