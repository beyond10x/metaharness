# The adapter contract — every `metaharness ⇄ vendor` mapping, contract-tested — v0.1 (proposed)

Status: **proposed** (operator brain-dump, 2026-08-22, `docs/ROADMAP.md` item 1), reviewed and
written decision-complete the same day. Companion to `metaharness-protocol-v0.1.md` (§ 8.5, the
conformance tiers this builds on) and to `engineering-protocols`' `contract-testing` principle,
whose vocabulary this reuses. The first slice (LP-equivalent CT-1) is built alongside this page;
**CT-2 and CT-3 are built** (2026-08-23 — one recorded capture run per adapter, then the version
pair reconciled and Q18 closed by protocol amendment a8), and **CT-4 is built** (2026-08-23 — the
authoring shape, both adapters declaring through it, and the record itself pinned as bytes). The
milestone table is closed.

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

§ 8.5's tiers are the contract's test cases; they run today (`conformance <kind>`, 20 claude / 10
codex vectors, free). What is missing is three things, in dependency order:

| # | milestone | content | acceptance |
|---|---|---|---|
| **CT-1** | **the record** | conformance emits a `contract_result` — `conformance <kind> --contract` prints it; a library `contract_result(kind, &[VectorOutcome])` builds it | **built with this page.** Both adapters emit a valid record; `provider` carries the pin; `breaking_changes ≤ failed`; a CLI test pins codex's provider string to `0.145.0` |
| **CT-2** | **recorded vendor samples as the contract** | today's C1/C2 vectors are synthesized in code; promote each adapter's to **recorded real wire** on disk (one captured hook input, one captured rollout/transcript), so a vendor-shape change is a red replay rather than a green test of a stale assumption. Capture is a one-time cost per pin | **built, 2026-08-23.** `--retain-dir` is the capture surface: the run copies its raw wire (transcript/rollout, hook inputs — never a credential) out of the scratch at wind-up. One hermetic capture run per adapter produced `fixtures/golden/` in each adapter crate: both faces, byte-exact expected streams, a `#[ignore]`d regeneration test per crate, and a mutation test proving a flipped byte fails its vector. The recorded wire immediately earned its keep twice: codex's real call arrives as `custom_tool_call` (the synthesized vectors used `function_call`), and its `session_meta` claims 0.144.0 out of the 0.145.0 binary — Q18 as a committed byte, warned as `version_outside_pin` in the golden stream |
| **CT-3** | **the version reconciliation (Q18)** | the pin is a pair — `doctor`'s `--version` source and the record's `cli_version` — and the contract asserts they agree, or names the gap. Closes Q18 | **built, 2026-08-23, and the investigation beat the milestone's own framing:** the pair disagreed because doctor and the spawn resolved **different binaries** — the operator's shell `PATH` finds pacman codex 0.145.0 at `/usr/bin`, the constructed child `PATH` finds npm codex 0.144.0 at `~/.local/bin` first. So the reconciliation is mechanical, not bookkeeping: `doctor` now resolves the vendor binary on **the child's `PATH`** (`child_path()`, exported by both adapters) and reports the resolved absolute path, and a `golden-version-pair` vector per adapter reads the recorded sample's own version claim against the pin — agreement passes silently, disagreement is a **named warning** (`warn C2 golden-version-pair — …`, and on stderr beside the `--contract` record), never a silent pass and never a failure. The acceptance clause holds today on both adapters: codex warns 0.144.0-vs-0.145.0, claude warns 2.1.240-vs-2.1.239. Q18 closed as protocol amendment a8; what remains — one install or two on the machine — is the operator's |
| **CT-4** | **symmetry across adapters** | one contract-vector authoring shape every adapter fills (claude, codex, and the next), so a new adapter's contract is a checklist, not a fresh invention | **built, 2026-08-23.** The shape is `ContractObligations` in `metaharness-protocol` (beside `conformance.rs`'s tiers): four rows — a launch vector, a recorded transcript/rollout vector, a recorded hook-input vector, a version pair — each answered `Filled(&[ids])` or `Gap(reason)`, with no `Default` and no optional field, so an adapter cannot be declared without answering all of them, and `contract_obligations(kind)` does not compile for a third adapter until it has one. Both adapters declare through it (`CONTRACT_OBLIGATIONS` in each crate's `vectors.rs`), and `crates/metaharness/tests/contract_symmetry.rs` checks each declaration against that adapter's *own* `conformance_vectors()` output and the `provider` its record carries — a named vector the run does not produce, produces in another tier or produces red is an unmet obligation, and so is a gap declared without a reason. **The original acceptance named pi/opencode/flux; flux is struck** (`ROADMAP.md` § 3, operator, 2026-08-23: *"i dont want to embed any flux related"*), pi and opencode do not exist yet, so the clause is **inherited by whichever adapter comes next**: it declares its contract by filling this shape, and it fills it before it is believed |

### What CT-4 found on its first run: codex tests no launch face

The checklist earned its keep the moment both adapters were made to fill it. Claude answers all four
rows; **codex answers three and has no launch vector at all** — no `fixtures/c1/`, so its argv and
child environment are pinned by the unit tests in `src/launch.rs` and by nothing a consumer can
read. Before the shape existed, that was invisible in exactly the way the `contract-testing`
principle warns about: the record said `checked: 10`, `failed: 0`, and said nothing whatsoever about
a face it never tested.

It is declared as `Obligation::Gap(reason)` rather than left absent, on the rule CT-3 already
established — **never a silent pass**. Filling it means recording the same launch expectations codex
already asserts in code, which moves `checked` and every vector-count pin; that is a deliberate
change, not a tidy-up, and it is not made in a wave whose whole point is that the record holds still.

### The record is pinned as bytes, because a consumer reads bytes

`engineering-protocols` ingests `conformance <kind> --contract` as evidence, and the two repositories
share a vocabulary and no code — the same gap the frame document has, closed the same way. Each
adapter's record is committed as the exact stdout of a live run
(`crates/metaharness/fixtures/golden/contract-result-<kind>.json`, recorded 2026-08-23 at CT-1..3 +
a9), and `crates/metaharness/tests/contract_golden.rs` rebuilds it through the real
`contract_result(kind, &conformance_vectors(kind))` and compares byte for byte, key order included.

Key order is part of the contract and not an implementation detail: nothing in the code asks
`serde_json` for sorted keys, so turning on its `preserve_order` feature anywhere in the workspace
would re-order every record this binary prints and the consumer would be the one to find out. A
`checked` that moves because a vector was added is legitimate — and the golden file is still
regenerated **deliberately**, through the `#[ignore]`d `regenerate_the_contract_records`, with the
diff read and the consumer told. Regenerating to restore green deletes the evidence of what moved.

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
