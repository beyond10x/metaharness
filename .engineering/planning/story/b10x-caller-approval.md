---
format: aep.planning-md/3
id: story:b10x-caller-approval
kind: story
status: implemented
title: b10x frame and ask decide through the loop's caller approver
summary: 'ADR 0104 point 3, metaharness half: the loop asks over --approve caller and metaharness answers'
relations:
- depends_on: story:b10x-harness-pin-0-20-0
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-26T12:38:25Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"test_result":1,"approval":1,"verification":1}}}
- {from: "proposed", to: "active", at: "2026-10-26T12:38:25Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1,"approval":1,"verification":1}}}
- {from: "active", to: "implemented", at: "2026-10-26T12:38:25Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"approval":1,"verification":1}}}
---
# Story: b10x-caller-approval

## Outcome

Under `--decisions frame` or `ask`, metaharness decides each write and each `run` of the b10x loop,
through the loop's caller approver, and says which calls it cannot decide.

## Context

Atlas ADR 0104 (accepted 2026-10-23, option A) point 2: "metaharness (L2) is the one layer that
decides per-call tool permission." Point 3: "L1 enforces and does not decide." Harness 0.20.0's
`story:approver-delegates-to-caller` is the L1 half: `run --approve caller` writes
`approval-required` on its `--json` record and reads one `allow`/`deny` line on stdin. This story is
the metaharness half for the b10x adapter. `b10x-chef-orchestrator`, 2026-10-26: "plus ADR 0104
point 3 (metaharness caller policy: the metaharness side of the delegated approval; read the ADR on
atlas origin/main and the harness story:approver-delegates-to-caller) as a story in the same wave".
Design: amendment a61. Depends on `story:b10x-harness-pin-0-20-0`.

## Acceptance

- `frame` and `ask` launch the loop with `--approve-up-to low --approve caller` and a stdin pipe;
  `observe` keeps its launch and decides nothing (`c1-caller-launch`,
  `the_caller_launch_differs_from_observe_only_in_its_approver`).
- An asked call reaches the run as `tool.requested` on `control_request`, and the decision is
  written to the loop as the one line its grammar accepts; `abstain` and `replace` are written as a
  deny that says why (`provider-emulated-caller-allow`, `provider-emulated-caller-deny`,
  `tests/b10x_caller_seam.rs`).
- An `approval-resolved` that disagrees with the answer is `warning{code: "approval-disagreed"}`.
- A frame that does not admit `file.read`, `dir.list` and `search` in its operation set and every
  subject rule is refused at start, and so is a frame-mode run with no frame.
- Against the released 0.20.0 binary and harness's emulator, a frame denies a write
  (`decided_by: "frame"`, no file written), an embedder denies it over `tool.decide`, and `observe`
  lets it run.
- `capabilities b10x` declares `ask` and `frame` delivered and `tool.decide` honoured; every tier
  stays unverified.

## Out of Scope

- The loop's reads: they run unasked at every ceiling 0.20.0 offers.
- Applying a `replace`: ADR 0104 leaves open how a rewrite reaches L1.
- Moving `aep drive` to Loom (ADR 0104 point 1).
- A paid run.

## Ambiguities

None.

## Open Questions

- ADR 0104 § Open: "How a rewrite decided at L2 is reported to L1 so the agent loop sees the changed
  call." Until it is answered, `replace` fails closed.
