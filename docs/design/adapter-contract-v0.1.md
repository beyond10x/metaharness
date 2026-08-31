# The adapter contract — every `metaharness ⇄ vendor` mapping, contract-tested — v0.1 (proposed)

Status: **proposed** (operator brain-dump, 2026-08-22, `docs/ROADMAP.md` item 1), reviewed and
written decision-complete the same day. Companion to `metaharness-protocol-v0.1.md` (§ 8.5, the
conformance tiers this builds on) and to `engineering-protocols`' `contract-testing` principle,
whose vocabulary this reuses. The first slice (LP-equivalent CT-1) is built alongside this page;
**CT-2 and CT-3 are built** (2026-08-23 — one recorded capture run per adapter, then the version
pair reconciled and Q18 closed by protocol amendment a8), and **CT-4 is built** (2026-08-23 — the
authoring shape, every adapter declaring through it, and the record itself pinned as bytes). The
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

§ 8.5's tiers are the contract's test cases; they run today (`conformance <kind>`, 20 claude / 17
codex vectors, free — codex was 10 until its launch face was recorded on 2026-08-23; see the CT-4
section below). What is missing is three things, in dependency order:

| # | milestone | content | acceptance |
|---|---|---|---|
| **CT-1** | **the record** | conformance emits a `contract_result` — `conformance <kind> --contract` prints it; a library `contract_result(kind, &[VectorOutcome])` builds it | **built with this page.** Both adapters emit a valid record; `provider` carries the pin; `breaking_changes ≤ failed`; a CLI test pins codex's provider string to `0.145.0` |
| **CT-2** | **recorded vendor samples as the contract** | today's C1/C2 vectors are synthesized in code; promote each adapter's to **recorded real wire** on disk (one captured hook input, one captured rollout/transcript), so a vendor-shape change is a red replay rather than a green test of a stale assumption. Capture is a one-time cost per pin | **built, 2026-08-23.** `--retain-dir` is the capture surface: the run copies its raw wire (transcript/rollout, hook inputs — never a credential) out of the scratch at wind-up. One hermetic capture run per adapter produced `fixtures/golden/` in each adapter crate: both faces, byte-exact expected streams, a `#[ignore]`d regeneration test per crate, and a mutation test proving a flipped byte fails its vector. The recorded wire immediately earned its keep twice: codex's real call arrives as `custom_tool_call` (the synthesized vectors used `function_call`), and its `session_meta` claims 0.144.0 out of the 0.145.0 binary — Q18 as a committed byte, warned as `version_outside_pin` in the golden stream |
| **CT-3** | **the version reconciliation (Q18)** | the pin is a pair — `doctor`'s `--version` source and the record's `cli_version` — and the contract asserts they agree, or names the gap. Closes Q18 | **built, 2026-08-23, and the investigation beat the milestone's own framing:** the pair disagreed because doctor and the spawn resolved **different binaries** — the operator's shell `PATH` finds pacman codex 0.145.0 at `/usr/bin`, the constructed child `PATH` finds npm codex 0.144.0 at `~/.local/bin` first. So the reconciliation is mechanical, not bookkeeping: `doctor` now resolves the vendor binary on **the child's `PATH`** (`child_path()`, exported by both adapters) and reports the resolved absolute path, and a `golden-version-pair` vector per adapter reads the recorded sample's own version claim against the pin — agreement passes silently, disagreement is a **named warning** (`warn C2 golden-version-pair — …`, and on stderr beside the `--contract` record), never a silent pass and never a failure. The acceptance clause held on both adapters the day it was built: codex warned 0.144.0-vs-0.145.0, claude warned 2.1.240-vs-2.1.239. **Claude's pair is since reconciled** (protocol amendment a11, 2026-08-23): the pin moved 2.1.239 → 2.1.240 to the version the recorded sample was already written by and a live run had driven end to end, so claude's `golden-version-pair` passes silently and the recorded bytes were not touched — which is the mechanism working, not the vector going quiet. Codex still warns, because there the two versions are two installs and no capture agrees with the pin. Q18 closed as protocol amendment a8; what remains — one install or two on the machine — is the operator's |
| **CT-4** | **symmetry across adapters** | one contract-vector authoring shape every adapter fills (claude, codex, and the next), so a new adapter's contract is a checklist, not a fresh invention | **built, 2026-08-23.** The shape is `ContractObligations` in `metaharness-protocol` (beside `conformance.rs`'s tiers): four rows — a launch vector, a recorded transcript/rollout vector, a recorded hook-input vector, a version pair — each answered `Filled(&[ids])` or `Gap(reason)`, with no `Default` and no optional field, so an adapter cannot be declared without answering all of them, and `contract_obligations(kind)` does not compile for a third adapter until it has one. Both adapters declare through it (`CONTRACT_OBLIGATIONS` in each crate's `vectors.rs`), and `crates/metaharness/tests/contract_symmetry.rs` checks each declaration against that adapter's *own* `conformance_vectors()` output and the `provider` its record carries — a named vector the run does not produce, produces in another tier or produces red is an unmet obligation, and so is a gap declared without a reason. **The original acceptance named pi/opencode/flux; flux is struck** (`ROADMAP.md` § 3, operator, 2026-08-23: *"i dont want to embed any flux related"*), pi and opencode do not exist yet, so the clause is **inherited by whichever adapter comes next**: it declares its contract by filling this shape, and it fills it before it is believed |

### What CT-4 found on its first run: codex tested no launch face — **closed 2026-08-23**

The checklist earned its keep the moment both adapters were made to fill it. Claude answered all four
rows; **codex answered three and had no launch vector at all** — no `fixtures/c1/`, so its argv and
child environment were pinned by the unit tests in `src/launch.rs` and by nothing a consumer could
read. Before the shape existed, that was invisible in exactly the way the `contract-testing`
principle warns about: the record said `checked: 10`, `failed: 0`, and said nothing whatsoever about
a face it never tested.

It was declared as `Obligation::Gap(reason)` rather than left absent, on the rule CT-3 already
established — **never a silent pass** — and the gap was closed in its own change, which is the point
of naming it rather than fixing it in a wave whose value was that the record held still.

**How it was closed.** Six recorded expectations under `crates/metaharness-codex/fixtures/c1/`,
declared `Obligation::Filled` and checked against the run that produces them: `c1-strict-hermetic`,
`c1-api-key`, `c1-loopback`, `c1-loopback-subscription-refusal`, `c1-unsupported-option-refusal`,
`c1-memory-ancestor-refusal`. Two of them are the loopback door's (`loopback-provider-v0.1.md`,
LP-4) and two are refusals codex has and claude does not, which is the shape the milestone predicted:
*one authoring shape, per-adapter content*.

