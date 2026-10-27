---
format: aep.planning-md/3
id: release-plan:release-0-20-0
kind: release-plan
status: draft
title: 'Publish Metaharness 0.20.0: an off-pin vendor version is reported, not refused'
summary: H9 advisory; unmeasured Claude H11 and builtin H1a read unk; version_outside_pin warned once (a62)
revision: 1
---
## Why this release

`story:off-pin-vendor-version-is-advisory` (a62): a Claude or Codex binary outside the adapter's pin
is measured and reported, never refused for its version alone. Operator, 2026-10-27: "There
shouldn't be a hard failure due to that." A takt run whose child codex was 0.164.0 (or 0.160.0
under an older pin) spent a step's tokens and then stopped on `H9 Gap`.

## Version

`0.20.0`: H9 no longer moves the exit code or a consumer's gating fold, a strict Claude run on an
unmeasured version exits 3 instead of 1, and the Claude stream gains a `version_outside_pin`
warning. Each changes what a caller reads; minor bump under the pre-v1 scheme.

## Procedure (AGENTS.md § Releases and § Release publication identity; as run for 0.19.0)

1. On `release/0.20.0` from `main`: the unit commit and one commit, "Release 0.20.0", holding
   `CHANGELOG.md`, the workspace version 0.19.0 → 0.20.0, the `Cargo.lock` workspace members, the
   `README.md` status line and this record. The whole gate runs on that commit.
2. Open the PR through the bot. After CI passes, the bot fast-forwards `main`, then writes and pushes
   the annotated tag `0.20.0`.
3. Publish: download the Release workflow's four archives and `SHA256SUMS` by run id and check them;
   `b10x-gates scan-artifact` on both Linux executables; the bot publishes the GitHub Release and
   uploads the assets. No registry publishing.
4. Run `verify-release.sh metaharness 0.20.0` and write the announcement.
