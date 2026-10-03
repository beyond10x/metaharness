---
format: aep.planning-md/3
id: dependency-blocker:release-artifact-validation
kind: dependency-blocker
status: open
title: Release executables require supported artifact scans and remapped build paths
relations:
- blocks: story:bot-release-publication
- blocks: release-plan:release-0-9-0
revision: 1
---
## Publication withheld after artifact validation

Metaharness 0.9.0 source is merged through the single integration PR #22 at 781b14bc93f9ba176643a26204c7092554160269. Required main checks passed. Annotated tag 0.9.0 resolves to that exact commit; tag object is 6eb0c4c94802b71bff7775439ee53ef3bc172b08. Release workflow 37122857332 completed successfully, including all four platform packages and Verify release archives. All four downloaded archives match the workflow's SHA256SUMS.

Publication remains withheld. Gates 0.1.12 scan-artifact accepts ELF64 little-endian executables, not compressed archives or Mach-O binaries. After extracting the exact archives without executing their contents, both macOS executables refused with `artifact format unsupported`. Linux executable scans also refused: aarch64 reported 488 private-rule matches and x86_64 reported 445, with `personal-paths` in the reported coordinates. No matched private strings were published. The release workflow's cargo build currently has no source-path remapping.

Required resolution: qualify a Gates scanner supporting both actual macOS executable formats; produce release archives with build-source paths remapped and all required checks intact; scan the actual executables and verify checksums before bot-authenticated publication. Do not waive or bypass these refusals, overwrite the existing annotated tag, or report a release as published. The operator's one-PR constraint still applies: PR #22 is already merged, so any additional source delivery needs an agreed route.

No GitHub Release was created and no issues were closed. GitHub issue repair remains implemented but the publication story and release plan stay active. The release coordinator owns the retained integration tree and private scan logs until the dependency is resolved. Documentation publication is asynchronous and is not this blocker.

Evidence: https://github.com/beyond10x/metaharness/pull/22 and https://github.com/beyond10x/metaharness/actions/runs/37122857332. Archive checks and scanner exit statuses were observed on 2026-10-03.
