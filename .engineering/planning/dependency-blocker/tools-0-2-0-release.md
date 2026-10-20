---
format: aep.planning-md/3
id: dependency-blocker:tools-0-2-0-release
kind: dependency-blocker
status: open
title: tools 0.2.0, the release whose contract documents are reachable without a proprietary crate, is not out yet
owner: metaharness
refs:
- provider: atlas
  reference: adr-0095
relations:
- blocks: story:metaharness-tools-adopts-tool-contract
- blocks: story:metaharness-tools-on-tools-crates
withholds: test_result
revision: 1
---
## Upstream requirement

tools 0.2.0: the release in which the contract documents (`contracts/*/v1alpha1/contract.yaml`
and their command names and schemas) are reachable without a proprietary implementation crate,
through `b10x-tool-contract` (Apache-2.0). Recorded 2026-10-20 from `adr-changes`: "tools found that
the contract YAMLs are embedded only by the proprietary crates, so b10x-tool-contract alone gives
you no command names or schemas. tools is now running story:contract-documents-without-an-implementation
and releases 0.2.0 after it; pin tools 0.2.0 for the tool-contract adoption."

## What it withholds

A green `task check` against the new pin (`test_result`) for `story:metaharness-tools-adopts-tool-contract`.

## Clearing condition

Record here the tools 0.2.0 tag object, its commit and what `b10x-tool-contract` exposes at that
tag (read from the tools checkout), then move to `cleared`. Cross-store edges are not admitted by
`aep`; the reference stays in prose.
