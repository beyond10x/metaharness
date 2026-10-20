---
format: aep.planning-md/3
id: story:b10x-adapter-on-harness-over-llm
kind: story
status: active
title: The b10x adapter pins the harness release that builds on llm
summary: HARNESS_REVISION, PINNED_VERSIONS and the three git pins move to the llm-based harness release, qualified with the credential-free native fixture and a research page.
owner: metaharness
tags:
- pins
refs:
- provider: atlas
  reference: adr-0097
relations:
- decomposes: epic:layer-pins-follow-their-owners
- depends_on: story:trace-ir-domain
scope:
- confidence: inferred
  path: .cargo/config.toml
- confidence: inferred
  path: .github/workflows/gate.yml
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: crates/metaharness-aep/src/drive.rs
- confidence: cited
  path: crates/metaharness-aep/src/drive_tests.rs
- confidence: cited
  path: crates/metaharness-b10x/Cargo.toml
- confidence: cited
  path: crates/metaharness-b10x/fixtures/golden/README.md
- confidence: cited
  path: crates/metaharness-b10x/fixtures/golden/loop.expected.jsonl
- confidence: cited
  path: crates/metaharness-b10x/fixtures/golden/loop.jsonl
- confidence: cited
  path: crates/metaharness-b10x/fixtures/golden/version.txt
- confidence: cited
  path: crates/metaharness-b10x/src/lib.rs
- confidence: cited
  path: crates/metaharness-b10x/src/seam.rs
- confidence: cited
  path: crates/metaharness-b10x/src/vectors.rs
- confidence: cited
  path: crates/metaharness-b10x/tests/native_fixture.rs
- confidence: cited
  path: crates/metaharness-cli/src/lib.rs
- confidence: cited
  path: crates/metaharness-tools/Cargo.toml
- confidence: cited
  path: crates/metaharness-tools/src/lib.rs
- confidence: cited
  path: crates/metaharness-tools/src/resolve.rs
- confidence: cited
  path: crates/metaharness-tools/src/server.rs
- confidence: cited
  path: crates/metaharness-tools/src/tests.rs
- confidence: cited
  path: crates/metaharness/fixtures/golden/contract-result-b10x.json
- confidence: inferred
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: cited
  path: docs/research
- confidence: cited
  path: evals/aep/driven.steps.yaml
- confidence: inferred
  path: spec/coverage.md
- confidence: inferred
  path: website/docs/harnesses/b10x.md
revision: 57
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T09:05:35Z", actor: "human:timo", revision: 40, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-20T09:05:36Z", actor: "human:timo", revision: 41, decided_on: {"recorded":{"review_outcome":2}}}
---
# Story: the b10x adapter pins the harness release that builds on llm

## Scheduling

Draft until wave 3 of 2026-10-20 took it as unit 1. The pin target is harness **0.15.0** (approval of
wave 3, `b10x-chef-orchestrator`, 2026-10-20), not 0.14.1: 0.15.0 deletes `b10x-harness-tools` and
`b10x-harness-toolchain` (harness tools-1; the catalogue layer moves to `b10x-harness-catalogue`)
and adds the public llm port (`story:llm-port-is-a-public-crate`). Held by
`dependency-blocker:harness-0-15-0-release` until the tag exists. Scoped against harness `main`
`947d0e482d7c8b14fb963bf29355b2c2af4d5a3a` (tools-1 merged, verified by `b10x-chef-orchestrator`).
Lands before the tools stories (decision D1 of wave 3: the edge `depends_on
story:metaharness-tools-on-tools-crates` was removed and reversed).

## Outcome

`metaharness-b10x` observes the in-house harness at the release where it builds on llm (ADR 0097
point 2), and every claim the adapter makes about that binary is pinned to that version and verified
against it or labelled unverified (invariant 4).

## Context

- The pin today: `PINNED_VERSIONS = ["0.13.3"]`, `HARNESS_REVISION = "798325f03cf5a18df8fadb346d31b314826136ec"`
  (`crates/metaharness-b10x/src/lib.rs:80,87`); two git pins in `crates/metaharness-b10x/Cargo.toml:14-19`,
  `b10x-harness-tools` and `b10x-harness-loop`, at the same rev; `b10x-harness-loop` is taken for
  `micro_usd_as_decimal` so the cost figure on the wire is rendered by the loop's own function. The
  `b10x-harness-tools` pin leaves with `story:metaharness-tools-on-tools-crates`; this story moves
  the loop pin.
