---
format: aep.planning-md/3
id: release-plan:release-0-22-0
kind: release-plan
status: draft
title: 'Publish Metaharness 0.22.0: metaharness::qualify'
summary: 'New public API qualify: the floor rows a binary version decides, with no model call'
revision: 1
---
## Why this release

`story:qualify-a-binary-without-a-model-call`: metaharness gains `metaharness::qualify`, which reports
the floor rows a binary's version decides (H9 for every kind, H11 for Claude) from one `--version`
and no model call, and says whether `--strict-version` would refuse the binary.

## Version

`0.22.0`: a new public API, `qualify`, with no change to an existing one; minor bump under the pre-v1
scheme.

## Procedure (AGENTS.md § Releases and § Release publication identity; as run for 0.21.0)

1. On `unit/tr-qualify`, which already merges `main` at 0.21.0: "Release 0.22.0" holding
   `CHANGELOG.md`, the workspace version 0.21.0 to 0.22.0, the `Cargo.lock` workspace members, the
   `README.md` status line and this record. The whole gate runs on that head.
2. Open the PR through the bot. After CI passes, the bot fast-forwards `main`, then writes and pushes
   the annotated tag `0.22.0`.
3. Publish: download the Release workflow's four archives and `SHA256SUMS` by run id and check them;
   `b10x-gates scan-artifact` on both Linux executables; the bot publishes the GitHub Release and
   uploads the assets. No registry publishing.
4. Run `verify-release.sh metaharness 0.22.0` and write the announcement.
