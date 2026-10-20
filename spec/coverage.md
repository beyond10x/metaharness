# Executable contract and coverage boundary

This is a bounded retrofit of Metaharness's observable boundaries. It preserves
frame/1 and adapter ownership. It does not claim to specify every event,
credential rule, process-isolation mechanism or vendor version.

ESS 0.56.0 validates both manifests. The core model uses ess/15;
conformance.json is a declared-coverage ess-conformance/35 suite. Regenerate it:

```console
ess verify conform synthesize --path spec --suite-format 5 --strict-requires --out spec/conformance.json
```

`task check` validates both manifests, checks suite drift, and executes the
production target in `crates/metaharness-aep/src/drive/ess_conformance.rs`.
CI installs ESS 0.56.0 from commit
`84ee8d38eb69e3a4507de801fdcb248232ed6e6e`. Use that compiler locally.

The committed count is **87 selected scenarios**: 87 passed, zero failed,
skipped, unsupported or synthesis refusals. Unknown suite versions, steps,
bindings, shapes and predicates fail closed. The target handles only the
constructs this suite uses; extending the specification may require extending it.

| Obligation | Production observation |
|---|---|
| Terminal and unknown evidence | Adapter-owned synthetic inputs traverse RolloutReader; normalized is_error is read back and absent cost must remain None. |
| Stream closure | A real Run over ScriptedRunner produces closing reason and event count. No vendor process or model starts. |
| Final answer, model selection and tool outcome | Adapter-owned synthetic inputs traverse the production Codex reader. Nine generated outcome scenarios read normalized terminal and tool fields; absent or contradictory evidence stays unknown. Source mappings are documented in `docs/research/2026-10-03-codex-observations.md`, with bounded native fixture observations documented separately. |
| Native termination | The real protocol reader distinguishes missing, zero, nonzero and signal evidence. Separate Rust executable fixtures drive both real spawn runners through measured exit0, exit7 and SIGKILL, unavailable wait status, and terminal failure with exit0. |
| Frame integrity | The AEP driver mints a frame and Frame::parse_document reads it; modified and untagged frames are refused. |
| Tool decisions | Production Codex mapping and answer_events consult a real AEP engine; malformed, patch and unsupported calls are denied. |
| Workspace and file containment | The production name predicate and owned tool server operate on a selected directory; an outside symlink is refused. This does not prove process or cgroup isolation. |
| Accounting | Real finite and uncapped policy functions execute. Ledger views read files written by SpendBudget and AdmissionBudget, including persisted invocation count. |
| Session format (`trace-ir/1`, domain `metaharness.trace`) | The production projection `project_document` and the three production readers run over committed inputs only: one framed event per wire kind, classified by the node the writer emits; completeness over three marker shapes built through the production `EventStream`; the digest over the framed bytes beside the vendor's reference; the two recorded `evals/aep/runs` streams retained as rendered documents whose view is read back from disk and re-projected; the three golden vendor records normalized through `ClaudeSeams`, `RolloutReader` and `B10xSeams`; the same three records through the verb's own library entry, `metaharness::project_vendor_record` (`metaharness project --from`, amendment a28), with the source name, the record digest, the unclaimed attestation and the census against the adapter's expected stream read off the rendered document, and the event stream's name refused as a `--from`. No vendor process or model starts. |

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
- Removing production final-answer extraction failed
  ReadFinalAnswer/outcome/authoritative (35 passed, one failed). Restoration passed.
- `ess verify conform mutate --class guard-negate --emit ...`, execution of every
  emitted suite against production, and `--collect ...` killed **18 of 18 mutants**.
  No survivors, inconclusive, unwitnessed or equivalent mutants. Baseline: 36
  scenarios, zero refusals. Other mutation classes were not claimed.

Local design comparison found the unobservable invariants, indistinguishable
outcomes and blank-reference discrepancy above. It was coordinator review, not an
independent agent review: workers exhausted their host quota. It covered this
projection against amendments a18–a22, not every older design section.
A local adversarial regression caught reused call identifiers invalidating internal
state while leaving the last public result as success. The implementation now
emits an unknown result and warning; the regression passes after first failing.