- What changes when harness builds on llm: the wire and credential crates go (`harness-messages`,
  `harness-responses`, `harness-http`, `harness-credential`, ADR 0097 Context), so
  `credential_source`, the `usage` shape (llm's `cache_creation_input_tokens` and siblings) and cost
  rendering may move; each is read from the new record, not assumed (the 0.12.1 pin move found two
  claims that reading the code did not show: AGENTS.md § Live-evaluating, last bullet).
- The pin-move protocol is AGENTS.md § Live-evaluating: build the binary at the pinned revision,
  `--strict-version` refuses before spawn when the resolved binary is off the pin, and the research
  page records the surface (`docs/research/2026-09-15-b10x-harness-0.12.1-adapter-surface.md`).
  Since 2026-10-03 the credential-free native fixture (`task:b10x-native-fixture`, `docs/research/
  2026-10-03-native-codex-fixture.md` pattern) qualifies a pin without a paid run; a paid probe is
  outside the standing approval for this work and would need its own spend authorization.

## Acceptance

`task check` exits 0 with `HARNESS_REVISION`, `PINNED_VERSIONS` and every harness git pin in
`crates/metaharness-b10x/Cargo.toml` and `crates/metaharness-tools/Cargo.toml` naming the harness
0.15.0 tag's commit (reachable from harness main), `b10x-harness-tools` replaced by
`b10x-harness-catalogue` in both, `Cargo.lock` holding one harness revision and one tools revision,
and `git grep -n 'b10x.harness.tools'` outside `CHANGELOG.md` and `.engineering/` returning 0 hits; the recorded golden
loop and version-pair vectors are re-recorded against that binary with harness's own emulator and the
`metaharness.trace` normalize scenario over the b10x golden record still passes; the credential-free
native fixture passes against it; `docs/research/<date>-b10x-harness-0.15.0-adapter-surface.md`
lists every adapter claim as verified or unverified against that release, including the one reply
change harness names for the file operations now run on the tools crates (`dir_list` of a
non-regular entry: `kind:"other"`, `bytes:null`); `CHANGELOG.md` names the pin move, the crate swap
and the research page.

## Order

Lands first in the chain. All harness git pins in both manifests move to the 0.15.0 commit together,
so `Cargo.lock` never holds two harness revisions. At 0.15.0 metaharness's served file operations run
on the tools crates through harness's adapter (harness `CHANGELOG.md:12-21`), at whatever tools tag
harness pins (0.1.0 at `947d0e48`, `Cargo.toml:37-40`; 0.2.0 if it is tagged before the cut, harness
plan `docs/plans/2026-10-20-wave-tools-1.md:98-102`). `story:metaharness-tools-on-tools-crates`
follows on the same four `crates/metaharness-tools/src` files. Depends on `story:trace-ir-domain`
(implemented). `story:b10x-record-carries-the-loops-cwd` follows on `seam.rs`, `vectors.rs`,
`loop.expected.jsonl`; `story:call-seam-for-harness` follows on `crates/metaharness-b10x/src/{lib,seam,vectors}.rs`.

## Out of scope

- The direct tools dependencies, contract names and schemas (`story:metaharness-tools-adopts-tool-contract`,
  `story:metaharness-tools-on-tools-crates`); this story only swaps the harness crate name.
- Any paid run. A probe that spends requires `METAHARNESS_LIVE=1`, an explicit budget and the
  operator's spend authorization (invariant 5; standing approval excludes spending). The golden is
  re-recorded with harness's emulator (`crates/harness-cli/tests/fixtures/fake_responses.py` at
  `947d0e48`; the llm-port step may move it to `crates/harness-llm`) on loopback with
  `--credentials none`.

## Scope

Derived 2026-10-20 by `aep:story-scoper` against metaharness `3d697011` and harness `947d0e48`
(harness main, tools-1 merged); 0.15.0 adds the llm port on top. Cited unless marked inferred.

- **Pin constants:** `crates/metaharness-b10x/src/lib.rs:80,87`: `0.15.0` and the tag's commit, not
  the tag object (`crates/metaharness-aep-eval/src/lib.rs:1160` compares with `git rev-parse HEAD`);
  doc `:102-108` to Harness 0.15.0 and Substrate 0.7.17 `00fa634c` (harness
  `crates/harness-substrate/Cargo.toml:29-30`).
- **b10x manifest:** `crates/metaharness-b10x/Cargo.toml:14-23`: `b10x-harness-tools` (`:19`) becomes
  `b10x-harness-catalogue`, the crate exporting `entry_names` (harness
  `crates/harness-catalogue/src/lib.rs:59`, `catalogue.rs:931`, same 7 entries); `b10x-harness-loop`
  (`:23`) changes rev only; comment `:14-18` rewritten.
- **Seam import path:** `crates/metaharness-b10x/src/seam.rs:934,938,1116`: `b10x_harness_tools::entry_names`
  becomes `harness_catalogue::entry_names` (`[lib] name = "harness_catalogue"`, harness
  `crates/harness-catalogue/Cargo.toml:11-12`); `:396` unchanged (`micro_usd_as_decimal`,
  `crates/harness-loop/src/price.rs:253`). Inferred: stale text `:190-196`, `:226-243`, dead fallbacks
  `:380`, `:472`, tests `:604`, `:838`, `:1077`.
