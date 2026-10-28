---
format: aep.planning-md/3
id: release-plan:release-0-23-0
kind: release-plan
status: draft
title: 'Publish Metaharness 0.23.0: the run-spec document, a bounded version probe under load, Harness 0.22.0'
summary: RunSpec::from_document and per-kind check_spec refusals; 60 s --version probe with a 5 s drain; Harness 0.22.0
revision: 1
---
## Why this release

Wave D170 (`approval-record:wave-d170`), together with what landed on `main` since 0.22.0:

- `story:run-spec-document`: `RunSpec::from_document_str` and `RunSpec::from_document` read a session
  request as a run-spec document, refusing unknown keys and bad values by name; `check_spec` refuses
  the per-kind combinations the adapters refused only at launch. Unblocks takt's
  `dependency-blocker:metaharness-session-request-document`.
- `story:version-probe-under-load`: the `--version` probe waits 60 s and drains for at most 5 s after
  exit (takt 0.17.0 trial finding F4).
- `story:b10x-harness-pin-0-22`: Harness 0.22.0 is the admitted `b10x-harness`.
- Already on `main`: ESS 0.75.0 and `story:session-jsonl-adapter` (`metaharness project --from
  claude-code-session`).

## Version

`0.23.0`: new public API (`RunSpec::from_document*`, `RunSpecDocumentError`, the adapters' guards)
and a narrowed admission (0.21.0 `b10x-harness` refused under `--strict-version`, more refusals in
`check_spec`); minor bump under the pre-v1 scheme.

## Procedure (AGENTS.md § Releases and § Release publication identity; as run for 0.22.0)

1. On `wave/d170`: "Release 0.23.0" holding `CHANGELOG.md`, the workspace version 0.22.0 to 0.23.0,
   the `Cargo.lock` workspace members, the `README.md` status line and this record. The whole gate
   runs on the integration branch.
2. Open the PR through the bot. After CI passes, the bot fast-forwards `main`, then writes and pushes
   the annotated tag `0.23.0`.
3. Publish: download the Release workflow's four archives and `SHA256SUMS` by run id and check them;
   `b10x-gates scan-artifact` on both Linux executables; the bot publishes the GitHub Release and
   uploads the assets. No registry publishing.
4. Write the announcement and run `verify-release.sh metaharness 0.23.0 <file>`.
