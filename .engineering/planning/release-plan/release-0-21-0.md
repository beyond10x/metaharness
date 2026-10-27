---
format: aep.planning-md/3
id: release-plan:release-0-21-0
kind: release-plan
status: draft
title: 'Publish Metaharness 0.21.0: Harness 0.21.0 on Substrate 0.13.0'
summary: 'Pin-only hop: harness crates and the admitted b10x-harness release move to 0.21.0'
revision: 1
---
## Why this release

`story:b10x-harness-pin-substrate-0-12`: metaharness admits the Harness release that embeds Substrate
`0.13.0` (ADR 0109 confinement contract and its follow-ups). Operator, 2026-10-27: "yes, we want to
upgrade to latest substrate for: loom, takt, harness, metaharness."

## Version

`0.21.0`: `--strict-version` admits another Harness release than 0.20.0 did, which changes what a
caller reads; minor bump under the pre-v1 scheme.

## Procedure (AGENTS.md § Releases and § Release publication identity; as run for 0.20.0)

1. On `release/0.21.0` from `main`: the store commit, the pin commit (both harness revisions,
   `HARNESS_REVISION`, `PINNED_VERSIONS`, the lock invariant test, the golden re-recorded from the
   checksum-verified Harness 0.21.0 archive) and "Release 0.21.0" holding `CHANGELOG.md`, the
   workspace version 0.20.0 → 0.21.0, the `Cargo.lock` workspace members, the `README.md` status line
   and this record. The whole gate runs on that head.
2. Open the PR through the bot. After CI passes, the bot fast-forwards `main`, then writes and pushes
   the annotated tag `0.21.0`.
3. Publish: download the Release workflow's four archives and `SHA256SUMS` by run id and check them;
   `b10x-gates scan-artifact` on both Linux executables; the bot publishes the GitHub Release and
   uploads the assets. No registry publishing.
4. Run `verify-release.sh metaharness 0.21.0` and write the announcement.