The observation is **not** claude's. A codex launch vector records `program`, `args`, `env`, **the
whole scratch `config.toml`** and the credential-copy list, because on this vendor the seam, the
model provider and the sandbox posture are keys in a file rather than flags on a command line — a
vector that recorded only the argv would pin nothing about the hook, and an unrecognised key under
`[hooks]` is dropped *without failing the config load*. The copy list is in the observation for the
same reason: "how many credentials travel" is H6's claim and LP-4's upgrade both, and a loopback
run's empty list is the evidence rather than a comment about it.

**What it cost the record, deliberately:** `checked: 10 → 17` on codex (+6 launch, +1 the allow
round trip in the spawn tier), regenerated through the `#[ignore]`d `regenerate_the_contract_records`
with the diff read — one line, one field. Claude's record did not move: `checked: 20`, unchanged.
The consumer reading these bytes is told, which is the other half of the rule.

### The direct-provider adapter fills the same contract without inventing a hook — 2026-08-31

`b10x` is the first adapter added after CT-4 and the first whose far side is a loop this collection
owns rather than a vendor harness. It still owes both faces of the mapping. Three deterministic
vectors now pay the applicable rows: `c1-observe-launch` records the executable, argv and whole
child environment; `golden-loop-record` replays a real `b10x-harness 0.8.0 --json` record byte for
byte; `golden-version-pair` compares the capture's version banner with `PINNED_VERSIONS`. The
capture used harness's loopback-only deterministic Responses endpoint, so its evidence label is
`provider_emulated`, never `vendor_live`.

The loop record has no version field. CT-3 therefore pairs it with `b10x-harness --version` captured
from the same installed binary; adding a version field to the fixture would forge a fact the wire
did not state. The recorded-hook row remains a reasoned N/A: this adapter is observe-only and has
no metaharness decision hook. Fabricating hook input to make four rows say `Filled` would violate
the adapter boundary the contract is meant to protect.

The launch vector found two real drifts before the first golden was accepted. Harness 0.8.0 reads
an optional profile from the config home, so the b10x child now gets a scratch `XDG_CONFIG_HOME`
even when toolchain discovery requires `HOME`; otherwise the operator's `[default]` permission
profile silently changes the eval arm. And the opening attestation now says `decisions: observe`
instead of inheriting `HermeticAttestation::none`'s generic `frame` default. The golden was captured
only after both launch claims were true.

That mode is explicit at launch. `frame` and `ask` would claim a per-call control seam this adapter
does not own, so b10x refuses both and tells the caller to pass `--decisions observe`. The generic
capability check accepts that delivered mode without requiring `tool.decide`: vendor adapters
implement observe by allowing each hook request, while the direct-provider adapter observes calls
whose records already say `decision_required: false` and `seam: none`. Treating the two mechanisms
as identical made the only truthful b10x mode impossible to start.

### The record is pinned as bytes, because a consumer reads bytes

`engineering-protocols` ingests `conformance <kind> --contract` as evidence, and the two repositories
share a vocabulary and no code — the same gap the frame document has, closed the same way. Each
adapter's record is committed as the exact stdout of its deterministic conformance run
(`crates/metaharness/fixtures/golden/contract-result-<kind>.json`, recorded 2026-08-23 at CT-1..3 +
a9 for the vendor adapters and 2026-08-31 for b10x), and
`crates/metaharness/tests/contract_golden.rs` rebuilds it through the real
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