The native-status extension under amendment a21 adds three wire-reading scenarios
and a22 adds nine Codex observation outcomes to the original 24-case baseline.
Native-status production is checked by actual credential-free Rust subprocesses
in `tests/native_termination.rs`; it does not qualify a vendor binary. The final
18-mutant audit includes the additional native-status and Codex observation guards.
Six mutants add synthesis refusals, but each is killed by executed scenarios;
none scores unwitnessed or inconclusive. The unchanged baseline has zero refusals.

Random state-machine exploration, caller replay, exhaustive guard analysis,
determinism certification and metamorphic certification were not run. This model
contains bounded observations and immutable snapshots, not the upstream AEP state
machine. No released specification exists for a release-diff baseline. The suite
freshness check is not a breaking-change classifier.

## Exact types and remaining boundaries

The separate monetary manifest preserves Optional<Binary64> for the source's
Option<f64>. ESS 0.56.0 declines its conformance codecs and Rust synthesis; no
surrogate type or zero default is substituted. Production Rust tests cover unknown
and observed costs, failed persistence, crash/resume, corrupt ledgers, finite
narrowing and preservation of mode and authorization.

Core Rust synthesis produced 59 generated capabilities, four explicit obligations
and zero refusals; the generated contract crate compiled locally. Generated
behavior is not the production target and is not production evidence. Existing
runtime types remain authoritative in this retrofit; unsigned bounds and ordered
I/O also have production Rust tests.

The suite does not qualify new vendor versions. Paid probes and real transcripts
remain outside the gate and public source. Vendor pins advance only for the explicitly qualified surfaces recorded in the
current native reports; untested claims remain unverified.

## Final 0.9.0 extension

Amendment a23 adds five steering scenarios, and a24 adds four process-write
admission/argv scenarios. The combined45-scenario model passes with no skipped,
unsupported or refused obligations. Its guard-negation audit kills25of25mutants,
with no survivors, inconclusive, stillborn, unwitnessed or equivalent results.
Eleven mutants add synthesis refusals but each is killed by executed scenarios;
the baseline has none. Earlier36-scenario/18-mutant results above are historical.

Planted production defects also fail by name: premature idle EOF fails three quiet
scenarios, waiting until a pending decision deadline fails pending-decision-steering,
and dropping the no-wire interrupt fallback fails quiet-wireless-interrupt. All were
restored and rerun green. Independent read-only review found the startup fixture
race, progress throttling and a leading-hyphen argv edge; each was corrected.

Process-write scenarios inspect actual production admission and argv. Native
probes on this host record explicit run withholding and no effects; they do not
claim successful kernel containment. The retained Rust probe checks declared writes,
EROFS on undeclared existing directories and outside denial on a capable host.
Integer event literals are compared through exact decimal parsing into i64; the new
count assertions first failed on 2.0 versus 2 serialization. Independent review
then caught large-integer rounding; a reproduced red regression now passes after
exact decimal normalization. Unknown shapes and out-of-range expectations still fail closed.

## ADR 0096 extension — the session format domain

Protocol amendment a26 (2026-10-20) names `spec/domains/trace.yaml` the owning
specification of the `trace-ir/1` document (Atlas ADR 0096 point 7). The domain
`metaharness.trace` adds 34 generated scenarios: nineteen classify outcomes
(eleven family outcomes and one outcome per `unk` kind, seven guarded and
`auth.expired` the unguarded default that only it can reach, so all twenty wire
kinds are driven; a default is kept because without one ESS requires the guards
to partition the input and every classify guard-negate mutant is stillborn
instead of audited), five completeness shapes
(absent, complete, misplaced, miscounting, duplicated: every verdict
`stream_completeness` decides), two digest outcomes, two retained projections
of the recorded runs (each pinning its node count and its opening timestamp in
milliseconds by literal), two view-invariant scenarios carrying twelve predicates
(`counts.nodes >= 1`, `counts.lines >= 1`; `counts.nodes == counts.lines`,
`byte_stable == true`, `golden_stable == true` (a fresh render is the committed
`evals/aep/runs/<run>.trace-ir.json` byte for byte, P1 across builds),
`opening_node.index == 0`, `opening_node.source_line == 1`,
`opening_timestamp_present == true`, `census.events_total == counts.nodes`,
`census.unk == families.unk`, `families.session_start == 1`,
`families.stream_closed == 1`), and four
normalize outcomes, one per source kind. The combined 79-scenario model passes
with no skipped, unsupported or refused obligations. Earlier 45-scenario/25-mutant
results above are historical.

