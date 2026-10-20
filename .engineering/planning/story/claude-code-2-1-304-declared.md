---
format: aep.planning-md/3
id: story:claude-code-2-1-304-declared
kind: story
status: active
title: The Claude Code adapter declares 2.1.304 and the built-in plugins it ships
refs:
- provider: atlas
  reference: adr-0096
relations:
- decomposes: epic:loom-agent-work-through-metaharness
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/metaharness-claude
- confidence: inferred
  path: crates/metaharness-claude/fixtures/c1/api-key.json
- confidence: inferred
  path: crates/metaharness-claude/fixtures/c1/strict-hermetic.json
- confidence: cited
  path: crates/metaharness-claude/src/launch.rs
- confidence: cited
  path: crates/metaharness-claude/src/lib.rs
- confidence: inferred
  path: crates/metaharness-claude/src/vectors.rs
- confidence: cited
  path: crates/metaharness-claude/tests/adapter.rs
- confidence: cited
  path: crates/metaharness-claude/tests/native_fixture.rs
- confidence: cited
  path: crates/metaharness/fixtures/golden/contract-result-claude.json
- confidence: inferred
  path: crates/metaharness/src/audit.rs
- confidence: inferred
  path: crates/metaharness/src/builder.rs
- confidence: inferred
  path: crates/metaharness/src/run.rs
- confidence: inferred
  path: crates/metaharness/tests/adversary_b10x_h7_resolver_pass2.rs
- confidence: inferred
  path: crates/metaharness/tests/audit.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: cited
  path: docs/research
- confidence: inferred
  path: website/docs/harnesses/claude.md
- confidence: inferred
  path: website/docs/status.md
revision: 24
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T20:30:18Z", actor: "human:timo", revision: 23}
- {from: "proposed", to: "active", at: "2026-10-20T20:30:18Z", actor: "human:timo", revision: 24}
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

Derived 2026-10-20 by `aep:story-scoper` against `6b984c46` and the installed binaries 2.1.302-2.1.304
(2.1.259 is not on disk). Cited unless marked inferred.

- **Built-ins in 2.1.304:** 12 names compiled into the binary (`rg -a` over the installed version):
  `cc-plugin-agents-md`, `cc-plugin-telemetry`, `cc-plugin-plugin-authoring`, `cc-plugin-sec-default`,
  `mods-guide`, `mermaid`, `responsive-mode`, `easel`, `tips`, `diff`, `you-should-know`, `claude-test`.
  In a fresh offline scratch home (`unshare --user --net`, `CLAUDE_CONFIG_DIR` at scratch), `claude plugin
  details <name>@builtin` resolves 3: `agents-md`, `telemetry` (not with
  `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1`, which loopback mode already sets, `launch.rs:1241`), and
  `plugin-authoring`; the same 3 as 2.1.288 (`docs/research/2026-10-03-native-claude-fixture.md:34-35`).
  `sec-default` is policy-only (managed settings); four sit behind server flags that default false.
  `claude plugin list --json` prints `[]` (it never lists built-ins). Built-ins cannot be removed by
  scratch-home isolation: they are compiled in. takt's relayed count of 4 is not confirmed for the
  credential-free surface; the 4th is likely account-dependent (inferred).
- **H1a today:** compares the record's plugin names with `attestation.installed_plugins`
  (`crates/metaharness/src/audit.rs:444-464`, `builder.rs:1498`), ignoring `source`; `InstalledPlugin`
  requires a digest (`plugin.rs:127`), so any built-in in the record reads `gap`. H9 is `ok` only when the
  record's version is in `PINNED_VERSIONS` (`audit.rs:695`).
- **Pin surfaces:** `crates/metaharness-claude/src/lib.rs:127`; `crates/metaharness-claude/tests/adapter.rs:48`;
  `crates/metaharness-claude/tests/native_fixture.rs:514` (literal `2.1.288`), `:564-568` (three built-in
  names); `crates/metaharness/fixtures/golden/contract-result-claude.json` (`provider: claude 2.1.259`).
- **Launch and attestation:** `crates/metaharness-claude/src/launch.rs:1468-1509` (`attest`), `:1634`
  (`plugin_posture`), `:1166` (`build_env`), `:1435` (`build_settings`).
- **Inferred, if built-ins are declared and checked:** `crates/metaharness/src/audit.rs:432-466` and
  `FloorInputs` (`:39-44`); `crates/metaharness/src/run.rs:209-221`; `builder.rs:733,904,1147,1498`;
  `FloorInputs` literals in `crates/metaharness/tests/audit.rs:80`,
  `crates/metaharness/tests/adversary_b10x_h7_resolver_pass2.rs:57`.
- **Inferred, if the launch always sets the traffic switch:** `crates/metaharness-claude/fixtures/c1/api-key.json`,
  `fixtures/c1/strict-hermetic.json`; if a vector is added, `src/vectors.rs` (golden `checked` 28 → 29).
- **Not touched (inferred):** `src/marketplace.rs` (built-ins are not in the registry).
- **Documents:** `docs/research/<date>-claude-code-2.1.304-adapter-surface.md` (new; note `--max-turns` is
  missing from 2.1.304's `--help` though present in the binary), `CHANGELOG.md`, a design amendment
  superseding `docs/design/metaharness-protocol-v0.1.md:2149` ("retain its older pin"); inferred:
  `website/docs/harnesses/claude.md:9`, `website/docs/status.md:20`.
- **Safety fact:** H1a matches names only (`audit.rs:445-448`); a built-in declared by name alone would let
  an ambient plugin of the same name pass, so the comparison must require `source == "<name>@builtin"`.
- **Estimate:** pin alone: 1 constant, 1 test literal, 1 golden string, 2 native-fixture literals; with
  declared built-ins: about 4-6 new tests and 2 `FloorInputs` literal sites. No paid run: the native
  fixture qualifies credential mode `none` (and should add `loopback`).

## Ambiguities

- `requires-stakeholder-input` — what H1a ("plugins are exactly the declared set") means for built-ins,
  whose set varies with environment, account and server flags. Default taken by the coordinator
  (2026-10-20, reversible in the amendment): the launch keeps disabling what it can (the traffic switch
  for `telemetry`); H1a reads `ok` when every plugin in the opening record is either a declared installed
  plugin (digest-checked, as today) or a built-in matched on `source == "<name>@builtin"` whose name is in
  the pinned version's compiled built-in list, and the floor names each built-in it saw; any other plugin
  is a `gap`. The Acceptance's "each built-in plugin 2.1.304 ships" reads as "each built-in seen in the
  2.1.304 opening record per credential mode driven (`none`, `loopback`)"; operator-login sessions are
  unverified until takt's own `session.started` plugin list is read.
## Observed in an operator-login session (relayed 2026-10-20)

`b10x-chef-orchestrator` read takt's Claude 2.1.304 vendor stream (system/init event, `claude_code_version`
2.1.304, in takt's trial state directory): `plugins` lists four, each with source `<name>@builtin`:
`cc-plugin-sec-default`, `cc-plugin-agents-md`, `cc-plugin-telemetry`, `cc-plugin-plugin-authoring`. The
fourth beside the credential-free three is `cc-plugin-sec-default`. This machine has no machine-level
managed settings (`/etc/claude-code/` does not exist, checked 2026-10-20), so the policy that enables it
reaches the session through the operator's account or organization (inferred). Under the H1a default
above it is in the pinned binary's compiled list and matched on its `@builtin` source, so it reads as a
named built-in, not a gap; the implementor pins that shape with a vector built from this list.
