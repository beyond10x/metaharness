---
format: aep.planning-md/3
id: story:codex-strict-record-rows
kind: story
status: active
title: '--hermetic strict can pass on Codex: H1a, H1b, H4 and H5 become decidable'
refs:
- provider: atlas
  reference: adr-0098
relations:
- decomposes: epic:loom-agent-work-through-metaharness
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/metaharness-codex/fixtures/c1
- confidence: inferred
  path: crates/metaharness-codex/fixtures/golden/rollout.expected.jsonl
- confidence: cited
  path: crates/metaharness-codex/src/launch.rs
- confidence: cited
  path: crates/metaharness-codex/src/rollout.rs
- confidence: inferred
  path: crates/metaharness-codex/src/vectors.rs
- confidence: cited
  path: crates/metaharness-codex/tests/native_fixture.rs
- confidence: inferred
  path: crates/metaharness-protocol/src/hermetic.rs
- confidence: inferred
  path: crates/metaharness/fixtures/golden/contract-result-codex.json
- confidence: cited
  path: crates/metaharness/src/audit.rs
- confidence: cited
  path: crates/metaharness/src/builder.rs
- confidence: inferred
  path: crates/metaharness/src/custody.rs
- confidence: inferred
  path: crates/metaharness/src/process.rs
- confidence: inferred
  path: crates/metaharness/src/run.rs
- confidence: inferred
  path: crates/metaharness/src/spawn_codex.rs
- confidence: inferred
  path: crates/metaharness/src/spawn_codex_vectors.rs
- confidence: inferred
  path: crates/metaharness/tests/adversary_b10x_h7_resolver_pass2.rs
- confidence: inferred
  path: crates/metaharness/tests/audit.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: inferred
  path: docs/research
- confidence: cited
  path: website/docs/harnesses/codex.md
- confidence: cited
  path: website/docs/status.md
revision: 26
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T20:30:18Z", actor: "human:timo", revision: 25}
- {from: "proposed", to: "active", at: "2026-10-20T20:30:18Z", actor: "human:timo", revision: 26}
---
# Story: `--hermetic strict` can pass on Codex: H1a, H1b, H4 and H5 become decidable

## Scheduling

Unit 3 of wave 4 (approved 2026-10-20 by `b10x-chef-orchestrator`), after `story:claude-code-2-1-304-declared`
(shared `audit.rs`, `FloorInputs`). Decision D1 of wave 4: B — no change to invariant 3 or amendment a13;
strict stays exit 3 on Codex and takt's Codex trials keep `--hermetic on`. Option A (decide H1a, H4, H5
from the launch's own observation) is recorded as open for Timo.

## Outcome

`metaharness run codex --hermetic strict` exits 0 on a clean run, because the four gating rows that
read the opening record (H1a plugins, H1b output style, H4 credential source, H5 MCP servers) are
decided for Codex, each from an observation in the rollout where Codex writes one, and otherwise from
what the launch imposed, labelled as metaharness's own claim, never as the vendor's record.

## Context

- Measured by `b10x-chef-orchestrator` with metaharness 0.9.3 and codex-cli 0.153.4 (evidence in that
  session's job directory, `codex-smoke/events.jsonl` and `stderr.txt`, read 2026-10-20):
  `metaharness run codex --hermetic strict --decisions observe -p …` exits 3. The opening record
  carries `plugins: null`, `output_style: null`, `credential_source: null`, `mcp_servers: null`. The
  floor prints, for example, `H1a unk gating record … the opening record carries no plugin list, and a
  missing list is not an empty one`, `H4 unk gating record … the opening record does not say where the
  credential came from`, `H5 unk gating record … the opening record carries no MCP server list, which is
  unk and never zero`.
- The same record's `hermetic.imposed` already states what the launch did for each of the four:
  `h1a` "CODEX_HOME=<scratch>/codex-home, and no plugin was injected: the declared set is empty",
  `h1b` "CODEX_HOME=<scratch>/codex-home", `h4` "neither OPENAI_API_KEY nor CODEX_API_KEY is in the
  child environment", `h5` "the scratch config.toml declares an empty [mcp_servers] and the scratch home
  has no other source; codex has no --strict-mcp-config, so this is the whole of it". H2 and H6 read
  `ok` as "metaharness imposed it" (advisory rows).
- Whether 0.10.1 behaves the same is inferred, not measured: nothing in waves 1-3 touched the Codex
  floor rows.

## Acceptance

On codex-cli 0.153.4 (the pin): H1b is decided from the record: the launch writes `personality` into the
scratch `config.toml`, the adapter reads the rollout's `turn_context.personality` (it arrives after the
opening record, so the floor reads the later event or the opening record is completed from it; the
implementor names which), and H1b reads `ok` when it equals what the launch wrote and `gap` otherwise; a
vector per shape and the regenerated `rollout.expected.jsonl` line show it. H1a, H4 and H5 stay `unk` on
Codex, and the floor's wording for each names why (the rollout carries no plugin list, credential source
or MCP server list at 0.153.4; deciding them from the launch is open under invariant 3). The H2 attestation
names only the sources the launch writes (`launch.rs:1229` names `hooks.json`, which `materialise_codex`
does not write, `builder.rs:1205-1208`). `metaharness run codex --hermetic strict` still exits 3, now with
H1b `ok`, named in the changelog. The 0.153.4 surface (default `personality`, the `apps` and
`remote_plugin` features, the config layers the binary names) is recorded in a research page, observed
where a credential-free run can observe it and labelled unverified otherwise. No paid run.

## Out of scope

- A paid run.
- Claude Code (`story:claude-code-2-1-304-declared`) and b10x rows.

## Ambiguities

- Resolved by D1 = B (`b10x-chef-orchestrator`, 2026-10-20): no record row is decided from the launch on
  Codex. The former default (an imposition with a scratch-home digest) is withdrawn; the scoper found its
  precondition false under a ChatGPT login (account plugins and `codex_apps` are not in the scratch home)
  and invariant 3 and a13 forbid it.

## Scope

- inferred: `crates/metaharness-codex/src/launch.rs` (attest, the scratch home), `crates/metaharness-codex/src/seam.rs`
  (the opening record), `crates/metaharness-codex/src/vectors.rs`, `crates/metaharness/src/audit.rs`
  (H1a, H1b, H4, H5 arms), `crates/metaharness-codex/tests/`, `docs/design/metaharness-protocol-v0.1.md`,
  `website/docs/hermetic.md`, `CHANGELOG.md`. To be scoped by `aep:story-scoper` before proposal.