Every value a scenario checks is read off the document `project_document` wrote
or off the reader's emissions; the target restates no mapping. The retained
view re-reads the rendered document from disk, re-projects the committed stream
it names (`metaharness.run`) and compares bytes, so byte stability is observed
at read time rather than remembered. The target now checks a `generated` payload
field by shape and every literal field by value, admits a view predicate whose
right-hand side is another field or a boolean, and accepts the ordinary
`ess-conformance/34` suites `ess verify conform mutate` emits (whose reports
read `inconclusive` for want of declared coverage; the collector scores them on
executed scenarios, and the gate keeps the declared-coverage `/35` suite).

Hardening evidence, 2026-10-20, each defect restored and rerun green:

- Swapping the classify table's `assistant_text` payload to `assistant_thinking`
  in the specification failed ClassifyEventKind/outcome/assistant-text
  (78 passed, one failed).
- Changing the production `UNK_REASON` constant and making the production
  completeness flag copy the marker (`stream_complete = families contains
  stream_closed`) failed all eight ClassifyEventKind/outcome/unk-* scenarios and
  all three VerifyStreamCompleteness/outcome/inconsistent-* scenarios (68 passed,
  eleven failed). A shape the domain does not name (here a duplicated marker read
  as complete) is reported as an undeclared outcome, so it fails by scenario name
  instead of aborting the run.
- Review found three gaps in the first revision and each is closed in the
  specification, not the target: the `unk` outcome was unguarded and seven of its
  kinds were never driven (a production mutant reclassifying `auth.expired`
  survived); `misplaced-or-miscounting` was one variant and only the misplaced
  shape was built (a completeness check without the count comparison survived);
  and `>= 0` bounds on unsigned counters could not fail. The regression guard
  `crates/metaharness-aep/tests/adversary_trace_ir_domain.rs` reads the committed
  suite and fails while any wire kind the domain states is never driven.
- `ess verify conform mutate --class guard-negate --emit`, execution of every
  emitted suite against production, and `--collect` killed **52 of 52 mutants**:
  no survivors, inconclusive, stillborn, unwitnessed or equivalent results.
  Baseline: 79 scenarios, zero refusals. Thirty-three mutants add synthesis
  refusals but each is killed by executed scenarios. Other classes were not claimed.
  An earlier revision guarded all nineteen classify outcomes with no default;
  its audit left 19 classify mutants stillborn (ESS-COMMAND-004/005), which is
  why `auth.expired` is the unguarded default above.

Two boundaries are stated rather than claimed. `metaharness.source` is observed
as the constant `metaharness.event/1`; whether it ever carries a vendor format
name is `story:project-reads-vendor-transcripts`'s open question and is marked
`UNMAPPED:` in the domain. The b10x golden record is read as captured from
`b10x-harness 0.15.0` (re-recorded 2026-10-20 over the 0.9.1 capture this
suite was written against) and no count of its nodes is declared, so the
re-recording moved no trace scenario. Reading a vendor record through
`metaharness project` itself, changing any reader or the projection, and
publishing a `trace-ir/1` schema remain outside this suite.

## Amendment a27 extension — the declared scratch working directory

Protocol amendment a27 (2026-10-20) admits `--hermetic strict` over an embedder's
declared scratch copy (`--cwd <dir> --scratch-cwd`). The domain
`metaharness.workspace` gains `DeclareScratchWorkingDirectory` with four
generated scenarios: the declaration without a directory is refused
(`check_spec`, every kind); a declared copy under strict is admitted (H7
`declared`, H11 imposed, exit 0); a declared copy with an `AGENTS.md` one level
above it reports H7 `declared`, H11 unavailable and exit 1; a named directory
without the declaration keeps a6 (H7 unavailable, exit 1). Each scenario is a real
strict Claude run over `ScriptedRunner` in a directory the target made, drained to
its end; the H7 state and H11 verdict are read off `session.started`'s attestation,
the exit code off the production floor through `Run::audit` and `Run::exit`, and the
planned cwd off the runner's log (`ScriptedLog::launched_cwds`), never off argv.
The outcome is derived from those observations and not from the probe, so a shape
the domain does not declare fails by scenario name. The combined 83-scenario model
passes with no skipped, unsupported or refused obligations. Earlier
79-scenario/52-mutant results above are historical.

