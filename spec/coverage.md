# Executable contract and coverage boundary

This is a bounded retrofit of Metaharness's observable boundaries. It preserves
frame/1 and adapter ownership. It does not claim to specify every event,
credential rule, process-isolation mechanism or vendor version.

ESS 0.69.2 validates both manifests. The core model uses ess/29;
conformance.json is a declared-coverage ess-conformance/35 suite. Regenerate it:

```console
ess verify conform synthesize --path spec --scenarios spec --suite-format 5 --strict-requires --out spec/conformance.json
```

`--scenarios spec` selects the authored scenarios `spec/ess-inputs.yaml` lists;
without it ESS selects none and writes the 107 generated ones only.
`task check` validates both manifests, checks suite drift, and executes the
production target in `crates/metaharness-aep/src/drive/ess_conformance.rs`.
CI installs ESS 0.69.2 from commit
`a3bc2df3df986e8d340553145e8615a00f982331`. Use that compiler locally.

The committed count is **116 selected scenarios** (110 generated, 6 authored):
116 passed, zero failed, skipped, unsupported or synthesis refusals. Unknown suite versions, steps,
bindings, shapes and predicates fail closed. The target handles only the
constructs this suite uses; extending the specification may require extending it.

| Obligation | Production observation |
|---|---|
| Terminal and unknown evidence | Adapter-owned synthetic inputs traverse RolloutReader; normalized is_error is read back and absent cost must remain None. |
| Stream closure | A real Run over ScriptedRunner produces closing reason and event count. No vendor process or model starts. |
| Final answer, model selection and tool outcome | Adapter-owned synthetic inputs traverse the production Codex reader. Nine generated outcome scenarios read normalized terminal and tool fields; absent or contradictory evidence stays unknown. Source mappings are documented in `docs/research/2026-10-03-codex-observations.md`, with bounded native fixture observations documented separately. |
| Native termination | The real protocol reader distinguishes missing, zero, nonzero and signal evidence. Separate Rust executable fixtures drive both real spawn runners through measured exit0, exit7 and SIGKILL, unavailable wait status, and terminal failure with exit0. |
| Audited run exit (`metaharness.session.ExitAuditedRun`, amendment a48) | A real `--hermetic strict` Claude run over a declared scratch copy, audited. A measured child is `/bin/sh` under the production `SpawnRunner`, exiting 0, exiting 7 or killed by SIGKILL; an unmeasured one is `ScriptedRunner`. The report's `child exit:`, `control verdict:` and `audit verdict:` lines and `Run::exit`'s code are read back. |
| Frame integrity | The AEP driver mints a frame and Frame::parse_document reads it; modified and untagged frames are refused. |
| Tool decisions | Production Codex mapping and answer_events consult a real AEP engine; malformed, patch and unsupported calls are denied. |
| Workspace and file containment | The production name predicate and owned tool server operate on a selected directory; an outside symlink is refused. This does not prove process or cgroup isolation. |
| Context files (`metaharness.workspace.DeclareContextFiles`, amendment a35) | Each probe is a real start of all three kinds over `ScriptedRunner` in a git work tree the target made, whose `.git` is a gitfile naming a sibling git directory. Every refusal production raises (no `--cwd`, an empty path, absolute, `..`, a path that is not UTF-8, symbolic link, not a regular file, not there, not text, untracked, count cap, byte cap: twelve, one outcome each) must come from every kind before the spawn with the same reason, and the target maps the refusal type exhaustively, so a thirteenth fails to compile; the accepted probe reads `session.started.context_files` off each run and compares every digest with the tree's bytes. No vendor process or model starts. |
| Accounting | Real finite and uncapped policy functions execute. A resumed narrowing (amendment a46) goes through `spending::policy` over a launch record carrying the remembered terms. Ledger views read files written by SpendBudget and AdmissionBudget, including persisted invocation count. |
| Session format (`trace-ir/1`, domain `metaharness.trace`) | The production projection `project_document` and the three production readers run over committed inputs only: one framed event per wire kind, classified by the node the writer emits; completeness over three marker shapes built through the production `EventStream`; the digest over the framed bytes beside the vendor's reference; the two recorded `evals/aep/runs` streams retained as rendered documents whose view is read back from disk and re-projected; the three golden vendor records normalized through `ClaudeSeams`, `RolloutReader` and `B10xSeams`; the same three records through the verb's own library entry, `metaharness::project_vendor_record` (`metaharness project --from`, amendment a28), with the source name, the record digest, the unclaimed attestation and the census against the adapter's expected stream read off the rendered document, and the event stream's name refused as a `--from`; the same three records and the recorded `decomposer-clean` stream summarized to `trace-ir-summary/1` (`metaharness project --to trace-ir-summary`, amendment a42), every summary field pinned as its rendered JSON text, `null` included. No vendor process or model starts. |

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
Option<f64>. ESS 0.66.0 declines its conformance codecs and Rust synthesis; no
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
`b10x-harness 0.20.0` (re-recorded 2026-10-26 under amendment a60, over a 0.19.0
capture of the same day under amendment a59, which had replaced a 0.18.0
capture of 2026-10-22 under amendment a50, which had replaced a 0.15.1 capture
of 2026-10-20, which had replaced a 0.15.0 capture, which had replaced the 0.9.1
capture this suite was written against) and no count of its nodes is declared,
so no re-recording added or removed a trace scenario; the 0.15.1 one added the
`started` line's `cwd`, which no scenario reads, the 0.18.0 and 0.19.0 ones
each differ from the one before only in two `captured_at` stamps and one context
digest, and the 0.20.0 one only in the two stamps, so the only expectation each
moved is the `transcript_digest` of
`SummarizeRecord/outcome/harness-record-summarized` (`spec/domains/trace.yaml`;
`spec/conformance.json` regenerated on ESS 0.69.2, 116 scenarios). The
secret-named key route a50 adds (`--api-key-secret`) is
covered by Rust tests and by no scenario here; it is one of the credential
rules this suite does not claim. Reading a vendor record through
`metaharness project` itself, changing any reader or the projection, and
publishing a `trace-ir/1` schema remain outside this suite.

