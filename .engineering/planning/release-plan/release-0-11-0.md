---
format: aep.planning-md/3
id: release-plan:release-0-11-0
kind: release-plan
status: draft
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
revision: 1
---
## Why this release

takt pins metaharness: wave 4 declares Claude Code 2.1.304 and its built-in plugins (`story:claude-code-2-1-304-declared`),
lands the Codex record rows invariant 3 allows (`story:codex-strict-record-rows`), and moves the ESS pin to
0.66.0 (`story:ess-pin-0-66-0`; Timo, 2026-10-20: each release moves the ESS pin to the newest ESS
release). Wave 3 (harness 0.15.1, the loop's cwd, tool-contract names) is unreleased on `main` and ships
here too. Approved with wave 4 by `b10x-chef-orchestrator` under the standing exception.

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
