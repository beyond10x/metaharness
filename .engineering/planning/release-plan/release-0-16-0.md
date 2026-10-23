---
format: aep.planning-md/3
id: release-plan:release-0-16-0
kind: release-plan
status: draft
title: Publish Metaharness 0.16.0 so native Claude runs reach Glob and Grep and drive loads driver-steps/2 maps
revision: 1
---
## Why this release

Two defects reported by `ess-ui-templates` on 2026-10-23, both fixed on `main`:

- `story:claude-dir-list-search-unregistered`: Claude Code 2.1.305 and 2.1.309 offer `Glob` and `Grep`
  only when `--tools` or `--allowedTools` names them, so a native frame admitting `dir.list` or `search`
  reached no tool (a53).
- `story:aep-pin-0-76-0`: the linked AEP 0.68.1 reads `aep.driver-steps/1` only, so `metaharness aep
  drive` refused every store adopted with AEP 0.76.0 (a54).

`b10x-chef-orchestrator`, 2026-10-23: "Go on with aep-pin-0-76-0 and 0.16.0. Before the release: grep
takt, takt-trial, loom and ess-ui-templates frames for task.todo. If any of them admits it, list it as
Breaking in the 0.16.0 notes and tell that owner in one line."

The grep found no frame admitting `task.todo` in takt, the takt trial scratch, loom or
ess-ui-templates, so no owner is told. The refusal stays listed as Breaking for any other caller.

## Version

`0.16.0`: a native frame admitting `task.todo` is now refused before the spawn, and the AEP library
moves from 0.68.1 to 0.76.0, which changes what `--aep-binary` must report; minor bump under the
pre-v1 scheme.

## Procedure (AGENTS.md § Releases and § Release publication identity; as run for 0.15.0)

1. Make one commit, "Release 0.16.0", on `release/0.16.0` from the integration head. It holds:
   - `CHANGELOG.md`
   - the workspace version, 0.15.0 → 0.16.0
   - the `Cargo.lock` workspace members
   - the `README.md` status line
   - this record

   The whole gate runs on that commit.
2. Open the PR through the bot. After CI passes (Gate, MSRV 1.98), the bot fast-forwards `main`, then
   writes and pushes the annotated tag `0.16.0`.
3. Publish:
   - Download the Release workflow's four archives and `SHA256SUMS` by run id and check them.
   - Run `b10x-gates scan-artifact` on both Linux executables; both must pass.
   - The bot publishes the GitHub Release and uploads the assets.

   No registry publishing.
4. Run `verify-release.sh metaharness 0.16.0`, write the announcement, and ask `brain-main` to post it.
