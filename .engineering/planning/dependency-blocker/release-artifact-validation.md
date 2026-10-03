---
format: aep.planning-md/3
id: dependency-blocker:release-artifact-validation
kind: dependency-blocker
status: cleared
title: Release executables require supported artifact scans and remapped build paths
relations:
- blocks: story:bot-release-publication
- blocks: release-plan:release-0-9-0
revision: 4
transitions:
- {from: "open", to: "cleared", at: "2026-10-03T13:55:04Z", actor: "human:timo", revision: 4}
---
## Publication withheld after artifact validation

Metaharness 0.9.0 source is merged through the single integration PR #22 at 781b14bc93f9ba176643a26204c7092554160269. Required main checks passed. Annotated tag 0.9.0 resolves to that exact commit; tag object is 6eb0c4c94802b71bff7775439ee53ef3bc172b08. Release workflow 37122857332 completed successfully, including all four platform packages and Verify release archives. All four downloaded archives match the workflow's SHA256SUMS.

Publication remains withheld. Gates 0.1.12 scan-artifact accepts ELF64 little-endian executables, not compressed archives or Mach-O binaries. After extracting the exact archives without executing their contents, both macOS executables refused with `artifact format unsupported`. Linux executable scans also refused: aarch64 reported 488 private-rule matches and x86_64 reported 445, with `personal-paths` in the reported coordinates. No matched private strings were published. The release workflow's cargo build currently has no source-path remapping.

Required resolution: qualify a Gates scanner supporting both actual macOS executable formats; produce release archives with build-source paths remapped and all required checks intact; scan the actual executables and verify checksums before bot-authenticated publication. Do not waive or bypass these refusals, overwrite the existing annotated tag, or report a release as published. The operator's one-PR constraint still applies: PR #22 is already merged, so any additional source delivery needs an agreed route.

No GitHub Release was created and no issues were closed. GitHub issue repair remains implemented but the publication story and release plan stay active. The release coordinator owns the retained integration tree and private scan logs until the dependency is resolved. Documentation publication is asynchronous and is not this blocker.

Evidence: https://github.com/beyond10x/metaharness/pull/22 and https://github.com/beyond10x/metaharness/actions/runs/37122857332. Archive checks and scanner exit statuses were observed on 2026-10-03.

## Prepared repair verified on 2026-10-03

The integration branch now sets release RUSTFLAGS to remap the runner home and workspace to neutral build paths. The remaining embedded golden fixture paths were further redacted under `/fixture/operator`, with derived slugs and Codex hook expectations updated. Capture versions, record order, tool calls and identifiers are unchanged. Generated event expectations were rebuilt with workspace feature resolution; the standalone adapter generator had changed one floating-point serialization, and regenerating with the released CLI's feature graph restored it. Fixture READMEs document the redaction and correct regeneration command.

`task check` passed after these changes. Workflow YAML parsed and the extracted package shell step passed `bash -n`. The rebuilt x86_64 Linux release executable reports metaharness 0.9.0 and has SHA256 ff55489feea8dace8e6d15aaba93863e2068d018ce2ccb5ff190467902f92e04. Gates 0.1.12 scan-artifact passed on those exact bytes: input_bytes=30027456, privacy_bytes=21843309, extracted_bytes=16713220, sections=34, scanner_invocations=1. The exact executable's offline conformance passed Claude 27/27, Codex 17/17 and b10x 7/7 vectors.

This is local x86_64 Linux qualification of the prepared repair, not replacement evidence for the existing tag's CI archives. All four release platforms still need rebuilt, verified, scanned archives from an agreed source delivery. Gates Mach-O support remains unavailable, the single integration PR is already merged, and the annotated 0.9.0 tag has not moved. The release blocker stays open; no release publication or issue closure is claimed.

## Operator release decision, 2026-10-03

The operator explicitly instructed "cut a release now" and rejected adding macOS scanning: "no - you dont, we dont need this". Gates development is cancelled; its unused managed worktree was retired without changes. macOS scanning is not a prerequisite for this release. Retain four platform builds, version smoke tests, archive verification and checksums, and scan the Linux executables with existing Gates. Do not claim macOS scan results.

The path correction is committed on the existing integration branch at 08ead65c774dbce87d7764900e76a8acb788cc4c and passed the full gate plus an exact rebuilt x86_64 Linux artifact scan and 51 offline vectors. Deliver this correction under the renewed release instruction, retaining the integration branch and required branch checks. The earlier PR #22 is merged. Preserve the existing annotated 0.9.0 tag; cut 0.9.1 from the corrected gated main commit instead.

Publication work is active again. The dependency is cleared by the implemented Linux path correction and the operator's explicit release boundary. Rebuilding and verifying all four 0.9.1 archives is remaining release work, not evidence already obtained. Close the GitHub issues only after publication is verified.
