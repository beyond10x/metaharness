---
format: aep.planning-md/3
id: release-plan:release-0-11-0
kind: release-plan
status: implemented
title: Publish Metaharness 0.11.0 so takt can pin Claude Code 2.1.304 and the wave-3 changes
tags:
- release
refs:
- provider: atlas
  reference: adr-0098
relations:
- delivers: story:claude-code-2-1-304-declared
- delivers: story:ess-pin-0-66-0
- delivers: story:codex-strict-record-rows
revision: 3
transitions:
- {from: "draft", to: "implemented", at: "2026-10-21T12:00:25Z", actor: "human:timo", revision: 3}
---
## Why this release

takt pins metaharness: wave 4 declares Claude Code 2.1.304 and its built-in plugins (`story:claude-code-2-1-304-declared`),
lands the Codex record rows invariant 3 allows (`story:codex-strict-record-rows`), and moves the ESS pin
(`story:ess-pin-0-66-0`; Timo, 2026-10-20: each release moves the ESS pin to the newest ESS release). The
release shipped ESS 0.67.0, not the 0.66.0 this plan first named: ESS 0.67.0 (tag object `e7de924b`, commit
`4579d804`, issues 619 and 620 fixed) was released before the cut, and `b10x-chef-orchestrator` held the release
for it. Wave 3 (harness 0.15.1, the loop's cwd, tool-contract names) was unreleased on `main` and shipped here
too. Approved with wave 4 by `b10x-chef-orchestrator` under the standing exception.

## Version

`0.11.0`: new pins, new H1a and H7 behaviour, contract-name resolution; no removed surface; minor bump
under the pre-v1 scheme.

## Procedure (AGENTS.md § Releases and § Release publication identity; as run for 0.10.1)

1. After wave 4 closes on `main` with the whole gate green: one commit "Release 0.11.0" on `release/0.11.0`
   from that head: `CHANGELOG.md` (cut `## [Unreleased]` as `## [0.11.0] — <date>`), workspace version
   0.10.1 → 0.11.0, `Cargo.lock` workspace members, `README.md` status. The whole gate runs on it.
2. PR through the bot; after CI (Gate, MSRV 1.98, Security and privacy, Build documentation) the bot
   fast-forwards `main`; push output kept in a file.
3. Annotated tag `0.11.0` ("Metaharness 0.11.0") through the bot.
4. Release workflow: download the `release-*` artifacts by run id, check the four archive names and
   `SHA256SUMS`, extract both Linux archives and run `b10x-gates scan-artifact` on each executable.
5. Publish: `b10x-gates api --method POST --path /repos/beyond10x/metaharness/releases` (leading `/`), notes
   from a file; assets through `b10x-gates gh --repo <tree> -- release upload` with paths relative to the
   tree (no home paths in argv). Verify author, uploaders, tag object and digests.
6. Record the publication here and move to `implemented`; send the tag sha to `b10x-chef-orchestrator`.

## Not in this release

A paid run; crates.io or any other registry.
## Procedure as run (2026-10-21)

1. Managed tree `wt-0612033e7f95` from `main` `5df311b` (wave 4 closed), branch `release/0.11.0`.
2. Commit `16340162` "Release 0.11.0": both manifests `requires: ess 0.67.0`, CI `ess-cli` at `4579d804`;
   `spec/conformance.json` regenerated on ESS 0.67.0, byte-identical (103 scenarios, 3 authored, 0 refusals);
   `CHANGELOG.md` `[Unreleased]` rewritten to Keep a Changelog 1.1.0 and cut as `[0.11.0] — 2026-10-21`;
   workspace version 0.10.1 → 0.11.0, lockfile members, README status; three adversary tests that read the
   `[Unreleased]` prose now read `[0.11.0]` (`b10x-chef-orchestrator` accepted the changed locator in
   `adversary_b10x_loops_cwd`). Whole gate green: 80 test binaries, 989 passed, 0 failed, 50 ignored;
   conformance 103/103 on ESS 0.67.0.
3. https://github.com/beyond10x/metaharness/pull/38, checks Gate, MSRV 1.98, Security and privacy,
   Build documentation green; `main` fast-forwarded through the bot `5df311b..16340162`; PR merged
   2026-10-21T11:16:32Z.
4. Annotated tag `0.11.0` ("Metaharness 0.11.0"): object `967abdafbccdd619aab3cb7a1ac3d60b1b47cdd6`,
   commit `16340162d0d83a03faea0c3039cf695bd1f8286e`, tagger `b10x-bot[bot]`, pushed through the bot.
5. Release workflow run 39590850503 green; artifacts downloaded by run id; four archive names present;
   `sha256sum --check SHA256SUMS` exit 0; both Linux executables pass `b10x-gates scan-artifact` (x86_64
   sha256 `47904faf0fa388c524e938c7cdd4cdd2efa870856aded08494fbe2789db3148a`, aarch64
   `d4c58d466e72cf06c6f6d9cff4e71084a90f0392f02eab4066233681be37cfa5`); `metaharness --version` prints
   `metaharness 0.11.0`.
6. GitHub Release 419499202 created by `b10x-gates api --method POST --path
   /repos/beyond10x/metaharness/releases`, notes in the short shape (summary, Breaking 4, Changed 6, link to
   the CHANGELOG section); assets uploaded by `b10x-gates gh --repo <tree> -- release upload 0.11.0 <paths>`
   with tree-relative paths. Tag sha sent to `b10x-chef-orchestrator`, which verified the release
   independently. Release tree finished, archived and gc'd; `release/0.11.0` deleted locally (`-d`) and
   already absent on the remote.

## Publication record

| field | value |
|---|---|
| release | 419499202, https://github.com/beyond10x/metaharness/releases/tag/0.11.0, author `b10x-bot[bot]`, not draft, not prerelease, published 2026-10-21T11:54:07Z |
| tag | object `967abdaf…`, commit `16340162…` |
| `metaharness-0.11.0-aarch64-apple-darwin.tar.gz` | 8,777,723 bytes, `sha256:6e7de8a9f927f9bfe2837575221ce66a7381769c12d64b5e9603aec2c4932b60` |
| `metaharness-0.11.0-aarch64-unknown-linux-gnu.tar.gz` | 9,656,432 bytes, `sha256:c0ef8d7179be0779be94844ab03ce9475351f2bc9525681a820c59fc93327ba5` |
| `metaharness-0.11.0-x86_64-apple-darwin.tar.gz` | 9,318,376 bytes, `sha256:98e2302aa59aa1d8aba1c182f340a357e5bd290829301071ef42e141bda3d4e6` |
| `metaharness-0.11.0-x86_64-unknown-linux-gnu.tar.gz` | 10,304,796 bytes, `sha256:21ccd5ddd48131d0eefb93cccd21f50cb0bf0835f4710620f2883083ac80828f` |
| `SHA256SUMS` | 460 bytes, `sha256:4b245c9cb1030b59c3ea008606042ed6a466ab29cfac018a837595c9cd66169e` |

Every asset uploaded by `b10x-bot[bot]`; the archive digests equal `SHA256SUMS`.
