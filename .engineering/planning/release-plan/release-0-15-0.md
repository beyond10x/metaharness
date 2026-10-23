---
format: aep.planning-md/3
id: release-plan:release-0-15-0
kind: release-plan
status: draft
title: Publish Metaharness 0.15.0 so takt can pin Claude Code 2.1.309 and Codex 0.160.0
revision: 1
---
## Why this release

The installed Claude Code is 2.1.309 and the installed Codex is 0.160.0, and metaharness 0.14.0 accepts
only Claude Code 2.1.304/2.1.305 and Codex 0.153.4. takt's trial on 2026-10-23 had to run on a
checksum-verified older binary. takt pins the release that accepts both.

`b10x-chef-orchestrator`, 2026-10-23: "qualify the installed harness versions and release: Claude Code
2.1.309 and Codex 0.160.0 … Qualify by the same bounded native qualification as a33 (credentials none and
loopback), add the pins beside the old ones, cut a metaharness release … Approved under the grant in
~/beyond10x/CLAUDE.md (class C: dependency upgrade, commit, PR, merge, release)."

## Version

`0.15.0`: both `PINNED_VERSIONS` arrays change length, and Codex refuses `--worktree`; minor bump under
the pre-v1 scheme. The specification and gate move to ESS 0.68.0, the newest ESS release.

Not in this release: `story:aep-pin-0-76-0` and `story:claude-dir-list-search-unregistered`, both
reported by `ess-ui-templates` on 2026-10-23. The orchestrator said to put the AEP pin move into 0.15.0
"if it fits, else the next release". It moves 206 AEP commits, so it goes to the next release.

## Procedure (AGENTS.md § Releases and § Release publication identity; as run for 0.14.0)

1. Make one commit, "Release 0.15.0", on `release/0.15.0` from the integration head. It holds:
   - `CHANGELOG.md`
   - the workspace version, 0.14.0 → 0.15.0
   - the `Cargo.lock` workspace members
   - the `README.md` status line
   - this record

   The whole gate runs on that commit.
2. Open the PR through the bot. After CI passes (Gate, MSRV 1.98), the bot fast-forwards `main`, then
   writes and pushes the annotated tag `0.15.0`.
3. Publish:
   - Download the Release workflow's four archives and `SHA256SUMS` by run id and check them.
   - Run `b10x-gates scan-artifact` on both Linux executables; both must pass.
   - The bot publishes the GitHub Release and uploads the assets.

   No registry publishing.
4. Run `verify-release.sh metaharness 0.15.0`, write the announcement, and ask `brain-main` to post it.
