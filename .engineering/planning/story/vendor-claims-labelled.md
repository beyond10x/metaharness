---
format: aep.planning-md/3
id: story:vendor-claims-labelled
kind: story
status: draft
title: Vendor claims behind the hook quoting are pinned or labelled
summary: The two a37 vendor claims are cited to the pinned binary or labelled unverified.
owner: metaharness
refs:
- provider: atlas
  reference: adr-0096
relations:
- decomposes: epic:session-format
- informed_by: review-result:adr0096-w5-independent-review
revision: 1
---
# Story: vendor claims behind the hook quoting are pinned or labelled

## Scheduling

Draft, filed 2026-10-21 at the close of wave 5 from `review-result:adr0096-w5-independent-review` (s4-3,
s5-1).

## Outcome

Every claim about what a vendor binary does with the hook command is cited to the pinned binary or labelled
unverified, as AGENTS.md invariant 4 requires.

## Context

- a37 § 6 (`docs/design/metaharness-protocol-v0.1.md`) states that `${CLAUDE_PROJECT_DIR}` in a shell-form
  hook command is neither substituted nor checked by Claude; no evidence is cited, and the near-miss test
  (`crates/metaharness-claude/src/launch.rs:3104-3131`) checks only that planning succeeds (s4-3).
- `crates/metaharness-codex/src/launch.rs:1172-1186` states as fact that Codex 0.153.4 runs the hook command
  through a POSIX shell; a37 lists that reading as unverified, taken from the binary's strings (s5-1).

## Acceptance

- expected_before: today both claims stand in the design document and the code comment with no pinned-source
  citation and no unverified label. After: each is either cited to a driven run or source at the pinned
  version, or labelled unverified in a37 and in the comment.

## Scope

Inferred: `docs/design/metaharness-protocol-v0.1.md` (a37), `crates/metaharness-claude/src/launch.rs`,
`crates/metaharness-codex/src/launch.rs`.
