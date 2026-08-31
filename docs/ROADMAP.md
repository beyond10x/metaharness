# Roadmap — operator-scheduled directions

Serves **O3** and **O6** of `atlas/ROADMAP.md`, the collection's objectives; this page orders the work inside this repository.

Brain-dumped by the operator on 2026-08-22 and recorded here so nothing lives only in a chat.
Each item is a direction, not a work order; the ones with design questions get a design page
before code (AGENTS.md's rule). Ordered roughly by how much they de-risk everything else.

## 1. Contract-test every adapter mapping

Each adapter is a mapping `metaharness <-- adapter --> vendor` (claude, codex, xyz), and the
mapping should be **contract-tested**: the vendor side pinned by recorded wire samples, the
metaharness side pinned by the protocol's event/command vocabulary, and drift on either side a
red contract rather than a surprise in a paid run. Today's nearest things are the conformance
vectors (C1–C3) and the pinned-version doctor; a contract suite would make the pairing explicit
and per-adapter-symmetrical. **Open question:** reuse engineering-protocols' contract tooling
(`contract-testing` principle, `contract_result` evidence, its conformance crates) rather than
inventing a second harness-contract shape — the operator suspects yes; needs a short design
page mapping "adapter contract" onto that tooling's vocabulary.

**Answered and built, 2026-08-23.** `docs/design/adapter-contract-v0.1.md` is that page and its
four milestones are all built: the record (CT-1), recorded real wire on both faces (CT-2), the
version pair (CT-3) and the per-adapter authoring shape (CT-4). The reuse is the vocabulary and
not a dependency — `engineering-protocols` never appears in a `Cargo.toml` here. What the last
milestone surfaced is carried in that page: the two adapters were not symmetrical, because codex
tested no launch face, and the checklist said so out loud instead of leaving it absent. **That gap
is closed as of 2026-08-23** — six recorded C1 expectations, `checked: 10 → 17` on codex — and the
way it closed is the argument for the shape: an absence nobody could see became a declared gap,
then a deliberate count movement with the diff read.

**The b10x adapter now meets that standard too (2026-08-31).** Its free contract has one recorded
C1 launch, one byte-exact recorded loop replay, two `provider_emulated` enforcement excerpts and
one version-pair vector; its hook-input row is a reasoned N/A because this observe-only adapter has
no metaharness hook seam. The resulting `contract_result` is pinned as bytes at `checked: 5`. The
launch recording also caught and closed ambient profile discovery by putting `XDG_CONFIG_HOME` in
scratch, corrected the opening attestation from the generic `frame` default to `observe`, and the
strict audit now consumes the resolved binary's version and the loop's credential class rather
than adapter literals.

## 2. More harnesses: pi, opencode

Two further adapter crates on the same pattern as `metaharness-codex` (research record first,
pinned versions, rollout/transcript reader, capabilities with undriven claims labelled, live
proof last): **pi** and **opencode**. Sequence after the codex spawn (CX-M2) lands, so the
builder's kind-dispatch has two real implementations before it grows a third.

Whichever of the two comes first inherits CT-4's acceptance clause: it declares its contract by
filling `ContractObligations` — a launch vector, a recorded transcript/rollout vector, a recorded
hook-input vector, a version pair — and `contract_obligations(kind)` will not compile until it
does. The clause originally named flux as a third; § 3 struck it.

**Contract-first slice built, 2026-08-31.** `metaharness-pi` pins Pi 0.80.3 and
`metaharness-opencode` pins OpenCode 1.4.7. Each has a recorded C1 scratch-home launch and an
observed version-pair vector, and each declares the model-backed JSON wire and blocking hook input
as gaps. Neither is a `Kind` yet: the compiler-enforced dispatch clause applies when those two
recorded faces close, rather than letting an incomplete crate become a command somebody can spend
through. The locally verified facts and next probes are in
`docs/research/2026-08-31-*-adapter-surface.md`.

## 3. metaharness usage ≡ flux usage — **narrowed by the operator, 2026-08-23**

> Operator, 2026-08-23, verbatim: *"i dont want to embed any flux related"* — said while
> refusing a proposed `metaharness-flux` adapter.

Driving metaharness should feel the same as driving flux (`~/projects/flux`, the NDJSON-protocol
agent runner). The item's two readings now stand differently:
- **UX parity**: `metaharness run …` streams and steering behave like `flux run --stream-json`
  (failure classification, bounded resume, ground-truth rules for unattended runs). Parity is
  behavioural and embeds nothing; whether the operator's refusal reaches it too is **unclear —
  ask before starting it**.
- **flux as an inner harness**: ~~a `metaharness-flux` adapter~~ — **refused** by the statement
  above. Nothing flux-related is embedded in this repository.

## 4. Sandbox inversion

Messing with each harness's own sandboxing is tedious and per-vendor. The alternative: **wrap
the inner harness in a sandbox metaharness owns** (bubblewrap / landlock / container — to be
chosen), and disable the harness's own sandbox inside it entirely. One enforcement surface,
vendor-independent, and the hermetic rows (H3, H7, H11, network isolation that § 8.2 currently
refuses to claim) become impositions metaharness can actually make rather than knobs it hopes
each vendor has. Interacts with the loopback provider (`loopback-provider-v0.1.md`): a sandbox
that blocks all egress except the loopback proxy would make "the model API is the only network"
an attestable row. Needs its own design page; the a6 `--cwd` declaration and the credential
custody both change shape under it.

**Designed, 2026-08-31.** `docs/design/sandbox-inversion-v0.1.md` chooses an injected, typed
`ProcessEnvelope` boundary: metaharness owns and seals policy, while a substrate-backed provider
outside this repository performs confinement and returns measured facts. This repository does not
grow a substrate dependency or duplicate namespace code in every adapter. The first implementation
slice is protocol values plus a scripted port; no runtime confinement is claimed yet.
