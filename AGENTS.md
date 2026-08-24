# AGENTS.md — metaharness

The contract for changing **this** repository. Org-wide rules — the naming convention, the
former-brand rule (atlas ADR 0001) and its four exemption categories, and the rule that renaming
anything another repo verifies is a coordinated migration with an ADR — live in
[`atlas/AGENTS.md`](https://github.com/beyond10x/atlas) and are not restated here.

`README.md` says what metaharness is and how to run one. This file says what must not break.

## What this repository owns

One interface to many agent harnesses: emit events, receive steering commands, run hermetically,
decide the tool surface per call. Library first (`metaharness`, builder API), binary second
(`metaharness-cli`, `metaharness run <kind>`).

## Invariants

Each is a claim that can be checked. Breaking one is a design change, not a refactor.

1. **`metaharness-protocol` depends on no adapter crate.** Its `[dependencies]` are `clap`
   (optional), `serde`, `serde_json`, `sha2` and nothing else. An adapter reaching into the protocol
   crate's dependency list inverts the seam the workspace exists to hold.
2. **Everything harness-specific lives in `metaharness-<kind>`.** A vendor field name, a vendor
   binary's flag, a vendor's transcript shape: in the adapter crate, nowhere else.
3. **Absence of evidence is not a property.** Hermeticity, tool restriction and denial behaviour are
   asserted from the run's own record — never from configuration, a directory layout or a flag that
   was passed. A run that pinned nothing is *nobody found out*, not *nothing moved*; that confusion
   made `--hermetic strict` unpassable once and is amendment a4.
4. **Every adapter claim about a vendor binary is pinned to a version and verified against it, or
   labelled unverified.** A vendor surface nobody has driven is documented as undriven.
5. **Nothing under `evals/` runs in `task check`.** A paid run is never part of a gate. The subject
   checkout is `EP_REPO` (default `~/beyond10x/engineering-protocols`).
6. **Nothing under `website/` runs in `task check`**, and the built site is never committed —
   publishing goes through the Pages deployment API, so `build/` never enters history. CI builds the
   site on every PR that touches it (`.github/workflows/docs.yml`); a broken link fails that build
   (`onBrokenLinks: 'throw'`).
7. **The `docs/` → `website/` split is one-way.** `docs/design/` and `docs/research/` carry the
   reasoning — open questions, the amendment record, per-claim evidence labels — and are not
   published. `website/docs/` states the conclusion and cites where the reasoning lives. Where the
   two disagree, **the design document is right and the site is stale.**
8. **A design decision is written before it is built.** The protocol wire (events out, commands in),
   the control seam, the hermetic contract and the adapter obligations are
   `docs/design/metaharness-protocol-v0.1.md`. A change to any of them amends that document, and the
   review's corrections are recorded in it.
9. **The b10x adapter decides nothing, and asserts that it decides nothing.** `metaharness-b10x`
   runs in `DecisionMode::Observe` only; every `tool.requested` it emits carries
   `decision_required: false` and `Seam::None`. Giving it a seam would put the driven arm's
   treatment on top of the observed arm and make the two differ in name only.

## Safety envelope

A run spawns a **vendor binary holding the operator's real credentials**. Everything below is that
boundary; changing any of it is its own change with its own review.

- **The hermetic floor.** A run shares credentials with the operator and nothing else: no ambient
  plugins, no account-level MCP servers, no inherited environment. A scratch config home is how it
  is achieved; the transcript is how it is proven. Never soften a floor verdict to make a run pass —
  that is what invariant 3 exists to stop.
- **Per-call tool decisions.** Which tools the harness may call is decided per call, by the
  embedder, through the protocol — never once at launch. A decision path that answers without
  consulting the embedder is a silent allow.
- **`metaharness.frame/1` is a cross-repository contract.** The frame is minted by
  `engineering-protocols`' driver and consumed here: digest-verified on load, and refused **by name**
  when unreadable, untagged, misshapen or edited after sealing. Never widen that reader to accept a
  document failing any of the four checks, and never reorder tag → shape → digest. The other side
  pins the same bytes; changing the format is a coordinated migration under the atlas rule, not an
  edit.
- **Credentials are copied per spawn, never logged.** The Codex adapter copies the operator's
  `auth.json` into a scratch `CODEX_HOME` per spawn. Nothing in an event, a refusal message or a
  retained transcript may carry a credential value.
- **A retained transcript is evidence and may contain anything the vendor wrote.** Treat it as
  sensitive; never commit one that came from a real session.

## Out of scope

| Belongs elsewhere | Repo |
|---|---|
| An agent loop of our own — turn assembly, tool round trips, budgets | `harness` |
| Sandboxed execution, confinement, the operation ledger | `substrate` |
| Terminating LLM requests, model routing, backends | `llmgw` |
| The methodology specification, workflows, evidence semantics | `engineering-protocols` |

metaharness drives a loop; it does not have one. The `metaharness-b10x` adapter *observes* the b10x
harness and does not implement it.

## The gate

```console
task check
```

`cargo fmt --check`, then `cargo clippy --workspace --all-targets -- -D warnings`, then
`cargo test --workspace`. Green before any push.

**A green local gate does not guarantee a green CI.** The steps mirror each other; the toolchain
does not. CI installs whatever `stable` is that day, and a newer clippy can fail a commit that
passed locally on an older one. Run `rustup update` before pushing anything you will not get a
second chance at, and read the gate's own exit status — never a pipeline's. `task check 2>&1 | tail`
reports `tail`'s status, not the gate's.

## Releases

Cut `CHANGELOG.md` under a version heading at a fully gated `main` commit, then write an annotated
tag whose name is the bare version — `0.1.0`, the version and nothing else (atlas § *Naming*).
The full gate comes first; component steps alone are not enough.

## Where work is tracked

| What | Where |
|---|---|
| Directions the operator has scheduled | `docs/ROADMAP.md` |
| Binding designs and their amendment records | `docs/design/` |
| Investigations behind a design | `docs/research/` |
| What shipped | `CHANGELOG.md`, and `git tag -n99` |
| The evaluation machinery and its results | `evals/` (never in the gate) |

## Conventions

- Rust CLIs use `clap`'s derive API. Hand-rolled argv parsing is not accepted.
- Task runner is `Taskfile.yml` (go-task). Do not add a Makefile.
- A claim on the public site obeys the same rule as a claim in the code: pinned to a version and
  verified against it, or labelled unverified.
