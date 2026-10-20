---
format: aep.planning-md/3
id: story:claude-code-2-1-304-declared
kind: story
status: draft
title: The Claude Code adapter declares 2.1.304 and the built-in plugins it ships
refs:
- provider: atlas
  reference: adr-0096
relations:
- decomposes: epic:loom-agent-work-through-metaharness
revision: 1
---
# Story: the Claude Code adapter declares 2.1.304 and the built-in plugins it ships

## Scheduling

Draft, filed 2026-10-20 on a request from `b10x-chef-orchestrator` (approved under the standing
exception for ADR 0096–0101 work; ADR 0096 L2 is metaharness's): deliver in the wave after wave 3 of
2026-10-20, then release, because takt's session executor pins metaharness and works around H9 and
H1a until a release declares 2.1.304.

## Outcome

`metaharness run claude` against the installed Claude Code 2.1.304 attests its hermetic rows from what
that version ships, including its built-in plugins, so an embedder running `--hermetic strict` needs no
workaround for H9 or H1a.

## Context

- Installed: `claude --version` prints `2.1.304 (Claude Code)` (read 2026-10-20).
- Pinned: `PINNED_VERSIONS = ["2.1.259"]` (`crates/metaharness-claude/src/lib.rs:127`, at `c2548b3b`,
  0.10.1). The reason it stayed there: "Claude's three built-in plugins leave H1a unqualified, so its
  pin remains 2.1.259" (`.engineering/planning/story/current-adapter-compatibility.md:86`).
- Relayed from takt by `b10x-chef-orchestrator`: 2.1.304 ships 4 built-in plugins that 0.10.1 does not
  declare, so takt's session executor works around H9 and H1a. Which four, and what each does to H9 and
  H1a, is not verified here; the scoper and the implementor read them from the 2.1.304 binary and its
  scratch home.
- The adapter's plugin surface: `INSTALLED_PLUGINS`, `KNOWN_MARKETPLACES`, `PLUGIN_CACHE_HOME`,
  `PLUGIN_REGISTRY_HOME`, `scratch_registry` (re-exported at `crates/metaharness-claude/src/lib.rs:49-50`).
- The pin-move protocol is AGENTS.md § Live-evaluating: build or obtain the binary at the version,
  `--strict-version` refuses before spawn off the pin, the credential-free native fixture qualifies the
  pin without a paid run, and a research page records the surface.

## Acceptance

`PINNED_VERSIONS` names 2.1.304; each built-in plugin 2.1.304 ships is declared by name in the adapter,
with what the launch does about it (disabled, isolated in the scratch home, or attested) and the H9 and
H1a rows it moves, each row reading what was observed against 2.1.304 (invariant 3: a declaration never
makes a row `ok` without an observation); the credential-free native fixture passes against 2.1.304;
`docs/research/<date>-claude-code-2.1.304-adapter-surface.md` lists every adapter claim as verified or
unverified against that version; `task check` exits 0; `CHANGELOG.md` names the pin move.

## Out of scope

- A paid run (outside the standing approval).
- Codex and b10x pins.

## Scope

- inferred: `crates/metaharness-claude/src/lib.rs`, `crates/metaharness-claude/src/launch.rs`,
  `crates/metaharness-claude/src/vectors.rs`, `crates/metaharness-claude/fixtures/`,
  `crates/metaharness/fixtures/golden/contract-result-claude.json`, `crates/metaharness/src/audit.rs`,
  `docs/research/`, `docs/design/metaharness-protocol-v0.1.md`, `website/docs/harnesses/claude.md`,
  `CHANGELOG.md`. To be scoped by `aep:story-scoper` before the wave is proposed.
