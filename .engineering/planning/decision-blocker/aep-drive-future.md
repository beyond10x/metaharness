---
format: aep.planning-md/3
id: decision-blocker:aep-drive-future
kind: decision-blocker
status: cleared
title: Nobody has decided whether metaharness aep drive is kept, moved to Loom or retired
owner: metaharness
refs:
- provider: atlas
  reference: adr-0098
relations:
- blocks: story:aep-drive-disposition
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-20T02:17:32Z", actor: "human:timo", revision: 3}
---
## The question

Atlas ADR 0098 Consequences: "`metaharness aep drive` (crate `metaharness-aep`) is a governed-run
driver inside metaharness and overlaps Commission; its future is decided in metaharness's plan."
Nobody has decided. Recorded 2026-10-20 on the owner's instruction to write the options and their
costs and not act. Decides: the operator.

## Facts, read 2026-10-20

| fact | source |
|---|---|
| crate size 8,087 lines: `drive.rs` 2,710, `drive_tests.rs` 2,829, `drive/ess_conformance.rs` 1,001, `eval.rs` 438, `eval_tests.rs` 609, `drive/spending.rs` 286, `drive/spending_tests.rs` 196, `lib.rs` 18 | `wc -l crates/metaharness-aep/src/**` |
| pins eight AEP crates (`aep-cli`, `aep-domain`, `aep-driver`, `aep-driver-spec`, `aep-engine`, `aep-project`, `trace-domain`; `aep-schema` dev) at rev `6d7a44d3607d2d9a6ffdf0a165993c546c43d0db` | `crates/metaharness-aep/Cargo.toml:14-20,28` |
| hosts the ESS conformance target for all five domains of `spec/` (45 scenarios), run by `cargo test --workspace` in `task check` | `crates/metaharness-aep/src/drive/ess_conformance.rs`; `Taskfile.yml` |
| mints `metaharness.frame/1`, the preserved contract the adapters read | AGENTS.md § Safety envelope |
| is the driver `aep-eval driven` and `aep-eval native` run; those evals are the O3/O6 measurement | AGENTS.md § Serves, § Live-evaluating; `crates/metaharness-aep-eval` |
| AEP refuses `run` with a model, `transition` and `hook` by name and points at this binary since ADR 0047 | AGENTS.md § Live-evaluating; `aep/crates/edge/aep-cli/src/drive.rs` |
| files naming `metaharness aep drive`: agentplugins 40, atlas 26, aep 16, bench 8, mandate 1, harness 1 (92 outside this repository); metaharness 11 | `grep -rIl 'metaharness aep drive'` over `~/beyond10x/*`, excluding `target/` and `.git/` |
| Loom depends on metaharness, never the reverse; the session executor is built first as a standalone crate in takt | ADR 0098 point 3; ADR 0099 point 3 |

## Options

| | option | what it does | what it costs |
|---|---|---|---|
| A | keep | the crate stays in this workspace as the governed-run driver; `aep-eval` arms, the ESS target and the 92 external call sites keep working | two governors in the organization, named as an overlap by ADR 0098; the eight-crate AEP pin stays in the shipped binary and moves with every AEP release; the overlap is re-read at every Loom executor change |
| B | move to Loom | the driver's functions (frame minting, step map, governed transitions, spending ledger) move into Loom's runtime; metaharness keeps the adapters and the frame reader | Loom would take the eight AEP crates, which ADR 0098 does not give it; frame minting (Loom) and frame reading (metaharness adapters) split across repositories, a coordinated migration under the atlas rule with its own ADR; the ESS target is re-homed into another crate here first; `aep-eval` is rewritten to drive through Loom; 92 external files re-pointed; the `Aep` verb leaves `metaharness-cli` |
| C | retire | the crate and the `Aep` verb are removed once Loom's `AgentExecutor` runs a session end to end | `aep-eval driven` and `native` lose their driver until the executor replaces them, so the O3/O6 measurement stops in the meantime; AEP keeps ADR 0047's redirect to a binary that no longer has the verb until AEP changes; the ESS target is re-homed first; 92 external files re-pointed; the AEP pin leaves this workspace |

## Recommendation, not acted on

A until Loom's session executor exists and has run a case (ADR 0098 point 2–3; takt first, ADR 0099
point 3), then decide between B and C on that executor's measurements. Reason: C before the
replacement exists removes the only governed-run path the organization has; B puts AEP crates into
Loom, which ADR 0098 does not ask for. Default on silence: A. Nothing here moves until the operator
answers; this blocker is cleared by that answer and by nothing else.

## What it blocks

`story:aep-drive-disposition`, which is rewritten to the chosen option when this clears.
## Decision

Resolved 2026-10-20: option A, keep `metaharness aep drive` until Loom's session executor runs a
case (ADR 0098 point 2–3; takt first, ADR 0099 point 3). Timo's answer, verbatim, as relayed by
session `adr-changes` the same day: "a". The relay: "Timo decided decision-blocker:aep-drive-future:
A, keep `metaharness aep drive` until Loom's session executor runs (his answer "a", 2026-10-20)."
Cleared on that answer and on nothing else. The B-versus-C question is re-asked when that executor
exists and has measurements; nothing moves before then.