## Amendment a27 extension — the declared scratch working directory

Protocol amendment a27 (2026-10-20) admits `--hermetic strict` over an embedder's
declared scratch copy (`--cwd <dir> --scratch-cwd`). The domain
`metaharness.workspace` gains `DeclareScratchWorkingDirectory` with four
generated scenarios: the declaration without a directory is refused
(`check_spec`, every kind); a declared copy under strict is admitted (H7
`declared`, H11 imposed, exit 0); a declared copy with an `AGENTS.md` one level
above it reported H7 `declared`, H11 unavailable and exit 1 until amendment a36,
which renamed it (see below); a named directory
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
  `("declared", false, 0)` rather than aborting the run. That scenario is now
  `memory-above-the-copy-is-reported` (a36), and this defect is no longer one.
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

## ESS 0.66.0 move — two guards and three authored scenarios

Since 0.57.0, ESS witnesses every child of an `all`/`any` at any depth and
refuses a child it finds no input for (`ESS-SYNTH-022`) where it used to skip it
silently. On the 0.56.0 specification, 0.66.0
refused three children: `CloseUnsteeredStream/failed-or-unknown-terminal` (the
`any: [is_error == false, subtype == "success"]` child failing alone) and
`CheckFiniteTerms/invalid-finite-terms` twice (`cap_micro_usd > 0` failing alone,
and `assumed_micro_usd_per_run <= cap_micro_usd` failing alone with both positive).
An authored scenario does not clear the refusal: with the refusals standing and
authored scenarios selected, 0.66.0 still wrote `refused: 3` and exited 1.

Two guards change, each because the specification was wrong:

- `positive-terminal-record` is `terminal_present == true`, `is_error == false`
  and no budget word. ESS evaluates guards in three-valued logic, and an absent
  `Optional` makes a comparison unknown, so `not: is_error == true` was unknown
  whenever `is_error` was absent and the `subtype == "success"` disjunct could
  never select the branch; the child ESS refused cannot fail alone under that
  logic. ESS's reference model (`ess verify conform run --target interpreted`)
  reported "the guard ... is Unknown over this input" for terminal records
  without `is_error` and without a budget word (probed with subtype `success` and
  with another subtype), where production (`Run::reason_from_record`) decides
  them. The new guard has the same truth table under ESS's logic as the old one.
