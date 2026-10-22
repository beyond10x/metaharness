---
format: aep.planning-md/3
id: release-plan:release-0-14-0
kind: release-plan
status: draft
title: Publish Metaharness 0.14.0 so loom and takt can pin harness 0.18.0
relations:
- delivers: story:codex-command-outcomes-reach-tool-result
- delivers: story:narrowed-cap-refused-before-persist
- delivers: story:exit-status-separates-task-control-audit
- delivers: story:mcp-arguments-checked-against-declared-types
- delivers: story:first-user-documents-agree-with-cli
- delivers: story:b10x-api-key-from-keychain
- delivers: story:aep-eval-native-arm-removed
revision: 1
---
## Why this release

Wave 8 of the ADR 0096 work moved the b10x adapter's harness pin to 0.18.0 (`330e2cde`) and removed
`aep-eval native`, whose `b10x-harness workflow run` harness 0.18.0 no longer has. loom and takt pin
metaharness and stay on harness 0.17 until a metaharness release carries the 0.18.0 pin.

`b10x-chef-orchestrator`, 2026-10-22, after verifying `main` at `9458d50` with bot author and committer:
"Go: release 0.14.0 through the repo's own process once the push runs on 9458d50 are green, so loom and
takt can move off harness 0.17 (class C under the grant)." The same message leaves the trees `mh-w8-u5`
and `mh-w8-u7` as they are.

## Version

`0.14.0`: removed surface (`aep-eval native`), new fields on `RunSpec`, `FloorInputs` and `AuditReport`,
a new `Credential` arm; minor bump under the pre-v1 scheme. ESS stays at 0.67.0, the newest ESS release.

## Procedure (AGENTS.md § Releases and § Release publication identity; as run for 0.13.0)

1. After the push runs on `9458d50` are green: one commit "Release 0.14.0" on `release/0.14.0` from that
   head: `CHANGELOG.md` (cut `## [Unreleased]` as `## [0.14.0] — 2026-10-22`), workspace version
   0.13.0 → 0.14.0, `Cargo.lock` workspace members, `README.md` status, this record. The whole gate runs
   on it.
2. PR through the bot; after CI (Gate, MSRV 1.98) the bot fast-forwards `main` and writes and pushes
   the annotated tag `0.14.0`.
3. The Release workflow's four archives and `SHA256SUMS` are downloaded by run id and checked, both
   Linux executables pass `b10x-gates scan-artifact`, and the bot publishes the GitHub Release and
   uploads the assets. No registry publishing.
