---
format: aep.planning-md/3
id: release-plan:release-0-17-0
kind: release-plan
status: draft
title: Publish Metaharness 0.17.0 so a driven step is told the commands its AEP 0.77.0 map declares
summary: aep drive links AEP 0.77.0; per-step commands reach the prompt (a56)
revision: 2
---
## Why this release

`story:aep-pin-0-77-0`, on `main` (a56): the AEP library moves from 0.76.0 to 0.77.0, so a step map's
`commands:` reaches `metaharness aep drive`, and a step that declares commands is told them instead of
"not `cargo`".

`b10x-chef-orchestrator`, 2026-10-23: "aep 0.77.0: A, approved under the grant … Bump metaharness-aep,
render per-step commands, correct the b10x-arm "Not `cargo`" line to the 0.77.0 checks map, then
release through your process (verify-release, announce)."

## Version

`0.17.0`: `--aep-binary` and the eval's child `aep` must now report 0.77.0, and a step that declares
`commands:` is held to them; minor bump under the pre-v1 scheme.

## Procedure (AGENTS.md § Releases and § Release publication identity; as run for 0.16.0)

1. Make one commit, "Release 0.17.0", on `release/0.17.0` from `main`. It holds:
   - `CHANGELOG.md`
   - the workspace version, 0.16.0 → 0.17.0
   - the `Cargo.lock` workspace members
   - the `README.md` status line
   - this record

   The whole gate runs on that commit.
2. Open the PR through the bot. After CI passes (Gate, MSRV 1.98), the bot fast-forwards `main`, then
   writes and pushes the annotated tag `0.17.0`.
3. Publish:
   - Download the Release workflow's four archives and `SHA256SUMS` by run id and check them.
   - Run `b10x-gates scan-artifact` on both Linux executables; both must pass.
   - The bot publishes the GitHub Release and uploads the assets.

   No registry publishing.
4. Run `verify-release.sh metaharness 0.17.0`, write the announcement, and ask `brain-main` to post it.
