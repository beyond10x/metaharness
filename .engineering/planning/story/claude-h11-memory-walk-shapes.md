---
format: aep.planning-md/3
id: story:claude-h11-memory-walk-shapes
kind: story
status: active
title: Which memory-file shapes the Claude H11 walk counts
refs:
- provider: atlas
  reference: adr-0098
relations:
- decomposes: epic:loom-agent-work-through-metaharness
- informed_by: review-result:adr0096-w4-claude-adversary-1
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/metaharness-aep/src/drive/ess_conformance.rs
- confidence: cited
  path: crates/metaharness-claude/fixtures/c1/memory-ancestor-refusal.json
- confidence: cited
  path: crates/metaharness-claude/src/launch.rs
- confidence: cited
  path: crates/metaharness-claude/src/lib.rs
- confidence: cited
  path: crates/metaharness-claude/src/vectors.rs
- confidence: cited
  path: crates/metaharness-claude/tests/adapter.rs
- confidence: inferred
  path: crates/metaharness/src/audit.rs
- confidence: cited
  path: crates/metaharness/src/builder.rs
- confidence: inferred
  path: crates/metaharness/src/run.rs
- confidence: cited
  path: crates/metaharness/tests/adversary_claude_2_1_304_pass1.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: inferred
  path: spec/conformance.json
- confidence: inferred
  path: spec/coverage.md
- confidence: inferred
  path: spec/domains/workspace.yaml
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T12:00:25Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-10-21T12:00:25Z", actor: "human:timo", revision: 7}
---
# Story: which memory-file shapes the Claude H11 walk counts

## Scheduling

Wave 5 unit 1 (`approval-record:adr-0096-wave-5`), decided C. Condition from `b10x-chef-orchestrator`
(2026-10-21): before the merge, every consumer of the H11 result in the beyond10x repositories is listed and
reported; if a consumer breaks, the unit stops and goes back to `b10x-chef-orchestrator`. The ESS scenario
`DeclareScratchWorkingDirectory/outcome/memory-above-the-copy-fails-strict` (a27) changes with it.

## Outcome

The H11 row for Claude ("no memory file outside the copied tree is discoverable") counts every memory-file
shape the pinned Claude Code would load from an ancestor, without refusing every run on a machine whose
temporary directory sits under the operator's home.

## Context

- The walk (`crates/metaharness/src/builder.rs`, about `:1948` at `5eacbd4`) counts `CLAUDE.md` and `AGENTS.md`
  only. Claude Code 2.1.304/2.1.305 also loads `CLAUDE.local.md`, `.claude/CLAUDE.md`, `.claude/AGENTS.md` and
  `.claude/rules/*.md` from ancestors once a project or local setting source is on (adversary pass 1; the
  implementor's control runs with `--setting-sources project,local`: 8 canaries of the `CLAUDE.md` family and
  4 of `AGENTS.md` reached the stub provider).
- Through metaharness's launch (`--setting-sources ""`), none reached any request, under `none` and
  `loopback`, on 2.1.304 and 2.1.305 (wave 4 unit 2, native lane).
- Widening the walk to those shapes (tried in unit 2's correction round 1) finds the operator's
  `~/.claude/CLAUDE.md` above every scratch directory when `TMPDIR` is under `HOME` (this machine:
  `~/.cache/claude-tmp`): 80 workspace tests failed, every default `metaharness run claude` was refused
  before spawn, and a `--cwd --scratch-cwd` copy under `HOME` read H11 unavailable. takt's copies live under
  `~/.cache`. The widening was reverted in unit 2.
- Amendment a33 keeps the walk because the vendor's `--setting-sources ""` gate is not a control metaharness
  holds; `$HOME/.claude/` is also the operator's user-config directory, which the launch replaces with a
  scratch config home.
- The adversary's case `h11_is_not_ok_over_a_memory_file_the_pinned_loader_reads_and_the_walk_does_not` is
  committed and ignored with this story's name.

## Decision needed

Options (for `b10x-chef-orchestrator` or Timo): A count the extra shapes everywhere and move metaharness's
own scratch roots outside `HOME` (takt's copies would still refuse); B count them except the operator's
user-config directory `$HOME/.claude/`, with the reason in an amendment; C keep the walk as it is and record
the per-version canary observation as the evidence the gate held.

## Acceptance

For Claude, H11 reads `ok` as "metaharness imposed it" when the launch passed `--setting-sources ""` with a
fresh scratch `CLAUDE_CONFIG_DIR` and the pinned version carries a recorded canary result showing no memory
file reached the model; the ancestor walk (the extra shapes `CLAUDE.local.md`, `.claude/CLAUDE.md`,
`.claude/AGENTS.md`, `.claude/rules/*.md` included) reports what it finds as information in the row and the
attestation, never as a refusal; a pinned version without a recorded canary result reads `gap`; a test fails
when `PINNED_VERSIONS` changes without a canary result for the new version; the c1 `memory-ancestor-refusal`
vector and the adversary's ignored H11 case are rewritten to the decided rule; an amendment states it.

## Decision (b10x-chef-orchestrator, 2026-10-21, about 02:00)

Option **C**, taken by `b10x-chef-orchestrator` as a trade-off inside the approved ADR 0096 design (verbatim
core): "H11 then rests on the imposed control: `--setting-sources ""` with a fresh CLAUDE_CONFIG_DIR, citing the
canary evidence per pinned version. The ancestor walk reports what it finds as information, not a refusal.
Guard: every change to PINNED_VERSIONS re-runs the canary probe, and a canary that reaches the model turns H11
to `gap`."

- Rests on: wave 4 unit 2's canary probe (0 of 9 memory files reached the model through the launch on 2.1.304
  and 2.1.305; 9 of 9 directly with project sources) and `b10x-chef-orchestrator`'s own probe (0 of 1; input
  17,184 against 17,179 tokens without the file).
