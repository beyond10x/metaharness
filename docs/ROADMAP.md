# Roadmap — operator-scheduled directions

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

## 2. More harnesses: pi, opencode

Two further adapter crates on the same pattern as `metaharness-codex` (research record first,
pinned versions, rollout/transcript reader, capabilities with undriven claims labelled, live
proof last): **pi** and **opencode**. Sequence after the codex spawn (CX-M2) lands, so the
builder's kind-dispatch has two real implementations before it grows a third.

## 3. metaharness usage ≡ flux usage

Driving metaharness should feel the same as driving flux (`~/projects/flux`, the NDJSON-protocol
agent runner). Two readings, both probably wanted, to be split in a design page:
- **UX parity**: `metaharness run …` streams and steering behave like `flux run --stream-json`
  (failure classification, bounded resume, ground-truth rules for unattended runs);
- **flux as an inner harness**: a `metaharness-flux` adapter, since flux already speaks a clean
  machine protocol — likely the cheapest adapter of all and a good contract-test subject for #1.

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