- **tools manifest and imports:** `crates/metaharness-tools/Cargo.toml:14-25` (`:24` becomes
  `b10x-harness-catalogue`, `:25` `b10x-harness-wire` changes rev only; the comment at `:14-23` is false
  at 0.15.0, harness `crates/harness-catalogue/Cargo.toml:17-26`); `src/lib.rs:47-49`,
  `src/resolve.rs:29,113,135`, `src/server.rs:5`, `src/tests.rs:114`: `b10x_harness_tools::` becomes
  `harness_catalogue::` (every symbol exported at harness `crates/harness-catalogue/src/lib.rs:59-67`).
  Decided by the coordinator: the real crate name, not a `package =` rename that would keep a key
  named after a deleted crate.
- **Lock:** `Cargo.lock:332-381`: `b10x-harness-toolchain` and `b10x-harness-tools` leave;
  `b10x-harness-catalogue` and `b10x-tool-{contract,fs,code-editor,toolchain}` at harness's tools pin
  arrive.
- **Invariant text:** `AGENTS.md:97-98` names `b10x-harness-tools`. Inferred: `.cargo/config.toml:3-8`,
  `.github/workflows/gate.yml:11-13` (tools now fetched through harness),
  `crates/metaharness-cli/src/lib.rs:183` (doc names `b10x-harness-tools`' path arithmetic).
- **Vectors:** `crates/metaharness-b10x/src/vectors.rs:153` (`reader()` stamps `0.9.1`); inferred
  `:254` if the capture names another tool than `file_read`.
- **Golden:** `crates/metaharness-b10x/fixtures/golden/{loop.jsonl,version.txt,loop.expected.jsonl,README.md}`,
  re-recorded at the 0.15.0 tag (a build at `947d0e48` reports `b10x-harness 0.14.1`, harness
  `Cargo.toml:17`, so the golden and `--strict-version` qualify only at the tag);
  `crates/metaharness/fixtures/golden/contract-result-b10x.json` (`"provider":"b10x 0.13.3"`,
  `crates/metaharness/tests/contract_golden.rs:38`).
- **Native fixture:** `crates/metaharness-b10x/tests/native_fixture.rs:505` (`"0.13.3"` in
  `assert_closed`); inferred: `:267-283` sends no `response.output_item.added`.
- **Documents:** `docs/research/<date>-b10x-harness-0.15.0-adapter-surface.md` (new), `CHANGELOG.md`
  `[Unreleased]`; inferred: an amendment in `docs/design/metaharness-protocol-v0.1.md`,
  `website/docs/harnesses/b10x.md:9,73`, `spec/coverage.md:211-212`.
- **The record:** the record-writing code is byte-identical from `f109612e` to `947d0e48`
  (`crates/harness-wire`, `crates/harness-loop/src/{event,price}.rs`, `crates/harness-cli/src/llm_port.rs`);
  `started` (`event.rs:114-201`) names no working directory; the converter writes `"cwd": Value::Null`
  (`crates/harness-cli/src/metaharness.rs:358`).
- **Behaviour under `metaharness-tools`:** file operations run on the tools crates through harness's
  adapter; only `dir_list` of a non-regular entry changes (harness `CHANGELOG.md:26-29`); the
  containment text is unchanged (`crates/harness-catalogue/src/local.rs:224`, tools
  `crates/tool-fs/src/error.rs:51`), so `ess_conformance.rs:513-516` holds. The tools crates at 0.1.0
  are `LicenseRef-B10x-Proprietary`, the class of the harness crates metaharness links today.
- **Shared, sequential:** with the tools-contract and tools-crates stories, the four
  `crates/metaharness-tools/src` files on the import lines, `AGENTS.md`, `.cargo/config.toml`,
  `.github/workflows/gate.yml`, `crates/metaharness-cli/src/lib.rs:183`, and `seam.rs:934-938,1116`.
- **Needs:** one harness build at the 0.15.0 tag, a loopback-only network namespace, a delegated
  cgroup; `METAHARNESS_NATIVE_B10X` takes an explicit binary path.
- **Estimate:** 5 fixtures change bytes (4 golden, `contract-result-b10x.json`); 2 certain test
  literals, 1 conditional; 9 import lines in 6 files.
- **Text naming `b10x-harness-tools`, stale once this lands (taken over from
  `story:metaharness-tools-on-tools-crates`, decision D5 of wave 3):** `AGENTS.md:97-99`,
  `crates/metaharness-cli/src/lib.rs:183`, `crates/metaharness-aep/src/drive.rs:1650,2464`,
  `crates/metaharness-aep/src/drive_tests.rs:314,1297`, `seam.rs:934`,
  `evals/aep/driven.steps.yaml:32`. The two manifest comments (`crates/metaharness-tools/Cargo.toml:14-23`,
  `crates/metaharness-b10x/Cargo.toml:14-18`) are deleted, not rewritten; the reasoning goes in the
  commit message.
