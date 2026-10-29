---
format: aep.planning-md/3
id: release-plan:release-0-24-1
kind: release-plan
status: draft
title: 'Publish Metaharness 0.24.1: Codex catalogue live follow-ups, ESS 0.77.0'
summary: live follow-ups ride-along, duplicate-name test, ESS 0.77.0
relations:
- supersedes: release-plan:release-0-24-0
revision: 1
---
## Why this release

Wave D221 (`approval-record:wave-d221`):

- `story:codex-catalogue-live-follow-ups` (ride-along `plan/codex-live-20261029`, D209): six live runs of
  the 0.24.0 Codex catalogue route at Codex 0.153.4 and 0.160.0, a test holding the
  `catalogue-malformed` refusal of duplicate names on Claude and Codex, and corrected CLI and Codex
  pages (amendment a67).
- `task:ess-pin-0-77-0`: ESS 0.77.0 is the specification toolchain; `spec/conformance.json` is
  byte-identical.
- `story:caller-supplied-catalogue-on-every-kind` and `story:independent-documentation` were already
  implemented in wave D193 and need no change.

## Version

`0.24.1`. No public type, flag, refusal or wire changes: one added test, documentation corrections and
the specification toolchain pin.

## Procedure

As in `release-plan:release-0-24-0`: the release commit and the full gate on `integration/d221`; one PR
through the bot; after CI, the bot fast-forwards `main` and pushes the annotated tag `0.24.1`; artifacts
are checked and scanned, the bot publishes the release, then `verify-release.sh metaharness 0.24.1
<announcement>`.
