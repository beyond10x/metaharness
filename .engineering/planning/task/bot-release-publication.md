---
format: aep.planning-md/3
id: task:bot-release-publication
kind: task
status: implemented
title: Separate read-only release packaging from bot-owned publication
relations:
- decomposes: story:bot-release-publication
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T10:01:52Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-03T10:01:52Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-03T14:22:47Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":2}}}
---
## Purpose and authorized scope

The operator's release goal requires every publication write to use b10x-bot[bot]. The existing release workflow publishes with github.token, which is the GitHub Actions identity; the repository has no configured App secrets. This must be corrected before triggering the next release.

## Decision

Keep GitHub Actions credential-free for packaging: retain its four platform builds, smoke checks and archive verification, make all permissions read-only, and retain SHA256SUMS as a downloadable workflow artifact. Remove only the automatic GitHub Release mutation step. The coordinator downloads the exact successful tag run's artifacts, scans them through b10x-gates, and creates/updates the release through the existing bot-authenticated b10x-gates route. This avoids introducing App credentials into candidate code or inventing another publisher.

## Acceptance

The workflow retains all four target archives, binary/tag version verification, archive presence and checksum verification; it has no contents:write permission, GH_TOKEN, gh release mutation or repository App secret. Its final artifact contains the checksums from the same run. AGENTS.md states the exact-tag artifact download, bot publication and post-publication verification boundary. Validate workflow syntax and inspect all original checks preserved. No release is reported complete until the published assets and exact tag are verified.

## Review and boundary

This is release tooling, not a new product runtime entity. No new executable script is committed; existing shell build/check steps are retained and the publisher is the installed Rust b10x-gates CLI. Source correctness and both required GitHub gate groups remain prerequisites. Operator authorization is the active release goal; one integration PR remains the delivery boundary.
