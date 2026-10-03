---
format: aep.planning-md/3
id: release-plan:release-0-9-1
kind: release-plan
status: active
title: Publish Metaharness 0.9.1 with the verified release path correction
relations:
- supersedes: release-plan:release-0-9-0
- delivers: epic:github-issue-repair
revision: 2
transitions:
- {from: "draft", to: "active", at: "2026-10-03T13:55:04Z", actor: "human:timo", revision: 2}
---
## Operator release decision, 2026-10-03

The operator explicitly instructed "cut a release now" and rejected adding macOS scanning: "no - you dont, we dont need this". Gates development is cancelled; its unused managed worktree was retired without changes. macOS scanning is not a prerequisite for this release. Retain four platform builds, version smoke tests, archive verification and checksums, and scan the Linux executables with existing Gates. Do not claim macOS scan results.

The path correction is committed on the existing integration branch at 08ead65c774dbce87d7764900e76a8acb788cc4c and passed the full gate plus an exact rebuilt x86_64 Linux artifact scan and 51 offline vectors. Deliver this correction under the renewed release instruction, retaining the integration branch and required branch checks. The earlier PR #22 is merged. Preserve the existing annotated 0.9.0 tag; cut 0.9.1 from the corrected gated main commit instead.

Publication work is active again. The dependency is cleared by the implemented Linux path correction and the operator's explicit release boundary. Rebuilding and verifying all four 0.9.1 archives is remaining release work, not evidence already obtained. Close the GitHub issues only after publication is verified.
