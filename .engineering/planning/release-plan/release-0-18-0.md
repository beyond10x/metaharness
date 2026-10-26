---
format: aep.planning-md/3
id: release-plan:release-0-18-0
kind: release-plan
status: draft
title: Publish Metaharness 0.18.0 so the b10x adapter admits b10x-harness 0.19.0
summary: b10x pin 0.18.0 -> 0.19.0 (a59); ESS 0.69.2
revision: 1
---
## Why this release

`story:b10x-harness-pin-0-19-0` (a59): the b10x adapter moves from harness 0.18.0 to 0.19.0, so a run
against the released loop no longer reads H9 `gap` or stops at `--strict-version`. The ESS pin moves
to 0.69.2, the newest ESS release.

`b10x-chef-orchestrator`, 2026-10-26: "metaharness admits b10x-harness 0.19.0 ... full gate, bot
commit/PR/fast-forward per AGENTS.md, then release per AGENTS.md § Releases".

## Version

`0.18.0`: a `b10x-harness` 0.18.0 binary now reads H9 `gap` and `--strict-version` refuses it, which
breaks a caller still on that binary; minor bump under the pre-v1 scheme, with a Breaking line.

## Procedure (AGENTS.md § Releases and § Release publication identity; as run for 0.17.0)

1. On `release/0.18.0` from `main`: the ESS 0.69.2 commit, the admission commit and one commit,
   "Release 0.18.0", holding `CHANGELOG.md`, the workspace version 0.17.0 → 0.18.0, the `Cargo.lock`
   workspace members, the `README.md` status line and this record. The whole gate runs on that commit.
2. Open the PR through the bot. After CI passes (Gate, MSRV 1.98), the bot fast-forwards `main`, then
   writes and pushes the annotated tag `0.18.0`.
3. Publish:
   - Download the Release workflow's four archives and `SHA256SUMS` by run id and check them.
   - Run `b10x-gates scan-artifact` on both Linux executables; both must pass.
   - The bot publishes the GitHub Release and uploads the assets.

   No registry publishing.
4. Run `verify-release.sh metaharness 0.18.0`, write the announcement, and ask `brain-main` to post it.