- `valid-finite-terms` drops `terms.cap_micro_usd > 0`: a positive charge that
  does not exceed the cap implies it, so no input lets it decide alone. The
  accepted inputs are unchanged.

A guard over an absent `Optional` stays unknown in 0.66.0, and a presence test
beside a comparison over the same field (`all: [defined(x), x == …]`) is refused in
turn: ESS cannot show that `defined(x)` fails alone only where the comparison is
unknown. Under 0.66.0 a terminal record without `is_error` was therefore outside what
`positive-terminal-record` decided. Production's handling of it
(`error` for any other subtype, `completed` for the legacy subtype `success`,
`budget` for a budget word beside it) was pinned beside `Run::reason_from_record` by
`a_terminal_record_without_is_error_closes_error_for_any_other_subtype` and
`a_terminal_record_without_is_error_closes_completed_for_the_legacy_success_subtype`
in `crates/metaharness/src/run.rs` only. Both tests stay; the suite holds the same
records since the `coalesce` guards (section below).

Three authored scenarios (`spec/scenarios/`, listed in `spec/ess-inputs.yaml`) send
inputs the guards decide and no generated scenario sends. ESS's reference model
passes all three.

| Authored scenario | Input | Expected | Production defect it alone fails on |
|---|---|---|---|
| `close-unsteered-budget-word-outranks-positive-verdict` | `is_error: false`, `terminal_reason: budget-exhausted` | `observed-budget-stop` | `is_error == Some(false)` read before the budget word |
| `close-unsteered-explicit-error-outranks-success` | `is_error: true`, subtype `success` | `failed-or-unknown-terminal` | the success reading tried before `is_error == Some(true)` |
| `finite-charge-above-positive-cap` | cap 5000000, charge 6000000 | `invalid-finite-terms` | the charge-above-cap check removed from `spend_terms_with_live` |

Every generated budget input sends `is_error: true`, so only the first scenario
holds budget precedence over a positive verdict. The second replaces a generated
input the old guard produced and the new one does not. The third holds the
comparison that the generated input (cap 0, charge 1) leaves to the zero-cap check.
Each defect was planted in production, run and restored, and no generated scenario
failed on any of them: the first against this suite (89 of 90 passed, the named
one failed), the second and third together against the earlier 91-scenario
revision (89 passed, two failed). Planting the a18 regression (an absent verdict
closing `completed`) fails the first `run.rs` test, and removing the legacy reading
fails the second; on this revision neither failed a scenario. Apart from the two outcome scenarios
the guard changes touch, every generated scenario is the one 0.56.0 wrote.

Mutation audit, 2026-10-20, against ESS's reference model rather than production:
`ess verify conform mutate --path spec --scenarios spec --target interpreted
--class guard-boundary --class guard-negate --class guard-connective` reports 93
mutants, 91 killed, one survived, one inconclusive, none stillborn, unwitnessed or
equivalent (baseline: 90 scenarios, zero refusals, ten not scored because the
reference model cannot observe their views). The survivor is the or-to-and mutant
of `positive-terminal-record`'s `none:` clause. On a record carrying a budget word
it changes nothing `observed-budget-stop`, declared first, does not already take;
under ESS's three-valued logic it also turns the branch from unknown to true on the
success record with absent `stop_reason` and `terminal_reason`, which is hidden only
because `observed-budget-stop` is unknown there too
(`story:session-close-decides-absent-fields`). So it stays equivalent only while the
declaration order and that gap stay as they are; the first authored scenario does
not kill it, and that story must revisit the clause. Dropping the clause would remove the site: on a scratch copy without it,
89 mutants, 88 killed, none survived. The inconclusive mutant negates
`ProjectRecordedRun/decomposer-clean-recorded`, whose scenarios the reference
model cannot run.

## Amendment a36 change — a memory file above the copy is reported

