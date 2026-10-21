---
format: aep.planning-md/3
id: decision-blocker:tools-implementation-licence
kind: decision-blocker
status: cleared
title: Timo decides whether Apache-2.0 metaharness may adopt the proprietary tools implementation crates
owner: metaharness
refs:
- provider: atlas
  reference: adr-0095
- provider: atlas
  reference: adr-0097
relations:
- blocks: story:metaharness-tools-on-tools-crates
revision: 4
transitions:
- {from: "open", to: "cleared", at: "2026-10-20T07:51:33Z", actor: "human:timo", revision: 3}
---
## The question

metaharness is Apache-2.0 (`Cargo.toml:21`) and ships a binary. tools 0.1.0 (tag `a9bf6eb6`,
commit `046dfb14`) publishes the implementations `b10x-tool-fs`, `b10x-tool-code-editor` and
`b10x-tool-toolchain` under `LicenseRef-B10x-Proprietary`, and only `b10x-tool-contract` under
Apache-2.0 (read from each crate's `Cargo.toml` at the tag, 2026-10-20). Whether metaharness may link
the proprietary crates is Timo's licence decision; `adr-changes` relayed on 2026-10-20: "adopt only
tool-contract until Timo's licence decision". Recorded 2026-10-20. Decides: Timo.

## Options

| | option | what it does | what it costs |
|---|---|---|---|
| A | relicense the three implementation crates to Apache-2.0 | metaharness adopts them as ADR 0097 point 3 intends and drops `b10x-harness-tools` | tools gives up the proprietary terms on the fs, code-editor and toolchain code |
| B | metaharness links the proprietary crates as they are | the adoption proceeds | the Apache-2.0 binary carries proprietary code: a licence grant to metaharness and every downstream (the published release archives) has to exist and be stated; the public repository's licence line stops being the whole truth |
| C | metaharness adopts the contract crate only and keeps its implementations (harness-tools at the pinned revision, or its own) | `story:metaharness-tools-adopts-tool-contract` lands now; the one path-containment check stays in two places | ADR 0097 point 3's "metaharness's own tool surface follows them" stays open; two implementations of the tool layer remain |

No recommendation is acted on. Until the answer, C is what the store does: the contract story is
unblocked, the implementation story is blocked here.

## What it blocks

`story:metaharness-tools-on-tools-crates` (the implementation crates). It does not block
`story:metaharness-tools-adopts-tool-contract`.

## Decision

Resolved 2026-10-20 (about 10:10): option A. Timo's decision, relayed by session `adr-changes`:
"decision-blocker:tools-implementation-licence is resolved by Timo (~10:10): option A, the tools
crates moved from harness become Apache-2.0 in tools 0.2.0." So `b10x-tool-fs`, `b10x-tool-code-editor`
and `b10x-tool-toolchain` are Apache-2.0 from tools 0.2.0 on, and metaharness adopts them at that
tag; until 0.2.0 is out `dependency-blocker:tools-0-2-0-release` holds both tools stories. Cleared
on this answer.
## Relicence landed

Recorded 2026-10-21 during wave 4: tools commit `d5614d87` makes `b10x-tool-fs`, `b10x-tool-code-editor` and
`b10x-tool-toolchain` Apache-2.0 from tools 0.2.0 on, as option A said. Adoption by metaharness is a wave 5
candidate.
