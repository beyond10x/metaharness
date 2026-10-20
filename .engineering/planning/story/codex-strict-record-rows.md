---
format: aep.planning-md/3
id: story:codex-strict-record-rows
kind: story
status: draft
title: '--hermetic strict can pass on Codex: H1a, H1b, H4 and H5 become decidable'
refs:
- provider: atlas
  reference: adr-0098
relations:
- decomposes: epic:loom-agent-work-through-metaharness
revision: 1
---
# Story: `--hermetic strict` can pass on Codex: H1a, H1b, H4 and H5 become decidable

## Scheduling

Draft, filed 2026-10-20 on a request from `b10x-chef-orchestrator`: takt's evals move to Codex (Timo,
about 18:10, relayed) and run under `--hermetic on` until strict passes. Not in wave 3; proposed after
it, beside `story:claude-code-2-1-304-declared`.

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

On codex-cli 0.153.4 (the pin), a clean `metaharness run codex --hermetic strict` run exits 0 with H1a,
H1b, H4 and H5 each reading a state that names its source: the rollout's own field where the rollout
carries one (read and cited per row), otherwise metaharness's imposition with the wording "metaharness
imposed it" and the attestation that backs it; a run whose scratch home is tampered with (a plugin, an
output style, an API key in the environment, an MCP server added after the launch wrote the home) still
fails strict on that row; a vector per row and per shape; the credential-free native Codex fixture
passes; a design amendment states which record rows may be decided from an imposition on which kind and
why that keeps invariant 3.

## Out of scope

- A paid run.
- Claude Code (`story:claude-code-2-1-304-declared`) and b10x rows.

## Ambiguities

- `requires-stakeholder-input` — whether a gating *record* row may read `ok` from metaharness's own
  imposition when the vendor's record cannot carry the field. Default: yes for Codex on these four rows,
  labelled as metaharness's claim, only when the launch both wrote the only source and observed it
  unchanged at spawn (a digest of the scratch home taken immediately before spawn, as H6 copies
  `auth.json`); otherwise the row stays `unk`. Decides: the design amendment's reviewer.

## Scope

- inferred: `crates/metaharness-codex/src/launch.rs` (attest, the scratch home), `crates/metaharness-codex/src/seam.rs`
  (the opening record), `crates/metaharness-codex/src/vectors.rs`, `crates/metaharness/src/audit.rs`
  (H1a, H1b, H4, H5 arms), `crates/metaharness-codex/tests/`, `docs/design/metaharness-protocol-v0.1.md`,
  `website/docs/hermetic.md`, `CHANGELOG.md`. To be scoped by `aep:story-scoper` before proposal.