Protocol amendment a36 (2026-10-21) rests Claude Code's H11 on the launch's own
control (`--setting-sources ""` and a fresh scratch config home), held by the floor
to the memory canary recorded for the record's version, and makes the ancestor walk
report what it finds. `DeclareScratchWorkingDirectory/outcome/memory-above-the-copy-fails-strict`
becomes `memory-above-the-copy-is-reported`: an `AGENTS.md` one level above the
declared copy now reads H7 `declared`, H11 imposed and exit 0, and the event carries
`memory_above_reported: true`, read off whether the imposed H11 row names the planted
file by its resolved path. The other three scenarios carry `memory_above_reported:
false`. The scenario is Claude-specific: the target drives a Claude run, and the
Codex and b10x walks are unchanged. The count stays at 103 (100 generated, 3
authored), zero refused, regenerated with ESS 0.67.0.

Hardening evidence, 2026-10-21, restored and rerun green: making the Claude walk
return nothing failed `memory-above-the-copy-is-reported` alone (102 passed, one
failed), observed as `strict-admits-the-declared-copy` with
`memory_above_reported: false`. The canary guard itself (a version with no canary,
or a canary that reached a request, reads H11 `gap`) is pinned by
`crates/metaharness/tests/audit.rs`, not by this suite.

## `coalesce` guards — the session close decides absent fields

ESS 0.67.0 lets a command guard write `coalesce(x, d) == v` from format `ess/29`
and reads `d` where `x` is absent (ESS changelog 0.67.0, beyond10x/ess#620; below
`ess/29` the validator refuses the spelling as `unsupported_format_version`, naming
`ess/29`). `spec/system.yaml` moves from `ess/15` to `ess/29`; every domain
validates unchanged, and no scenario outside `CloseUnsteeredStream` changes.

`CloseUnsteeredStream`'s guards now say what an absent field compares as:

- A budget word is `coalesce(<field>, "") == "budget-exhausted"` over `subtype`,
  `stop_reason` and `terminal_reason`: an absent field carries no word.
- `positive-terminal-record` is `terminal_present == true`,
  `coalesce(is_error, false) == false` (no explicit error) and any of
  `coalesce(is_error, true) == false` (an explicit `false`) and
  `coalesce(subtype, "") == "success"` (the legacy success). An absent `is_error`
  is neither verdict.
- Its `none:` budget clause is gone. ESS answers accepting branches in declaration
  order (its own `ESS-AUTHOR-041` refusal states the rule), and
  `observed-budget-stop`, declared first, is now decided on every input, so the
  clause decided nothing. It was the 2026-10-20 audit's survivor; kept, its
  or-to-and mutant is unwitnessed (`ESS-MUTATE-004`).

The grid is every terminal-record shape: `terminal_present` × `is_error` (true,
false, absent) × `subtype` (`success`, the budget word, another, absent) ×
`stop_reason` and `terminal_reason` (the budget word, another, absent), 216 shapes.
Production decides all 216, so none is left out. ESS's reference model passed 184
and left 32 unsupported on the previous guards (0.67.0); it passes 216 now.
`the_reference_model_closes_every_terminal_record_shape_as_production_does`
(`crates/metaharness-aep/src/drive/ess_conformance.rs`) holds it in `task check`:
each shape's expectation is production's own close over `ScriptedRunner`, and the
reference model must pass every one. The success record the repository's vectors
write is decided too
(`the_specification_decides_the_success_record_the_repository_writes`).

The suite keeps 103 scenarios (100 generated, 3 authored), zero refused. The three
`CloseUnsteeredStream` outcome scenarios change, and synthesis now sends optional
inputs absent: an absent verdict with subtype `success` and with another subtype
are generated scenarios, run against production. The generated suite also sends
`is_error: false` beside a budget word and `is_error: true` beside subtype
`success`, the inputs the first two authored scenarios were selected for; they stay.
Planted, run and restored, 2026-10-21: the a18 regression (an absent verdict
closing `completed`) fails `failed-or-unknown-terminal` (102 of 103) and 8 grid
shapes; removing the legacy reading fails `positive-terminal-record` (102 of 103)
and 4 grid shapes.

Mutation audit, 2026-10-21, the command above: 104 mutants, 103 killed, none
survived, one inconclusive (`ProjectRecordedRun/decomposer-clean-recorded`, as
before), none unwitnessed; all 14 `CloseUnsteeredStream` mutants are killed.

