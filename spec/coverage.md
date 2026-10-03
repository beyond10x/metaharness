# Specification draft and coverage boundary

This is a typed draft for the issue-repair integration branch. It does not claim
that Metaharness conforms, and it does not describe every protocol event, adapter,
credential rule, sealed frame, tool decision or governed workflow.

`ess-inputs.yaml` pins ESS 0.51.0. `system.yaml` uses `ess/15`. The executable
projection has three domains:

| Domain | Authority and status |
|---|---|
| `metaharness.session` | Existing terminal/closure wire types, plus the required repair in the protocol design's amendment a18 dated 2026-10-03. Baseline code does not yet implement its unknown/failure rules correctly. |
| `metaharness.workspace` | Naming contract already supported by pinned Harness `90f10a4314c1c630691c85e812bd8d5d23d73fcc`; Metaharness's mirrored `ws_` check is the discrepancy being repaired. |
| `metaharness.spending` | Existing local finite terms/ledger and explicitly **proposed** uncapped types and authorization checks from issue 14. No claim that uncapped execution already exists. |

Commands are operations a conformance target must drive through the production
reader, run loop, eligibility check or spending code. They are not added public
CLI verbs. The two classification events are target observations of existing
results, not new runtime wire events. Read-only checks use effect-free outcomes.
No generated reference implementation can establish production conformance.

The draft uses existing/proposed structs rather than inventing an entity lifecycle
for a value. There are no declared ownership relations: AEP owns execution/step
lifecycle, worktree owns managed-tree identity and leases, and substrate owns
confinement. The proposal has no new authority to alter those owners.

## Exact monetary types are a separate, currently non-executable projection

`monetary/ess-inputs.yaml` validates the exact `Optional<Binary64>` observation
types. The source terminal wire uses `Option<f64>`, and unknown Codex cost is
`None`. These are kept as declared types, not replaced with decimal/string
surrogates or zero defaults. This remains part of the same Metaharness system;
the separate manifest identifies a compiler coverage boundary, not a separate
service or deployable.

ESS 0.51.0 refuses these types in conformance synthesis with
`UnsupportedPrimitive: finite Binary64 is not admitted by the current conformance
suite and codecs`, and in Rust generation with `this target has no qualified
finite Binary64 codec`. Main-model scenarios therefore exclude monetary values.
The `UNMAPPED:` comments mark this explicitly at both affected core types.

## Draft-generation observations

Core validation: `metaharness v1 — 4 file(s), valid`.
Monetary validation: `metaharness v1 — 2 file(s), valid`.

Core conformance synthesis produces 15 generated scenarios and three explicit
`ESS-SYNTH-013` refusals. Struct invariants on `StreamClosure`, `SpendLedger` and
`InvocationAdmission` currently have no observable view. Those rules remain in
the specification; a generated suite alone does not test them. Three synthesis
notes additionally say it has not proved disjointness/overlap coverage between
terminal outcome guards. No refusal or note is counted as a passing scenario.

Core Rust synthesis emits typed contracts with 27 generated capabilities and two
implementation obligations. This draft phase did not compile those artifacts,
implement a target or run the suite. Passed, failed and skipped counts are
unmeasured, not zero. Generated artifacts and detailed command output are retained
outside source by the integration coordinator.

## Required production-target work

- Drive `FinishCodexCompletion` through `RolloutReader::push_line` and `finish`,
  using only synthetic payloads. Drive `CloseUnsteeredStream` through the real
  scripted `Run`; assert normalized result, closure and CLI outcome together.
  The error-property discriminator preserves absent/null/nonnull: explicit null
  is positive no-error evidence; any nonnull value is failure. With no error key,
  only a nonblank string last message is positive; blank/malformed/absent is unknown.
  These rules were reconciled with the issue-11 implementor's completion helper.
  Stream closure preserves legacy `subtype: success` as positive evidence unless
  an explicit error or budget stop takes precedence, matching the same repair.
- Drive `CheckSelectedName` through the shared production predicate, then add
  filesystem scenarios for canonical selection, actual substrate admission,
  symlink containment and managed-tree identity/lease preservation. Typed name
  syntax is never evidence that the process was confined.
- Drive finite terms through production preflight and real ledger reservation.
  The three declared struct invariants need observable production state, not a
  target-maintained mirror. Proposed uncapped checks have no runtime target until
  issue 14 is implemented.
- Keep scenario IDs explicit; an unknown/unimplemented scenario must not pass.
  Real-target runs must record suite identity and passed/failed/skipped counts.
  Keep paid vendor probes and real transcripts outside the source gate.

Named integration scenarios still required beyond this core suite include
`codex-partial-text-then-error-fails`, `codex-missing-terminal-is-incomplete`,
`codex-null-error-legacy-success`, `audited-and-unaudited-exit-agree`,
`managed-wt-directory-confined`, `relative-project-equivalent-to-canonical`,
`symlink-escape-refused`, `managed-tree-identity-and-lease-preserved`,
`reservation-precedes-spawn`, `reservation-persistence-failure-spawns-nothing`,
`finite-resume-cannot-become-uncapped`, and
`uncapped-resume-preserves-authorization`. These are a handoff list, not claims
that corresponding authored scenarios already exist.

Every `UNMAPPED:` comment in the domain files is outstanding. In particular,
whether explicitly authorized uncapped-to-finite resume narrowing is supported
remains undecided; this draft grants no policy transition. Invocation identity,
observation update ordering and crash recovery are also open implementation-design
obligations. Current finite defaults and upstream retry/iteration bounds remain
the baseline requirements.
