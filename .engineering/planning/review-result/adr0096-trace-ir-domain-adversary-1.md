---
format: aep.planning-md/3
id: review-result:adr0096-trace-ir-domain-adversary-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:trace-ir-domain: red, 2 warnings, 3 notes'
owner: metaharness
relations:
- reviews: story:trace-ir-domain
revision: 1
---
unit: story:trace-ir-domain — working tree wt-4a933645b202, branch impl/trace-ir-domain, uncommitted over d7fad095ab119ecd95b33c2d98b6a39b276f9ac3
verdict: red (strongest finding CONFIRMED, severity warning; nothing NEEDS-CHANGE; one red case in the tree, two planted production mutants survive the committed 72-scenario suite)
cases: executed 80→85 cargo tests (red 1) and 72→72 committed scenarios (red 0); two external probe suites executed 72 each (probe 1 red 6 by construction, probe 2 red 0)
origin: introduced 5 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory, metaharness-wave-adr0096/trace-ir-domain/adversary-scratch under the operator's cache directory (22 paths incl. a 7.2M source copy with a 1.2G target/)
needs-coordinator: none

## 1. What I touched

`git --no-pager diff d7fad09 --stat` (tracked files; all nine are the implementor's, none mine):

```
 CHANGELOG.md | 22 +   README.md | 2 +-   crates/metaharness-aep/src/drive/ess_conformance.rs | 705 +-
 docs/design/metaharness-protocol-v0.1.md | 32 +   docs/design/runs-side-by-side-v0.1.md | 6 +
 spec/conformance.json | 7582 +-   spec/coverage.md | 50 +-   spec/ess-inputs.yaml | 1 +   spec/system.yaml | 1 +
 9 files changed, 8346 insertions(+), 55 deletions(-)
```

My additions are untracked (`git status --short --untracked-files=all | grep '^??'`), all test paths:

| Path | Kind |
|---|---|
| crates/metaharness-aep/tests/adversary_trace_ir_domain.rs | Rust integration test, 5 cases |
| crates/metaharness-aep/tests/adversary-trace-ir-domain/matcher-probe-suite.json | suite derived from spec/conformance.json, 6 literals mutated |
| crates/metaharness-aep/tests/adversary-trace-ir-domain/matcher-omitted-field-probe-suite.json | suite derived from spec/conformance.json, 1 payload field removed |

`git diff d7fad09 --stat -- crates/metaharness-protocol crates/metaharness-claude crates/metaharness-codex crates/metaharness-b10x evals` is empty: no implementation, reader or fixture changed, by me or anyone. Session lease `adversary-trace-ir-domain-20261020` acquired at start (`worktree hook session-start`, exit 0) and released at end (`session-end`, exit 0).

## 2. Cases added (written before anything ran)

File: crates/metaharness-aep/tests/adversary_trace_ir_domain.rs in the unit tree. Expected values are literals from spec/domains/trace.yaml, not from code.

| Case | Asserts | Now (tree) | Against mutants (scratch copy) |
|---|---|---|---|
| `every_wire_kind_lands_in_the_family_the_domain_states` | all 20 wire kinds, framed through the production `EventStream` and `project_document`, land in the family trace.yaml:149-197 states; the 8 `unk` nodes carry `event_kind`, `reason == "no trace-ir/1 family"`, and are counted in `unk_kinds` | green | red: `auth.expired: the domain says unk, production wrote opaque` |
| `a_marker_that_is_last_but_miscounts_is_not_complete` | a production-written marker, last, with its count overwritten to 7 over 1 node: `stream_complete == false`, `closed == None` | green | red: `a marker counting 7 over 1 preceding node was read as complete` |
| `a_stream_with_two_markers_is_not_complete` | two markers → not complete (boundary the domain does not name) | green | green |
| `the_recorded_runs_digest_names_the_committed_file_bytes` | both recorded runs: `transcript_digest == SHA-256(file bytes)`; prints whether re-serialized lines equal the file (they do, both runs) | green | green |
| `the_suite_classifies_every_wire_kind_the_domain_declares` | every `EVENT_NAMES` kind is an input of some `ClassifyEventKind` scenario in spec/conformance.json | **red** | red |

Run of the case file alone, first execution (`PATH=…ess-0.56.0/path:$PATH RUSTC_WRAPPER=sccache CARGO_INCREMENTAL=0 cargo-watched test -p metaharness-aep --test adversary_trace_ir_domain -- --nocapture`, exit 101), verbatim:

```
running 5 tests
test a_marker_that_is_last_but_miscounts_is_not_complete ... ok
test a_stream_with_two_markers_is_not_complete ... ok
test every_wire_kind_lands_in_the_family_the_domain_states ... ok
decomposer-clean: re-serialized lines equal the file bytes
plan-reviewer-clean: re-serialized lines equal the file bytes
test the_recorded_runs_digest_names_the_committed_file_bytes ... ok
thread 'the_suite_classifies_every_wire_kind_the_domain_declares' (1679282) panicked at crates/metaharness-aep/tests/adversary_trace_ir_domain.rs:430:5:
spec/conformance.json classifies 13 of 20 wire kinds; stated in spec/domains/trace.yaml and never driven: step.left, turn.started, turn.ended, tool.decided, command.result, warning, auth.expired
test the_suite_classifies_every_wire_kind_the_domain_declares ... FAILED
test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

Scenario-lane probes (same test, `METAHARNESS_ESS_SUITE=<probe> METAHARNESS_ESS_REPORT=<scratch>/probeN-report.json`):

| Probe | Mutation | Result | Exit |
|---|---|---|---|
| 1 | `DeclareProcessWrites/process-explicit-subtrees` count 2→3; `CloseUnsteeredStream/observed-budget-stop` reason budget→completed; `ReadToolOutcome/command-success` numeric_exit true→false; `ProjectRecordedRun/decomposer-clean-recorded` nodes 16→15; `NormalizeRecord/claude-transcript-normalized` adapter claude→codex; `ClassifyEventKind/unk` reason → "no family" (every enum mutation is a declared variant, so only the by-value check can catch it) | `total 72, passed 66, failed 6`, all six named in `failures`; official report `conformance_status: failed` | 101 |
| 2 | `DeclareProcessWrites/process-explicit-subtrees`: `count` removed from payload, kept in shape | `total 72, passed 72, failed 0`; official `passed` | 0 |

## 3. Suite run (after the cases existed)

`cd <wt> && PATH=<ess-0.56.0 path>:$PATH RUSTC_WRAPPER=sccache CARGO_INCREMENTAL=0 cargo-watched test -p metaharness-aep` — exit **101**:

```
     Running unittests src/lib.rs (target/debug/deps/metaharness_aep-1516d2f537f7c6cb)
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
     Running tests/adversary_trace_ir_domain.rs (target/debug/deps/adversary_trace_ir_domain-82467b37cf66a60b)
test the_suite_classifies_every_wire_kind_the_domain_declares ... FAILED
thread 'the_suite_classifies_every_wire_kind_the_domain_declares' (1749657) panicked at crates/metaharness-aep/tests/adversary_trace_ir_domain.rs:430:5:
test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
cargo-watched: end, 46G free, exit 101, stopped_by_watch=0
```

target/metaharness-ess-report.json from that run: `{"total":72,"passed":72,"failed":0,"skipped":0}`; target/ess-conformance-report.json: `conformance_status: passed`, `unsupported 0`.

Mutant demonstration (scratch copy of the tree, worktree untouched; script adversary-scratch/make-mutant-copy.sh; Mut-1: projection.rs `Event::Opaque { .. } | Event::AuthExpired { .. } => Some(IrFamily::Opaque)`; Mut-2: event.rs:1016 `if stated != position - 1` → `if stated == u64::MAX`):

| Run in the copy | Result | Exit |
|---|---|---|
| `cargo-watched test -p metaharness-aep ess_generated_scenarios_drive_production_targets` | `total 72, passed 72, failed 0` — both mutants survive the committed suite | 0 |
| `cargo-watched test -p metaharness-aep --test adversary_trace_ir_domain` | 3 failed: `every_wire_kind…` (auth.expired → opaque), `a_marker_that_is_last_but_miscounts…`, `the_suite_classifies…` | 101 |
| `cargo-watched test -p metaharness-protocol --no-fail-fast` | Mut-2 caught by trace_ir.rs:573; Mut-1 caught by tests/wire.rs:629 and :638 | 101 |

So the full workspace gate (`task check`) still catches both mutants through the protocol crate's own tests; the conformance suite that amendment a26 names the owning specification's executable, and the package-scoped gate this unit's brief runs, catch neither.

## 4. Findings

All findings cover the working tree named in the header. Origin is `introduced` throughout: trace.yaml and the five `execute` arms did not exist at d7fad09.

| # | file:line | What is wrong | What was measured | What reaches it | Verdict | Origin | Severity |
|---|---|---|---|---|---|---|---|
| 1 | spec/domains/trace.yaml:194-197 | the `unk` outcome has no `when`, so ESS generates one scenario with one input (`step-entered`); the seven other kinds the summary at :197 lists are stated and never observed | red case `the_suite_classifies_every_wire_kind_the_domain_declares` (13 of 20 kinds driven); Mut-1 passes 72/72 scenarios, exit 0; guard case `every_wire_kind_lands…` catches it | `task check` → `ess_generated_scenarios_drive_production_targets` over spec/conformance.json; `cargo test -p metaharness-aep` alone passes the mutant; the workspace gate catches it via metaharness-protocol tests/wire.rs:629 | CONFIRMED | introduced | warning |
| 2 | crates/metaharness-aep/src/drive/ess_conformance.rs:871-875 (and trace.yaml:128, :212) | `misplaced-or-miscounting` is one variant and the arm builds only the misplaced shape (a line after a correctly-counting marker); the miscounting shape the domain's comment at :125-127 names is never observed | Mut-2 (count comparison removed) passes 72/72, exit 0; guard case `a_marker_that_is_last_but_miscounts_is_not_complete` catches it | same path as #1; workspace gate catches it via trace_ir.rs:573 | CONFIRMED | introduced | warning |
| 3 | spec/domains/trace.yaml:51, :80 | `index >= 0`, `events_total >= 0`, `unk >= 0` cannot fail for unsigned production fields; three of the eight generated view predicates observe only that the field is present (`view_matches` errs on null) | read from the suite's `expect_view` predicates; no mutant can move them | the four `…/invariant/at/metaharness.trace.RetainedProjection/…` scenarios in the gate | CONFIRMED | introduced | note |
| 4 | spec/coverage.md:149-151 (also CHANGELOG.md:13, docs/design/metaharness-protocol-v0.1.md:2142) | "four view invariants (`counts.nodes == counts.lines`, `byte_stable == true`, the census and opening-node bounds)" and a26's "one scenario per declared outcome and view invariant": the suite has four invariant *scenarios* carrying eight predicates (RetainedDocument 2, Census 2, Node 2, NodeCount 2); the two named predicates are one scenario, and NodeCount's `nodes >= 1, lines >= 1` is named nowhere | counted from spec/conformance.json `expect_view` steps | readers of the three documents | CONFIRMED | introduced | note |
| 5 | crates/metaharness-aep/src/drive/ess_conformance.rs:922 | `over_stream_bytes` compares the digest with bytes the target serialized itself (`stream_bytes(&lines)`), so NameDigest cannot distinguish a digest over the bytes as read from one over re-serialized lines, the regression trace_ir.rs:223-225 warns about; the recorded runs re-serialize byte-identically (measured), so no committed input distinguishes either | case `the_recorded_runs_digest_names_the_committed_file_bytes` prints "re-serialized lines equal the file bytes" for both runs | nothing found: every committed stream is written by the same serializer | INFEASIBLE | introduced | note |

Fixes named, not applied: #1 — eight guarded `unk` outcomes (one `when: kind == <kind>` each, same payload) or authored scenarios under `scenarios:` in spec/ess-inputs.yaml, then regenerate; the `classify` arm's outcome name for unk (ess_conformance.rs:837, `event("unk", …)`) would follow the new names. #2 — split the variant into `misplaced` and `miscounting`, each its own `inconsistent-*` outcome, and build the second shape by overwriting the production marker's count as my case does. #3 — `opening_node.index == 0`, `opening_node.source_line == 1`, `census.events_total == counts.nodes`, `census.unk == families.unk` (field-to-field `==` is what the new `view_matches` admits). #4 — doc lines. #5 — residue only.

## 5. Attacked and could not break

| Surface | Result |
|---|---|
| (a) an arm deciding from the input | none: `classify` names the outcome from the family read off the node; `completeness` from `(marker_present, stream_complete)`; `digest` from the document's `vendor_transcript`; `project_recorded_run` from `metaharness.run`; `normalize` detects `already_wire` from the file's first line and the adapter from the opening node; probe 1 and the mutants show the arms report what production did |
| (b) matcher weakening of the 45 | none: audit of all 72 scenarios — every pre-existing payload's key set equals its shape's key set and no payload value is null, so the new `event_matches` is equivalent to the base `if actual != &expected` (d7fad09 ess_conformance.rs:700) on them; probe 1 catches six mutated literals by name (exit 101). The one behavioural delta is probe 2: a payload that omits a declared field is now accepted by shape (needed for `record_id: {generated: true}`); no committed payload does that |
| (c) byte-identical regeneration | `ess --version` → 0.56.0; `ess verify conform synthesize --path spec --suite-format 5 --strict-requires --out <scratch>/regen-conformance.json` exit 0 ("72 selected scenario(s), 0 refusal"); `cmp` exit 0; both SHA-256 `4292e197…249e6d` |
| (d) domain vs code | adapter ids `claude`/`codex`/`b10x` match the three `ADAPTER_ID` constants; `metaharness.source` is `EVENT_FORMAT = "metaharness.event/1"`; `usage` folds into `run_outcome` and the suite does drive `usage` (second execute step of `run-outcome`); both recorded streams open with `session.started` adapter `claude` and end with a counting `stream.closed` (16, 14 lines); the three golden `.expected.jsonl` open with `session.started`; `version.txt` says `b10x-harness 0.9.1` as a26 states |
| (e) a26 and the § 1 pointer | consistent with trace.yaml on families, unk reason, stream_closed, three marker shapes, digest beside vendor reference, retained record, opening `session_start`, the `UNMAPPED:` source boundary; only the counting inexactness in #4 |
| boundary: two markers | production refuses it (my case green) |

## 6. Paths written outside the worktree

All under metaharness-wave-adr0096/trace-ir-domain/adversary-scratch/ in the operator's cache directory:
`regen.log`, `regen-conformance.json`, `make-probe-suites.sh`, `make-mutant-copy.sh`, `run1-adversary-cases-alone.log`, `run2-matcher-probe1.log`, `run3-matcher-probe2.log`, `run4-package-suite.log`, `run5-mutant-suite.log`, `run6-mutant-adversary-cases.log`, `run7-mutant-protocol-unit-tests.log`, `run8-mutant-protocol-all-tests.log`, `run5-6-exits.txt`, `probe1-report.json`, `probe1-report.results.json`, `probe2-report.json`, `probe2-report.results.json`, `mutant-tree/` (7.2M copy of the worktree without target/.git, two mutated files, plus `mutant-tree/target/` 1.2G — the only heavy item; delete when the findings are recorded). Inside the worktree's own ignored build dir my runs overwrote `target/metaharness-ess-report.json`, `target/ess-conformance-report.json` and `target/ess-conformance-report.results.json` (the package run wrote them last; they show 72/72 passed). Disk after: 59G free on `/` (was 48G at start; something else freed space meanwhile).

Commands and exit codes, in order: `worktree hook session-start` 0; `ess --version` 0; `ess verify conform synthesize …` 0; `cmp` 0; R1 case file alone 101; R2 probe 1 101; R3 probe 2 0; R4 `cargo test -p metaharness-aep` 101; R5 mutant suite 0; R6 mutant case file 101; R7 mutant `-p metaharness-protocol` 101; R8 same `--no-fail-fast` 101; `worktree hook session-end` 0. No commit, staging, store write, paid run or vendor launch; no GitHub/GitLab API call.

```findings
[
  {"file": "spec/domains/trace.yaml", "line": 194, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "the unguarded unk outcome makes ESS drive one of the eight kinds the summary at :197 names (step-entered), so the suite never observes step.left, turn.started, turn.ended, tool.decided, command.result, warning or auth.expired, and a production mutant reclassifying auth.expired as opaque passes all 72 scenarios"},
  {"file": "crates/metaharness-aep/src/drive/ess_conformance.rs", "line": 871, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "the misplaced-or-miscounting arm builds only the misplaced shape, so a production completeness check with the marker-count comparison removed passes all 72 scenarios; the single variant at trace.yaml:128 cannot ask for both shapes"},
  {"file": "spec/domains/trace.yaml", "line": 51, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "index >= 0 (:51) and events_total >= 0, unk >= 0 (:80) cannot fail for unsigned production fields, so three of the eight generated view predicates observe only presence; opening_node.index == 0, opening_node.source_line == 1, census.events_total == counts.nodes and census.unk == families.unk would observe values"},
  {"file": "spec/coverage.md", "line": 149, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "coverage.md:149-151, CHANGELOG.md:13 and a26 (metaharness-protocol-v0.1.md:2142) count four view invariants, one scenario per invariant; the suite has four invariant scenarios carrying eight predicates (RetainedDocument 2, Census 2, Node 2, NodeCount 2) and NodeCount's nodes >= 1, lines >= 1 is named nowhere"},
  {"file": "crates/metaharness-aep/src/drive/ess_conformance.rs", "line": 922, "category": "judgement", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "over_stream_bytes compares the digest with bytes the target serialized itself from the lines, so NameDigest cannot distinguish a digest over the bytes as read from one over re-serialized lines (the regression trace_ir.rs:223-225 names); both recorded runs re-serialize byte-identically and no committed input reaches a file that differs"}
]
```