## Amendment a42 extension — `trace-ir-summary/1`

Protocol amendment a42 (2026-10-21) adds `trace-ir-summary/1`, the one projection
of a `trace-ir/1` document that takt's harness-step rows and Loom's jsonl read,
written by `metaharness project --to trace-ir-summary` with or without `--from`.
The domain `metaharness.trace` gains the types `SessionSummary`,
`SummarizedModel` and `SummarizedToolCall`, the enum `TokensBasis` and the
command `SummarizeRecord`, with four generated
scenarios. Three are one per golden vendor record, read by
`metaharness::project_vendor_record`. The default is the recorded
`decomposer-clean` stream, read by `metaharness::project_file`. Each summarizes
the document twice through `metaharness_protocol::summarize`, parses the bytes
`TraceIrSummary::render` wrote, and observes every rendered field as its compact
JSON text, nested keys sorted; no value is read off the typed struct. The
`models` and `tokens_basis` literals pin each record's per-model figures and the
nodes its token counts came from (`"sum-of-requests"` for the b10x loop record,
`"session"` for the other three). ESS 0.67.0 admits no `null` payload
literal and no `Binary64` in this projection, and the text tells `null` from `0`
byte for byte. `cost_usd`'s type is `metaharness.monetary.SummaryCost` in
`spec/monetary`, for the top-level cost and each `models` entry's. The outcome
is named from the summary's own `source`. The suite
moves from 103 to 107 scenarios (104 generated, 3 authored), zero refused,
regenerated with ESS 0.67.0.

Hardening evidence, 2026-10-21, restored and rerun green: making `summarize`
write `0.0` for a cost the document does not state failed
`codex-rollout-summarized` and `harness-record-summarized` (105 passed, two
failed), the two records that state no cost. The Claude record states every
summary field, so its outcome pins no `null`. The Codex and b10x outcomes pin the
nulls. A call no result answers and a document with no terminal node occur in no
committed record. Both are pinned by the unit tests in
`crates/metaharness-protocol/src/trace_summary.rs`, not by this suite.

Mutation probe after the adversary pass, 2026-10-21: 17 hand mutants of
`trace_summary.rs`, each run against the protocol tests, the CLI tests and this
suite, the file restored and compared after each. All 17 fail at least one
layer. This suite fails 10 of them: turns off by one, the two cache keys
swapped, the timestamps swapped, the first tool call dropped, an absent token
count written as `0`, an absent cost written as `0`, an unstated request key
summed as `0`, a model named twice, a model's figures dropped, and
`tokens_basis` always `"session"`. Seven pass this suite and fail only the
protocol tests: the last timestamp taken as the largest, every call marked
answered, the last result's verdict kept, the first session close taken as
terminal, a `session_start` after node 0 taken as opening, a usage fold without
`num_turns` taken as terminal, and a non-finite cost written as `null`. No
committed record tells the first six apart from the production code, and no
committed record carries a non-finite cost.

## Amendment a46 extension — a resumed cap narrowed below the assumed charge

Protocol amendment a46 (2026-10-22) refuses a resumed `--budget-usd` below the
run's remembered assumed charge, while the host prepares the resume and before
AEP takes the run lock or rewrites the launch record. The domain
`metaharness.spending` gains the command `NarrowResumedCap` and the errors
`CapBelowAssumedCharge` and `NarrowingRefused`. The command takes three `Integer`
inputs in millionths of a dollar: the remembered cap, the remembered assumed
charge and the narrowed cap. Each carries the observed value as its `example:`
($1, $0.10, $0.05). The guards compare inputs with each other, which gives ESS's
witness search no literal to start from, so without the examples it finds no
witness for `cap-below-assumed-charge` (`ESS-SYNTH-003`). The error outcome is
declared before `cap-narrowed`, because ESS 0.67.0 refuses a declaration order
that disagrees with its precedence order (`ESS-COMMAND-004`).