Hardening evidence, 2026-10-20, each defect restored and rerun green:

- Changing the specification's admitted payload to `exit_code: 1` (in a scratch
  copy of `spec/`, run through `METAHARNESS_ESS_SUITE`) failed
  DeclareScratchWorkingDirectory/outcome/strict-admits-the-declared-copy
  (82 passed, one failed).
- Making the production floor read H11 `ok` whenever H7 is declared — the
  measured walk ignored under the declaration — failed
  DeclareScratchWorkingDirectory/outcome/memory-above-the-copy-fails-strict
  (82 passed, one failed), reported as the undeclared shape
  `("declared", false, 0)` rather than aborting the run.
- `ess verify conform mutate --class guard-negate --emit`, execution of every
  emitted suite against production, and `--collect` killed **55 of 55 mutants**:
  no survivors, inconclusive, stillborn, unwitnessed or equivalent results.
  Baseline: 83 scenarios, zero refusals. Thirty-five mutants add synthesis
  refusals but each is killed by executed scenarios. The three new guards are
  each killed by two scenarios of the command. Other classes were not claimed.

Two boundaries are stated rather than claimed. The declaration confines nothing
(`story:confinement-of-vendor-kinds`) and the target makes the copy itself, as
Loom's executor would; neither a vendor process nor a model starts, so the
scenarios establish what the floor decides about the record, not what a vendor
binary does in the copy. The Codex and b10x paths carry the same attestation rule
and are pinned by unit tests and the Codex C1 vector, not by this domain.

## Amendment a28 extension — the verb over a vendor record

Protocol amendment a28 (2026-10-20) gives `metaharness project` a `--from
<claude-code-stream-json|codex-rollout|harness-json>` that reads a vendor record
metaharness did not produce through that adapter's production reader. The domain
`metaharness.trace` gains `ProjectVendorRecord` with four generated scenarios:
one per vendor source, each calling the verb's own library entry
(`metaharness::project_vendor_record`) twice over the committed golden record and
reading off the rendered document that `metaharness.source` is the format's name,
the opening node is the adapter's `session_start`, `hermetic.mode` is `off` with
nothing imposed, `stream_complete` is false (nobody closes a replayed stream),
the two renders agree, `transcript_digest` is the SHA-256 of the record's bytes,
and the family census equals the adapter's own committed expected stream projected
through `project_document`; and the default, reached by the one `SourceKind` that
is not a `--from`, observing that the entry refuses the event stream's name and
names it. The outcome is named from the opening node's adapter, never from the
input, so a shape the domain does not declare fails by scenario name. The two
`UNMAPPED:` notes a26 left for `story:project-reads-vendor-transcripts` are
settled in the domain. The combined 87-scenario model passes with no skipped,
unsupported or refused obligations. Earlier 83-scenario/55-mutant results above
are historical.

Hardening evidence, 2026-10-20, each defect restored and rerun green:

- Changing the specification's claude payload to `hermetic_mode: "strict"` (in a
  scratch copy of `spec/`, run through `METAHARNESS_ESS_SUITE`) failed
  ProjectVendorRecord/outcome/claude-transcript-projected (86 passed, one failed).
- Making production write `metaharness.event/1` as the source of a replayed record
  — the a28 regression, `viewer.rs` handing `project_from` the wire's name — failed
  all three ProjectVendorRecord vendor outcomes by name (84 passed, three failed).
- `ess verify conform mutate --class guard-negate --emit`, execution of every
  emitted suite against production, and `--collect` killed **58 of 58 mutants**:
  no survivors, inconclusive, stillborn, unwitnessed or equivalent results.
  Baseline: 87 scenarios, zero refusals. Thirty-seven mutants add synthesis
  refusals but each is killed by executed scenarios. The three new guards are
  each killed by two scenarios of the command. Other classes were not claimed.

Two boundaries are stated rather than claimed. The suite does not claim that a
record other than the three goldens opens with `session.started` under its reader;
a record that does not is refused by the verb (a28 point 4), and
`crates/metaharness-cli/tests/project.rs` pins every reader over every record that
is not its own, the recorded event stream included. The verb's exit code (0, or 2
on the refusal) is pinned there too, not here: the target calls the library entry
and reads the refusal, never the binary.
