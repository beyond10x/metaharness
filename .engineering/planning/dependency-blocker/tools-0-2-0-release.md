---
format: aep.planning-md/3
id: dependency-blocker:tools-0-2-0-release
kind: dependency-blocker
status: cleared
title: tools 0.2.0, the release whose contract documents are reachable without a proprietary crate, is not out yet
owner: metaharness
refs:
- provider: atlas
  reference: adr-0095
relations:
- blocks: story:metaharness-tools-on-tools-crates
withholds: test_result
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-20T09:16:34Z", actor: "human:timo", revision: 3}
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
## Cleared without the tag (2026-10-20)

Not cleared by tools 0.2.0, which is still untagged (tools `origin/main` = `046dfb14`, 0.1.0). Cleared
because nothing in this store waits on it any more: after the re-scope of wave 3 against harness
`main` `947d0e48`, `story:metaharness-tools-adopts-tool-contract` takes the contract-name mapping from
`b10x-harness-catalogue` with no tools dependency (decision D6), and
`story:metaharness-tools-on-tools-crates` is superseded by `story:b10x-adapter-on-harness-over-llm`,
whose harness pin brings the tools crates at whatever tools tag harness 0.15.0 pins (decision D5).
Both `blocks` edges were removed before the move. If a later story needs `b10x-tool-contract`
directly, it files a new blocker.
