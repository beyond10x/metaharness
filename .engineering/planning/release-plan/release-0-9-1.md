---
format: aep.planning-md/3
id: release-plan:release-0-9-1
kind: release-plan
status: implemented
title: Publish Metaharness 0.9.1 with the verified release path correction
relations:
- supersedes: release-plan:release-0-9-0
- delivers: epic:github-issue-repair
revision: 4
transitions:
- {from: "draft", to: "active", at: "2026-10-03T13:55:04Z", actor: "human:timo", revision: 2}
- {from: "active", to: "implemented", at: "2026-10-03T14:22:47Z", actor: "human:timo", revision: 4}
---
## Operator release decision, 2026-10-03

The operator explicitly instructed "cut a release now" and rejected adding macOS scanning: "no - you dont, we dont need this". Gates development is cancelled; its unused managed worktree was retired without changes. macOS scanning is not a prerequisite for this release. Retain four platform builds, version smoke tests, archive verification and checksums, and scan the Linux executables with existing Gates. Do not claim macOS scan results.

The path correction is committed on the existing integration branch at 08ead65c774dbce87d7764900e76a8acb788cc4c and passed the full gate plus an exact rebuilt x86_64 Linux artifact scan and 51 offline vectors. Deliver this correction under the renewed release instruction, retaining the integration branch and required branch checks. The earlier PR #22 is merged. Preserve the existing annotated 0.9.0 tag; cut 0.9.1 from the corrected gated main commit instead.

Publication work is active again. The dependency is cleared by the implemented Linux path correction and the operator's explicit release boundary. Rebuilding and verifying all four 0.9.1 archives is remaining release work, not evidence already obtained. Close the GitHub issues only after publication is verified.

## Verified publication, 2026-10-03

Metaharness 0.9.1 is published at https://github.com/beyond10x/metaharness/releases/tag/0.9.1 (release 402542049, published_at 2026-10-03T14:19:57Z). The release and all five asset uploads belong to b10x-bot[bot]. Its annotated tag object 37c8c2ae2b1eecaf2ccfa536698f3f282789bbdc peels to main commit 6dbf68b9d284cd038c875074334dcff1f843427c. The merged tree 2684f16fa1ba015456638ba20355e89d29a651e4 is identical to the tested candidate.

PR #23 delivered the release correction on the existing integration branch after the operator renewed the release instruction; PR #22 delivered the issue repairs. Required Gate, MSRV 1.98 and common Security and privacy checks passed on the exact source commit. Release run https://github.com/beyond10x/metaharness/actions/runs/37128357701 passed all four platform jobs and Verify release archives. All four archive names and contents, README version, asset sizes and SHA256 digests were verified. A fresh download from the published release passed all four checksums and matched the workflow's SHA256SUMS.

Gates 0.1.12 passed both actual Linux executables: aarch64 SHA256 d1307295faba6e186ec4692ab79035cc5db4c9a000d13b28eaaae93d206e25ec and x86_64 SHA256 b6453521221cf5dac2b1be8a454efd86fc4595207f4da4c7bf19f1ee379bbe4c. No macOS artifact scan is claimed or required under the operator's explicit decision. No Gates source changes were made.

After verifying publication, GitHub issues #10 through #21 were closed as completed through the bot App. Each response records closed_by=b10x-bot[bot]; a fresh open-issue query returned an empty list. The earlier 0.9.0 tag remains unchanged and has no published GitHub Release. Source release completion is verified; documentation publication remains asynchronous and unverified.

This publication evidence is recorded after the release on the integration branch. It is not claimed to be present in the tagged source tree.
