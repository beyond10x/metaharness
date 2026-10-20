---
format: aep.planning-md/3
id: release-plan:release-0-10-0
kind: release-plan
status: draft
title: Publish Metaharness 0.10.0 so takt can pin the strict scratch-cwd admission and the trace domain
owner: metaharness
tags:
- release
refs:
- provider: atlas
  reference: adr-0098
relations:
- delivers: story:strict-hermetic-with-embedder-scratch-cwd
- delivers: epic:session-format
revision: 1
---
## Why this release

takt's session-executor crate S2 (Atlas ADR 0098 point 2, ADR 0099 point 3) waits on
`story:strict-hermetic-with-embedder-scratch-cwd` (takt gap G15) and needs a metaharness release
it can pin. The wave of 2026-10-20 also left unreleased work on `main` under `## [Unreleased]`
in `CHANGELOG.md`: the ESS domain `metaharness.trace` (ADR 0096 point 7) and the AGENTS.md
statements. Requested by session `adr-changes` on 2026-10-20 under the owner's standing exception
for ADR 0096–0101 work, which admits "a release through the repository's own process when another
repository needs to pin it".

## Version

`0.10.0`: a new ESS domain, a new `--hermetic strict` admission and the `project --from` verb are
features over 0.9.3 with no removed surface; minor bump under the pre-v1 scheme the changelog uses.
Decided by the coordinator; the approver may name another number.

## Procedure (AGENTS.md § Releases, read 2026-10-20)

1. After the wave of G15 closes on `main` with the full gate green: one commit "Release 0.10.0" on
   a `release/0.10.0` branch from that `main` head, changing `CHANGELOG.md` (cut `## [Unreleased]`
   under `## [0.10.0] — <date>`), `Cargo.toml` workspace version `0.9.3 → 0.10.0`, `Cargo.lock`
   (the workspace members' versions) and `README.md` § Status ("Tagged `0.10.0` (<date>)"), as the
   `Release 0.9.3` commit `61a8fa7` did (4 files). The full gate runs on that commit; the gate's
   own exit status is read, never a pipeline's.
2. Merge to `main` through the bot route (fast-forward push of the checked head, as the identity
   ruleset 22044982 requires) after CI Gate, MSRV 1.98 and Security and privacy pass.
3. Annotated tag `0.10.0` (the bare version, atlas § Naming) on that `main` commit, pushed through
   `b10x-gates bot`.
4. The Release workflow builds the four platform archives and `release-checksums`; download the
   `release-*` artifacts by run id, verify the four filenames and `SHA256SUMS`, extract the two
   Linux archives and scan their executables with `b10x-gates scan-artifact`; macOS scanning is not
   required (operator decision 2026-10-03).
5. Publish through `b10x-gates api` (release metadata, notes from a file) and `b10x-gates gh --
   release upload` (assets), with `--policy` pointing at the trusted policy clone; never a personal
   `gh` write. Verify the published release is owned by b10x-bot[bot], targets the exact annotated
   tag and holds the four archives and checksums. A green package run with no published release is
   queued, not released.
6. Record the publication here (release id, tag object, peeled commit, asset digests), then move to
   `implemented`. Documentation publication is asynchronous and reported as pending unless verified.

## Not in this release

A paid run; a vendor pin move; the ESS pin (stays 0.56.0); crates.io or any other registry
(excluded by the standing exception).

## Status

Draft, written with the wave proposal; executes only after the wave closes and only under the
approval the proposal asks for.
