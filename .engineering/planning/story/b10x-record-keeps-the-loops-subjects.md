---
format: aep.planning-md/3
id: story:b10x-record-keeps-the-loops-subjects
kind: story
status: draft
title: A b10x record keeps the subject the loop reported for each call
refs:
- provider: atlas
  reference: adr-0098
relations:
- decomposes: epic:loom-agent-work-through-metaharness
- informed_by: review-result:adr0096-w3-contract-names-adversary-2
revision: 1
---
# Story: a b10x record keeps the subject the loop reported for each call

## Scheduling

Draft, filed 2026-10-20 from the adversary's second pass on wave 3 unit 3
(`review-result:adr0096-w3-contract-names-adversary-2`, finding P2-1, `pre-existing`). Not in wave 3.

## Outcome

On a b10x run, metaharness's `tool.requested` names the file a call touched (its subject), as the loop's
own record does, so a frame's subject scope and the trace can judge where a b10x call acted.

## Context

- `Run::subjects` treats every b10x call as a call through `tool_invoke`: `publishes_verbs` is true for
  any b10x run (`crates/metaharness/src/run.rs:907-909`), and `subjects_of_verb` answers nothing for a tool
  that is not `tool_invoke` (`crates/metaharness-tools/src/resolve.rs:161-163`).
- The loop metaharness launches publishes its entries flat: the golden record's `published_tools` is
  `["file_read", …]`; `b10x-harness run --surface` defaults to `flat` (harness
  `f0870131:crates/harness-cli/src/lib.rs:744`); the b10x launch passes no `--surface`.
- The b10x adapter discards the subjects the loop recorded (`crates/metaharness-b10x/src/seam.rs:297-301`).
  The operation still resolves through the rendering table, so only the subject is lost.
- Second effect, read at source only: a made-up `tool_invoke {name:"file_read"}` on a flat run is refused
  by the loop but recorded as `file.read` plus its subject.
- Acceptance case already written, ignored until this lands:
  `crates/metaharness/tests/adversary_contract_names_b10x_pass2.rs::the_file_a_flat_b10x_call_read_is_on_metaharness_record`
  (the golden's `file_read {"path":"README.md"}` keeps `subjects: ["file:README.md"]`).

## Acceptance

On a b10x run, `publishes_verbs` follows the surface the loop published (flat unless the launch passed
`--surface verbs`), a flat call's subjects come from the loop's own record or from the entry's arguments,
a `tool_invoke` on a flat run reads as unknown (the loop refused it), the ignored acceptance case runs and
passes, and the b10x golden's expected stream changes only in the subjects it gains.

## Out of scope

- Claude Code and Codex.

## Scope

- inferred: `crates/metaharness/src/run.rs`, `crates/metaharness-b10x/src/seam.rs`,
  `crates/metaharness-tools/src/resolve.rs`, `crates/metaharness-b10x/fixtures/golden/loop.expected.jsonl`,
  `crates/metaharness/tests/adversary_contract_names_b10x_pass2.rs`, `CHANGELOG.md`.
