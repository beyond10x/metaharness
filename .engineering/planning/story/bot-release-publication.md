---
format: aep.planning-md/3
id: story:bot-release-publication
kind: story
status: implemented
title: Publish release artifacts only through the organization bot
relations:
- decomposes: epic:github-issue-repair
- delivers: release-plan:release-0-9-0
- delivers: release-plan:release-0-9-1
scope:
- confidence: cited
  path: .github/workflows/release.yml
- confidence: cited
  path: AGENTS.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T10:02:23Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-03T10:02:23Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-03T14:22:47Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":2}}}
---
## Purpose and authorized scope

The operator's release goal requires every publication write to use b10x-bot[bot]. The existing release workflow publishes with github.token, which is the GitHub Actions identity; the repository has no configured App secrets. This must be corrected before triggering the next release.

## Decision

Keep GitHub Actions credential-free for packaging: retain its four platform builds, smoke checks and archive verification, make all permissions read-only, and retain SHA256SUMS as a downloadable workflow artifact. Remove only the automatic GitHub Release mutation step. The coordinator downloads the exact successful tag run's artifacts, scans them through b10x-gates, and creates/updates the release through the existing bot-authenticated b10x-gates route. This avoids introducing App credentials into candidate code or inventing another publisher.

## Acceptance

The workflow retains all four target archives, binary/tag version verification, archive presence and checksum verification; it has no contents:write permission, GH_TOKEN, gh release mutation or repository App secret. Its final artifact contains the checksums from the same run. AGENTS.md states the exact-tag artifact download, bot publication and post-publication verification boundary. Validate workflow syntax and inspect all original checks preserved. No release is reported complete until the published assets and exact tag are verified.

## Review and boundary

This is release tooling, not a new product runtime entity. No new executable script is committed; existing shell build/check steps are retained and the publisher is the installed Rust b10x-gates CLI. Source correctness and both required GitHub gate groups remain prerequisites. Operator authorization is the active release goal; one integration PR remains the delivery boundary.

## Verified publication, 2026-10-03

Metaharness 0.9.1 is published at https://github.com/beyond10x/metaharness/releases/tag/0.9.1 (release 402542049, published_at 2026-10-03T14:19:57Z). The release and all five asset uploads belong to b10x-bot[bot]. Its annotated tag object 37c8c2ae2b1eecaf2ccfa536698f3f282789bbdc peels to main commit 6dbf68b9d284cd038c875074334dcff1f843427c. The merged tree 2684f16fa1ba015456638ba20355e89d29a651e4 is identical to the tested candidate.

PR #23 delivered the release correction on the existing integration branch after the operator renewed the release instruction; PR #22 delivered the issue repairs. Required Gate, MSRV 1.98 and common Security and privacy checks passed on the exact source commit. Release run https://github.com/beyond10x/metaharness/actions/runs/37128357701 passed all four platform jobs and Verify release archives. All four archive names and contents, README version, asset sizes and SHA256 digests were verified. A fresh download from the published release passed all four checksums and matched the workflow's SHA256SUMS.

Gates 0.1.12 passed both actual Linux executables: aarch64 SHA256 d1307295faba6e186ec4692ab79035cc5db4c9a000d13b28eaaae93d206e25ec and x86_64 SHA256 b6453521221cf5dac2b1be8a454efd86fc4595207f4da4c7bf19f1ee379bbe4c. No macOS artifact scan is claimed or required under the operator's explicit decision. No Gates source changes were made.

After verifying publication, GitHub issues #10 through #21 were closed as completed through the bot App. Each response records closed_by=b10x-bot[bot]; a fresh open-issue query returned an empty list. The earlier 0.9.0 tag remains unchanged and has no published GitHub Release. Source release completion is verified; documentation publication remains asynchronous and unverified.

This publication evidence is recorded after the release on the integration branch. It is not claimed to be present in the tagged source tree.
