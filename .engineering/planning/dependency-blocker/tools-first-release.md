---
format: aep.planning-md/3
id: dependency-blocker:tools-first-release
kind: dependency-blocker
status: cleared
title: tools has published no crate holding the harness tool layer under tool-contract/1
owner: metaharness
refs:
- provider: atlas
  reference: adr-0097
relations:
- blocks: story:metaharness-tools-on-tools-crates
withholds: test_result
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-20T05:15:50Z", actor: "human:timo", revision: 3}
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

## Cleared, 2026-10-20

tools 0.1.0 is released: annotated tag object `a9bf6eb6d32ab0a569586afe3a2dfc5000c26af6` peeling to
`046dfb14c36773b557d76de0de92e04ce23a388d`, https://github.com/beyond10x/tools/releases/tag/0.1.0,
published by b10x-bot[bot] at 2026-10-20T05:13:07Z (read from the tools checkout and `gh release
view`; relayed by session `adr-changes`). At that tag `crates/` holds `b10x-tool-contract`
(Apache-2.0), `b10x-tool-fs`, `b10x-tool-code-editor` and `b10x-tool-toolchain` (all three
`LicenseRef-B10x-Proprietary`), and `contracts/` holds the `code-editor/v1alpha1` contract with its
scenarios and fixtures. The release exists, so this blocker clears; what metaharness (Apache-2.0,
`Cargo.toml:21`) may depend on is `decision-blocker:tools-implementation-licence`.
