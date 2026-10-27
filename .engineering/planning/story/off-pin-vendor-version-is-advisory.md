---
format: aep.planning-md/3
id: story:off-pin-vendor-version-is-advisory
kind: story
status: implemented
title: An off-pin Claude or Codex binary is measured and reported, never refused for its version
summary: H9 advisory, the version recorded and printed once, H11 unk for an unmeasured Claude version, --strict-version opt-in
relations:
- decomposes: epic:loom-agent-work-through-metaharness
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-27T01:38:56Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"test_result":1,"approval":1}}}
- {from: "proposed", to: "active", at: "2026-10-27T01:38:56Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1,"approval":1}}}
- {from: "active", to: "implemented", at: "2026-10-27T02:33:34Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":2,"approval":1}}}
---
# Story: off-pin-vendor-version-is-advisory

## Outcome

A Claude or Codex binary outside the adapter's pin is measured and reported, never refused for its
version alone: H9 is an advisory row, the version is in the run record and printed once, a Claude
version with no memory canary reads H11 `unk` rather than `gap`, and `--strict-version` stays the
operator's opt-in refusal.

## Context

Operator, 2026-10-27: "Another defect is that currently my current codex version is just not
working together with takt because it, for whatever reason, is doing some weird version check,
which I think doesn't make much sense because we're using codex via metaharness, and metaharness
should just use that protocol from codex and just work. There shouldn't be a hard failure due to
that."

Reproduced on 2026-10-27 with metaharness 0.19.0 and takt on loom 0.43.0. `codex --version` on the
operator's `PATH` prints `codex-cli 0.160.0` (a pinned version), but the child's `PATH`
(`$HOME/.local/bin` first, built from the process's `HOME`) resolves `~/.local/bin/codex`, which a
standalone updater moved to 0.164.0 that morning. So the session ran on 0.164.0:

- `takt run --harness codex --forge none` on a scratch crate: step 2 spent 46,182 tokens, then
  `the session proposed nothing: the session's hermetic floor has a gating gap: H9 Gap: the vendor
  is 0.164.0 and the adapter is pinned to ["0.153.4", "0.160.0"]; a verdict that changed because
  the reader changed must be visible as such`, exit 5. The hold is Loom's session executor
  (`crates/loom-session-executor/src/lib.rs:1151` at loom 0.43.0): a Codex row that is gating and
  not `ok` holds the proposal; H9 is gating (`HermeticRow::severity`,
  `crates/metaharness-protocol/src/hermetic.rs:164`).
- `metaharness run codex --hermetic strict --decisions observe`: `H9 gap gating`, `audit verdict:
  gap (exit 1)`.
- `metaharness run codex --strict-version`: `--strict-version requires one of ["0.153.4",
  "0.160.0"]; ~/.local/bin/codex reported 0.164.0, so the run is refused before the
  spawn`, exit 2. takt and Loom do not pass `--strict-version`.
- `metaharness run codex --hermetic on`: exit 0 with one `version_outside_pin` warning in the event
  stream and nothing on stderr.

Claude has the same shape one row later: an off-pin Claude version has no `MEMORY_CANARIES` row, so
`memory_held_to_canary` (`crates/metaharness/src/audit.rs`) reads H11 `gap`, and Loom holds a
Claude proposal on any gating `gap` but H9 and builtin-only H1a.

## Acceptance

1. H9 is advisory on every kind: `HermeticRow::H9.severity()` is `Advisory`; its verdicts are
   unchanged (`ok` on the pin, `gap` off it, `unk` with no version). A scripted Codex strict run
   whose rollout names 0.164.0 exits as the same run on 0.160.0 does, with H9 `gap advisory`
   rendered.
2. A Claude record naming a version with no memory canary row reads H11 `unk` ("no memory canary
   recorded for <version>, so nobody measured whether this binary honours the control"), not
   `gap`; a canary row in which a canary reached a request, or the control sent fewer than were
   planted, still reads `gap`; no version still reads `unk`.
3. An off-pin Claude or Codex opening record puts one `warning{code: "version_outside_pin"}` in the
   event stream, and `metaharness run` prints its message to stderr once per run, in every
   hermetic mode.
4. `--strict-version` is unchanged: an off-pin or unreadable binary is refused before the spawn,
   exit 2.
5. A binary that does not speak the adapter's protocol still fails with the protocol's own error,
   as before (an existing test names it).
6. The ESS domain `metaharness.session` says it: `ExitAuditedRun`'s `off-the-pin` outcome is
   `control: unk, audit: no-verdict, exit_code: 3`, regenerated with ESS 0.70.0, and
   `task check` validates and runs it.
7. End to end: the takt run above, built with `[patch]` path overrides to this tree (never
   committed), proceeds past step 2 with the 0.164.0 binary.
8. Design amendment a62 records the change; `CHANGELOG.md` names it under Changed.

## Out of Scope

- Loom's executor comment and its `|| row.row == HermeticRow::H9` arm: dead once H9 is advisory;
  Loom's pin hop removes it.
- The `aep-eval` b10x identity check, which compares a release banner and a git revision before
  a paid eval, not a vendor pin.
- Moving `PINNED_VERSIONS` to 0.164.0: a pin move is a measurement (invariant 4), not this fix.

## Ambiguities

None.

## Open Questions

None.
