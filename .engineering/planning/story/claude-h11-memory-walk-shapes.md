---
format: aep.planning-md/3
id: story:claude-h11-memory-walk-shapes
kind: story
status: draft
title: Which memory-file shapes the Claude H11 walk counts
refs:
- provider: atlas
  reference: adr-0098
relations:
- decomposes: epic:loom-agent-work-through-metaharness
- informed_by: review-result:adr0096-w4-claude-adversary-1
revision: 2
---
# Story: which memory-file shapes the Claude H11 walk counts

## Scheduling

Decided; not implemented in wave 4 unit 2 (whose last correction reverted the widened walk). Implementation
is its own unit with its own adversary passes: first unit of wave 5 unless `b10x-chef-orchestrator` places it
before release 0.11.0.

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
