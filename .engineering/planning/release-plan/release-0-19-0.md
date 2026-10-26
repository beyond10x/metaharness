---
format: aep.planning-md/3
id: release-plan:release-0-19-0
kind: release-plan
status: draft
title: 'Publish Metaharness 0.19.0: b10x-harness 0.20.0 and the caller seam'
summary: b10x pin 0.19.0 -> 0.20.0 (a60); frame/ask decide through --approve caller (a61)
revision: 1
---
## Why this release

`story:b10x-harness-pin-0-20-0` (a60): the b10x adapter moves from harness 0.19.0 to 0.20.0, the first
loop with `run --approve caller`. `story:b10x-caller-approval` (a61): under `--decisions frame` and
`ask`, metaharness decides each write and `run` the loop asks about, the metaharness half of atlas
ADR 0104 point 3.

`b10x-chef-orchestrator`, 2026-10-26 (dispatch-queue #8): "Admit 0.20.0 in metaharness ... plus ADR
0104 point 3 ... as a story in the same wave, then release per AGENTS.md".

## Version

`0.19.0`: a `b10x-harness` 0.19.0 binary now reads H9 `gap` and `--strict-version` refuses it, and a
frame-mode b10x run without a frame, the CLI default, is refused before the spawn. Both break a
caller; minor bump under the pre-v1 scheme, with Breaking lines.

## Procedure (AGENTS.md § Releases and § Release publication identity; as run for 0.18.0)

1. On `release/0.19.0` from `main`: the admission commit and one commit, "Release 0.19.0", holding
   `CHANGELOG.md`, the workspace version 0.18.0 → 0.19.0, the `Cargo.lock` workspace members, the
   `README.md` status line and this record. The whole gate runs on that commit.
2. Open the PR through the bot. After CI passes, the bot fast-forwards `main`, then writes and pushes
   the annotated tag `0.19.0`.
3. Publish:
   - Download the Release workflow's four archives and `SHA256SUMS` by run id and check them.
   - Run `b10x-gates scan-artifact` on both Linux executables; both must pass.
   - The bot publishes the GitHub Release and uploads the assets.

   No registry publishing.
4. Run `verify-release.sh metaharness 0.19.0` and write the announcement.
