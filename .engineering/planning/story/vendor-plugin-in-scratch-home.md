---
format: aep.planning-md/1
id: story:vendor-plugin-in-scratch-home
kind: story
status: implemented
title: A marketplace plugin is installed into the hermetic scratch home, on request and attested
summary: metaharness run claude --plugin <repo>@<name>@<pin> places a third-party plugin into the scratch home; the attestation lists it.
owner: metaharness
tags:
- bench
- hermetic
relations:
- decomposes: epic:runs-side-by-side
revision: 4
---
# Story: A marketplace plugin is installed into the hermetic scratch home, on request and attested

## Outcome

A third-party Claude Code plugin — the first is `dev-team@bfinster` — runs under this driver with the same event stream as ours, so the bench compares two stacks through one instrument.

## Context

Hermeticity means "no ambient plugins" (`README.md` § The three promises). The bench needs the opposite on purpose: a named plugin placed into the scratch config home before launch. The `plugin` arm of `aep eval run` presumes the shipped plugin is installed; how it gets into the scratch home is not stated in `aep eval run --help` at 0.42.0.

## Acceptance

- `metaharness run claude --plugin <marketplace-repo>@<name>[@<version>]` installs the plugin into the scratch home before launch, pinned to a commit or version, and refuses an unpinned one.
- The hermetic attestation lists every installed plugin; a run with none says `plugins: none`.
- A conformance vector records the launch with a plugin present.
- `aep eval run --arm plugin` can pass the option through (the `aep` side is a one-line change filed there when this lands).

## Out of Scope

Codex plugins; Pi and OpenCode.

## Ambiguities

- `inferable` — Claude Code's marketplace install form is `/plugin marketplace add <repo>` then `/plugin install <name>@<marketplace>`; the headless equivalent is to be established by a research note under `docs/research/` first (`AGENTS.md` § research).

## Open Questions

None.
