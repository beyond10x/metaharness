# Executable contract and coverage boundary

This is a bounded retrofit of Metaharness's observable boundaries. It preserves
frame/1 and adapter ownership. It does not claim to specify every event,
credential rule, process-isolation mechanism or vendor version.

ESS 0.51.0 validates both manifests. The core model uses ess/15;
conformance.json is a declared-coverage ess-conformance/23 suite. Regenerate it:

```console
ess verify conform synthesize --path spec --suite-format 5 --strict-requires --out spec/conformance.json
```

`task check` validates both manifests, checks suite drift, and executes the
production target in `crates/metaharness-aep/src/drive/ess_conformance.rs`.
CI installs ESS 0.51.0 from commit
`0347ffa222939e3791e574d2dbe42d4b4b02d979`. Use that compiler locally.

The committed count is **27 selected scenarios**: 27 passed, zero failed,
skipped, unsupported or synthesis refusals. Unknown suite versions, steps,
bindings, shapes and predicates fail closed. The target handles only the
constructs this suite uses; extending the specification may require extending it.

| Obligation | Production observation |
|---|---|
| Terminal and unknown evidence | Adapter-owned synthetic inputs traverse RolloutReader; normalized is_error is read back and absent cost must remain None. |
| Stream closure | A real Run over ScriptedRunner produces closing reason and event count. No vendor process or model starts. |
| Native termination | The real protocol reader distinguishes missing, zero, nonzero and signal evidence. Separate Rust executable fixtures drive both real spawn runners through measured exit0, exit7 and SIGKILL, unavailable wait status, and terminal failure with exit0. |
| Frame integrity | The AEP driver mints a frame and Frame::parse_document reads it; modified and untagged frames are refused. |
| Tool decisions | Production Codex mapping and answer_events consult a real AEP engine; malformed, patch and unsupported calls are denied. |
| Workspace and file containment | The production name predicate and owned tool server operate on a selected directory; an outside symlink is refused. This does not prove process or cgroup isolation. |
| Accounting | Real finite and uncapped policy functions execute. Ledger views read files written by SpendBudget and AdmissionBudget, including persisted invocation count. |

The immutable Recorded entities expose existing observations: a closed stream and
snapshots of retained finite/admission ledgers. They add no running-session
lifecycle, AEP transition, public command, worktree ownership or mutation API.
Commands and events here are conformance operations. Closure values come from the
real run's final event; ledger values are read from disk, not from target bookkeeping.

The draft's three unobservable struct invariants now have observable views and
operations that produce their records. All three synthesis refusals were removed.
Indistinguishable outcome branches were combined so a target reports observed
results instead of picking a branch from the model's input guard. Authorization
evidence distinguishes missing, blank and nonblank references: the earlier
count > 0 guard incorrectly admitted whitespace-only authority.

## Reports

The target writes ess-conformance-results/1 statuses and invokes
`ess verify conform report` to produce `target/ess-conformance-report.json`
(ess-conformance-report/2). Its producer profile is
`external-scenario-status/1;runner=metaharness-rust-target@1`: Rust executed the
suite; ESS admitted it and assembled the report. The gate checks the native
report's conformance_status, not merely the converter's exit code. A compact
diagnostic report is also written beside it.

Mutation runs can explicitly select an emitted suite with METAHARNESS_ESS_SUITE
and an output path with METAHARNESS_ESS_REPORT. They use this same target.
Ordinary gate runs use the committed suite and count gate.

## Hardening evidence

The 2026-10-03 audit followed the planted-defect rule:

- Changing the specification's explicit-failure payload to success failed
  FinishCodexCompletion/outcome/explicit-failure. Restoration passed.
- Removing production admission persistence failed
  InvocationAdmission/invariant/at/metaharness.spending.RetainedAdmission/admission.
  Restoration passed. A target remembering only its intended count would miss this.
- `ess verify conform mutate --class guard-negate --emit ...`, execution of every
  emitted suite against production, and `--collect ...` killed **10 of 10 mutants**.
  No survivors, inconclusive, unwitnessed or equivalent mutants. Baseline: 24
  scenarios, zero refusals. Other mutation classes were not claimed.

Local design comparison found the unobservable invariants, indistinguishable
outcomes and blank-reference discrepancy above. It was coordinator review, not an
independent agent review: workers exhausted their host quota. It covered this
projection against amendments a18–a20, not every older design section.

The native-status extension under amendment a21 adds three wire-reading scenarios
to that 24-case baseline. Its producer is checked by actual credential-free Rust
subprocesses in `tests/native_termination.rs`; it does not qualify a vendor binary.
The ten-mutant audit above describes the original boundary model, not additional
mutants of the new native-status guards.

Random state-machine exploration, caller replay, exhaustive guard analysis,
determinism certification and metamorphic certification were not run. This model
contains bounded observations and immutable snapshots, not the upstream AEP state
machine. No released specification exists for a release-diff baseline. The suite
freshness check is not a breaking-change classifier.

## Exact types and remaining boundaries

The separate monetary manifest preserves Optional<Binary64> for the source's
Option<f64>. ESS 0.51.0 declines its conformance codecs and Rust synthesis; no
surrogate type or zero default is substituted. Production Rust tests cover unknown
and observed costs, failed persistence, crash/resume, corrupt ledgers, finite
narrowing and preservation of mode and authorization.

Core Rust synthesis produced 59 generated capabilities, four explicit obligations
and zero refusals; the generated contract crate compiled locally. Generated
behavior is not the production target and is not production evidence. Existing
runtime types remain authoritative in this retrofit; unsigned bounds and ordered
I/O also have production Rust tests.

The suite does not qualify new vendor versions. Paid probes and real transcripts
remain outside the gate and public source. Existing vendor pins remain unchanged
until their live compatibility evidence is complete.
