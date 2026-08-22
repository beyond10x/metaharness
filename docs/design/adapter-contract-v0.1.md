# The adapter contract — every `metaharness ⇄ vendor` mapping, contract-tested — v0.1 (proposed)

Status: **proposed** (operator brain-dump, 2026-08-22, `docs/ROADMAP.md` item 1), reviewed and
written decision-complete the same day. Companion to `metaharness-protocol-v0.1.md` (§ 8.5, the
conformance tiers this builds on) and to `engineering-protocols`' `contract-testing` principle,
whose vocabulary this reuses. The first slice (LP-equivalent CT-1) is built alongside this page;
everything past it is proposed.

## The idea, in the operator's words

> we map harness protocols (claude, codex, XYZ) to the semantics of meta-harness — do we make
> use of contract testing here? maybe even using tooling from engineering-protocols? So each
> adapter `metaharness <-- adapter --> [codex, claude, xyz]` must be contract tested.

## Verdict: yes, and the evidence that it is needed already exists

An adapter is a mapping with two faces, and either can drift:

```text
        the vendor's wire                     the protocol's semantics
  (transcript/rollout, hook input)  ── adapter ──▶  (metaharness.event/1, the decision seam)
        pinned by a version                    pinned by the event/command vocabulary
```

CX-M2 (the codex spawn, 2026-08-22) surfaced **three real drifts in one paid run**, and each is
exactly what a contract test exists to catch before a run costs money:

| drift | face | what a contract test pins |
|---|---|---|
| the hook's `tool_name` is `Bash`, not the `exec` the rollout uses or the `shell` the model-facing list uses | vendor wire | a recorded real hook input; the rendering table must agree with it |
| `codex --version` says `0.145.0`; the same binary's `session_meta.cli_version` says `0.144.0` (Q18) | vendor wire | the pinned version is a *pair of sources* that must be reconciled, not one string |
| the hook `tool_use_id` and the rollout `call_id` do not join | vendor wire | the correlation key the adapter claims must be the one the records share |

None of these is a bug in the adapter's logic; each is a fact about a vendor the adapter asserts
and could be wrong about. That is the definition of a contract.

## Reuse of engineering-protocols' tooling — the vocabulary, not a dependency

The established boundary holds: metaharness is private, engineering-protocols is public, **no
crate dependency crosses** (the migration kept the driver reading metaharness's *output*, never
linking it). So the reuse is the same shape as the frame document — a shared **vocabulary**, not
shared code:

- metaharness's conformance run emits a record in the **`contract_result` shape** EP already
  defines (`crates/aep-domain/src/evidence.rs`): `{checked, failed, breaking_changes, provider,
  consumer}`. An EP-driven eval, or any consumer, reads an adapter's conformance as a
  `contract_result` without knowing anything about metaharness's internals.
- the **discipline** transfers verbatim from the `contract-testing` principle: *a run that
  checked nothing also has zero failures, so the number of checks is part of the obligation*
  (`checked > 0`), and *the contract runner, not the author, decides whether anything moved*.
- what does **not** transfer is EP's `aep-contract` crate — that is the storage/interaction
  contract (CommandService/QueryService), a different contract entirely. Naming them the same
  word is the only thing they share.

### The `contract_result` fields, defined for an adapter

| field | meaning for `metaharness ⇄ vendor` |
|---|---|
| `provider` | the vendor and its pinned version, e.g. `codex 0.145.0` — the side that can move under us |
| `consumer` | `metaharness.event/1` — the protocol wire the adapter maps onto |
| `checked` | the number of conformance vectors run for this adapter (must be > 0, or the contract asserts nothing) |
| `failed` | vectors that did not match their expectation — any contract break, either face |
| `breaking_changes` | the subset of failures in the **vendor-facing tiers (C1 launch, C2 replay)** — a break here means *the vendor moved*, which is what breaks a consumer. A C3 failure is metaharness's own control machinery regressing, counted in `failed` but not here |

That split is the review's one real decision: `failed` is "the contract is red", `breaking_changes`
is "and it is the vendor's fault", and only the second is the number an operator pins a release on.

## What exists already, and what this adds

§ 8.5's tiers are the contract's test cases; they run today (`conformance <kind>`, 17 claude / 7
codex vectors, free). What is missing is three things, in dependency order:

| # | milestone | content | acceptance |
|---|---|---|---|
| **CT-1** | **the record** | conformance emits a `contract_result` — `conformance <kind> --contract` prints it; a library `contract_result(kind, &[VectorOutcome])` builds it | **built with this page.** Both adapters emit a valid record; `provider` carries the pin; `breaking_changes ≤ failed`; a CLI test pins codex's provider string to `0.145.0` |
| CT-2 | recorded vendor samples as the contract | today's C1/C2 vectors are synthesized in code; promote each adapter's to **recorded real wire** on disk (one captured hook input, one captured rollout/transcript), so a vendor-shape change is a red replay rather than a green test of a stale assumption. Capture is a one-time cost per pin | each adapter has ≥1 on-disk golden sample per face; a mutated byte fails its vector |
| CT-3 | the version reconciliation (Q18) | the pin is a pair — `doctor`'s `--version` source and the record's `cli_version` — and the contract asserts they agree, or names the gap. Closes Q18 | a recorded sample whose `cli_version` differs from the doctor pin is a named contract warning, not a silent pass |
| CT-4 | symmetry across adapters | one contract-vector authoring shape every adapter fills (claude, codex, and the next), so a new adapter's contract is a checklist, not a fresh invention | pi/opencode/flux adapters (`ROADMAP.md` 2–3) declare their contract by filling the shape |

## Decisions taken in this review (defaults if nobody objects)

1. **Vocabulary reuse, not a dependency** — metaharness emits the `contract_result` shape; EP
   never appears in its `Cargo.toml`. This is the only boundary-preserving option and it matches
   how the frame document already crosses.
2. **`breaking_changes` = vendor-tier failures only** — the C1/C2 subset. A C3 regression is
   metaharness's own and is `failed` but not breaking; conflating them would make every internal
   refactor look like a vendor outage.
3. **CT-1 ships now** because it is reversible, additive (a flag and a function), and independently
   useful regardless of CT-2–4's fate — it makes "an adapter's conformance is a contract" a fact a
   consumer can read today. The rest waits for this page's acceptance, per AGENTS.md.
4. **Recorded samples over synthesized (CT-2), but not retroactively urgent** — the synthesized
   vectors are honest today (their shapes were read off real installs); promotion to on-disk
   golden samples is the hardening that makes vendor drift *fail* rather than *go unnoticed*, and
   it is worth a milestone rather than a rewrite.