There are three generated scenarios, one per outcome. `cap-below-assumed-charge`
sends $1, $0.10 and $0.05, and then a narrowed cap of one millionth.
`cap-narrowed` sends a $1 cap and a charge of one millionth, narrowed to $0.05.
`narrowing-refused` sends a zero charge, a zero narrowed cap, and a zero
remembered cap under a $0.10 charge and under a one-millionth charge. The authored
scenario `narrowed-cap-below-assumed-charge` has four steps:

1. the observed terms, refused with `CapBelowAssumedCharge`;
2. the cap narrowed to exactly the charge, admitted;
3. the cap left at the remembered $1, admitted;
4. a $0.10 cap whose charge is $0.10, narrowed to $0.05, refused.

The production arm writes the remembered terms into an `aep_cli::drive::Launch`
record as a resume reads them, and calls `spending::policy` with the narrowed cap
as `--budget-usd` and live spending admitted. An admission counts as `cap-narrowed`
only when the policy it returns is the finite policy with the narrowed cap and
the remembered charge. A refusal counts as `cap-below-assumed-charge` only when
it is the typed `CapBelowAssumedCharge` and its fields equal the input. Any other
refusal counts as `narrowing-refused`. The suite moves from 107 to 111 scenarios
(107 generated, 4 authored), zero refused, regenerated with ESS 0.67.0.

Hardening evidence, 2026-10-22: each defect was planted, run and restored, and the
suite was rerun green. Both were measured when the authored scenario had only its
first two steps. Removing the floor failed 2 of 111: the generated
`cap-below-assumed-charge` and the authored scenario, each observed as
`cap-narrowed`. Writing the floor as `<=` failed 1 of 111: the authored scenario's
step at exactly the charge, observed as `cap-below-assumed-charge`.

Mutation audit, 2026-10-22, the command in the previous section: 117 mutants, 116
killed, none survived, one inconclusive (`ProjectRecordedRun/decomposer-clean-recorded`,
as before), none unwitnessed. All 10 `NarrowResumedCap` mutants are killed. Steps
3 and 4 were added because, with the first two steps only, both mutants that turn
a `<=` against the remembered cap into `<` survived.

UNMAPPED (amendment a46, D7): a cap at or above the charge but below the amount
the run's ledger has already reserved. `prepare` has no run directory to read the
ledger from, so that cap persists. The next reservation then refuses it as an
exhausted cap.

## Amendment a48 — the audited run's exit

Protocol amendment a48 (2026-10-22) makes the audit report name three facts on
lines of their own: `child exit:` (the harness process's own exit, `unknown` when
nothing measured it), `control verdict:` (the gating rows folded `gap` over `unk`
over `ok`) and `audit verdict:` (the exit code). No exit code changes. The domain
`metaharness.session` gains the enums `ChildExit`, `OpeningRecord`,
`ControlVerdict` and `AuditVerdict`, the command `ExitAuditedRun` and the event
`AuditedRunExited`, and the process exit status leaves the `UNMAPPED:` note
beside `CloseUnsteeredStream`.

`ExitAuditedRun` has three generated scenarios, one per outcome, chosen by the
opening record alone: `complete` reads `every-control-held` (exit 0),
`silent-on-plugins-and-mcp` (H1a and H5 `unk`) reads `nobody-found-out` (exit 3)
and `off-the-pin` (H9 `gap`) reads `a-gating-row-failed` (exit 1). The event
echoes the child (`child_exit: input.child`), and no guard reads it. Synthesis
fixes the child at the enum's first variant, `exited-zero`, so
`every_declared_child_exit_drives_the_audited_run_scenarios` runs each generated
scenario again with every child the event's shape declares, 12 runs through the
same arm. The authored scenario
`audited-run-child-exit-zero-with-unknown-controls-exits-three` is the case the
amendment exists for: a child that exited 0 beside unknown gating rows still
exits 3. The suite moves from 107 to 111 scenarios (107 generated, 4 authored),
zero refused, regenerated with ESS 0.67.0. Every other scenario gains the new
event in its `source` list and one `expect_no_event` step for it, and is
otherwise unchanged, compared as JSON values.

An auditor's own exit, which moves the audit verdict and never the control
verdict, and a run with no terminal record, which exits 3 beside a control
verdict of `ok`, are pinned by `crates/metaharness/tests/audit.rs`, not by this
suite.
