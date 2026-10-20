---
format: aep.planning-md/3
id: dependency-blocker:tools-first-release
kind: dependency-blocker
status: open
title: tools has published no crate holding the harness tool layer under tool-contract/1
owner: metaharness
refs:
- provider: atlas
  reference: adr-0097
relations:
- blocks: story:metaharness-tools-on-tools-crates
withholds: test_result
revision: 1
---
## Upstream requirement

A `tools` release (beyond10x/tools) holding the former `harness-tools` and `harness-toolchain` code as
crates under the `tool-contract/1` profile, with the `fs` and `code-editor` toolsets and a conformance
suite the crates pass (Atlas ADR 0095 points 1–3 and 5; ADR 0097 point 3). Recorded 2026-10-20.

## Why it is open

ADR 0097 Context: "`tools` holds no crate yet; its design names harness as the first adopter
(`tools/docs/design/tool-contracts.md:92`)". ADR 0097 Consequences: "`tools` T1 starts from
harness-tools rather than from an empty crate". Until a tag exists there is nothing to pin.

## What it withholds

A green `task check` against the new pin (`test_result`) for `story:metaharness-tools-on-tools-crates`.

## Clearing condition

Record here the `tools` tag, its git revision and the toolset versions it declares, then
`aep plan artifact move dependency-blocker:tools-first-release --to cleared`. A tag that holds the
contracts but not the crates does not clear it. Cross-store edges are not admitted by `aep`; the
reference stays in prose.
