---
format: aep.planning-md/3
id: story:b10x-harness-pin-substrate-0-12
kind: story
status: active
title: metaharness admits the b10x-harness release on the newest Substrate
summary: Harness moves Substrate 0.7.17 -> the release after 0.12.0 and ESS 0.70.0; move the harness pin, re-record the golden from the released archive
tags:
- adr-0109
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-27T09:12:33Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-27T09:12:33Z", actor: "human:timo", revision: 3}
---
# Story: b10x-harness-pin-substrate-0-12

## Outcome

`metaharness run b10x --strict-version` admits the Harness release that embeds the newest Substrate
release (the release after Substrate 0.12.0), and the harness crates metaharness compiles are that
release's.

## Context

The operator, 2026-10-27: "yes, we want to upgrade to latest substrate for: loom, takt, harness,
metaharness." metaharness reaches Substrate only through Harness: its lock holds no Substrate
crate, since `b10x-harness-catalogue`, `b10x-harness-loop` and `b10x-harness-wire` do not depend on
`b10x-harness-substrate`. The installed `b10x-harness` binary it drives does embed Substrate. Harness
`story:substrate-pin-0-12` moves that pin and ESS to 0.70.0; its next release is a pin-only hop here.
`b10x-chef-orchestrator` approved the chain under the grant (D80 and D81, 2026-10-27).

Checked on 2026-10-27 against the Harness change through a local path override: `task check`'s
tests pass apart from the two that pin the harness revision in the lock
(`the_lock_names_harness_684c99f5_and_tools_0_5_0_and_nothing_else`,
`cargo_resolves_exactly_the_pinned_harness`), which move with the pin.

## Acceptance

- Both manifests, the lock, `PINNED_VERSIONS` and `HARNESS_REVISION` name the new Harness release
  and its tag commit, and `task check` exits 0.
- The golden loop record is captured again from the released, checksum-verified archive against
  harness's emulator, and the replay's expected stream moves only in `harness_version`.
- A credential-free `metaharness run b10x --strict-version --decisions observe` against the
  emulator exits 0 on the new release.

## Out of Scope

- `crates/metaharness-aep-eval` starting itself under `systemd-run` (`src/lib.rs:1345`): moving it
  onto Substrate's `systemd` adapter would add a Substrate dependency metaharness does not have;
  ADR 0109 F2 A names such moves as later hops.
- A paid one-turn run.