- A is out: scratch roots outside `HOME` mean `/tmp`, which the operator's rules forbid (per-user quota).
- B would still refuse under `~/beyond10x`, which carries a private `CLAUDE.md` since 2026-10-21 01:4x.
## Scope

Derived 2026-10-21 by `story-scoper` at main 5df311b. Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `crates/metaharness-claude` (H11 attestation, refusal, per-version canary table) and `crates/metaharness` (the walk, the floor) — cited
- **Files:**
  - `crates/metaharness/src/builder.rs:2033` `memory_ancestors` (shared by Claude `:870`, Codex `:1151` and b10x `:655`/`:698`), the Claude `LaunchFacts` at `:933`, and the declared-copy tests at `:3884` (`a_memory_file_above_the_declared_copy_still_fails_strict`, a Claude run) — cited
  - `crates/metaharness-claude/src/launch.rs`: `LaunchContext::memory_ancestors` doc at `:191`, `LaunchRefusal::MemoryAncestorsFound` at `:427`, `memory_refusal` at `:556`, the refusal in `plan_launch` at `:735`, the three H11 branches of the attestation at `:1688-1739`, and tests at `:2780`, `:3046`, `:3108`, `:3165` — cited
  - `crates/metaharness-claude/src/lib.rs:180` `PINNED_VERSIONS`; the new canary table goes beside `MENTION_CANARIES` at `:274` — cited
  - `crates/metaharness-claude/src/vectors.rs:50`, `:144-240`, `:495`, `:583`, `:981` and `crates/metaharness-claude/fixtures/c1/memory-ancestor-refusal.json` — cited
  - `crates/metaharness/tests/adversary_claude_2_1_304_pass1.rs:188`, the ignored case to rewrite — cited
  - `crates/metaharness-claude/tests/adapter.rs:84`, pin-coupling test for the new table — cited
  - `crates/metaharness-claude/tests/native_fixture.rs:1049`, the native canary (ignored in `task check`) — cited
- **Also likely:** `crates/metaharness/src/audit.rs:857` (H11 ignores the version today); new fields on `FloorInputs` (`audit.rs:35`) and `LaunchFacts` (`run.rs:209`) break 12 struct literals; the wider walk must be Claude-only, since Codex and b10x still decide H11 from it — inferred
- **ESS spec / conformance:** if a declared copy under an `AGENTS.md` now reads H11 imposed, `spec/domains/workspace.yaml` (`DeclareScratchWorkingDirectory/outcome/memory-above-the-copy-fails-strict`), `spec/conformance.json`, `spec/coverage.md:222-250` and `crates/metaharness-aep/src/drive/ess_conformance.rs:121` change — inferred
- **Documents:** design doc § 8.1 H11 row `:1247`, a33 `:2759-2791`, a new amendment after a35; `website/docs/harnesses/claude.md:74-79`, `website/docs/reference/cli.md:195-198`, `CHANGELOG.md` — cited
- **Would collide with:** `crates/metaharness-claude/src/launch.rs`, `crates/metaharness/src/builder.rs`, `run.rs` `LaunchFacts`, `audit.rs` `FloorInputs`, the design doc's amendment tail, `spec/domains/workspace.yaml`
- **Size:** M, or L with the declared-copy flip and its ESS change; 12–20 files; no paid or native run for the gate — inferred

Scoper's contradictions with the story body: line reference stale (`:1948` → `:2033`); the walk is shared with Codex and b10x; the conformance scenario `memory-above-the-copy-fails-strict` flips under C; canary count 12, not 9; "pinned without a canary reads gap" is reachable only for a leaking row or an unpinned version; the canary covers the cwd and one level above, not `$HOME/.claude/CLAUDE.md`.
