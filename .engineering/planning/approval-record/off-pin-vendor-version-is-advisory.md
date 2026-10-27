---
format: aep.planning-md/3
id: approval-record:off-pin-vendor-version-is-advisory
kind: approval-record
status: draft
title: An off-pin vendor version is not a hard failure
relations:
- decides: story:off-pin-vendor-version-is-advisory
revision: 1
---
## Decision

Operator, 2026-10-27, relayed verbatim by the coordinating session: "Another defect is that
currently my current codex version is just not working together with takt because it, for whatever
reason, is doing some weird version check, which I think doesn't make much sense because we're
using codex via metaharness, and metaharness should just use that protocol from codex and just
work. There shouldn't be a hard failure due to that."

The coordinating session, 2026-10-27, under the operator's standing grant: "a version outside the
pin is not a hard failure. The check becomes advisory: the probe runs, the version is recorded in
the run record and printed once, a known-incompatible protocol ... still refuses with the protocol
error, and `--strict-version` stays opt-in for the operator who wants the pin. If the Claude H11
memory-canary floor is the same mechanism, treat it the same way". Approved in the same message:
the story, one wave unit (implementor and adversary in a managed worktree), the bot pull request,
the merge, and the release of 0.20.0 through this repository's process.

## What is decided

`story:off-pin-vendor-version-is-advisory` is accepted as written: H9 becomes advisory on every
kind, an unmeasured Claude version reads H11 `unk`, the version is warned once, `--strict-version`
is unchanged. The commits this authorises: the unit's commits on `unit/off-pin-version-advisory`,
the store commits riding the same pull request, the release commit and the annotated tag `0.20.0`.

## Evidence at decision time

Reproduced before the decision with metaharness 0.19.0 and the operator's `~/.local/bin/codex`
at 0.164.0: takt exit 5 on `H9 Gap`, `metaharness run codex --hermetic strict` exit 1,
`--strict-version` exit 2. The story's Context carries the text of each.
