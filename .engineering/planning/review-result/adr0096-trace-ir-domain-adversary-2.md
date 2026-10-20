---
format: aep.planning-md/3
id: review-result:adr0096-trace-ir-domain-adversary-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:trace-ir-domain: green for the tree, 1 warning, 2 notes'
owner: metaharness
relations:
- reviews: story:trace-ir-domain
revision: 1
---
unit: story:trace-ir-domain, pass 2 — working tree wt-4a933645b202, branch impl/trace-ir-domain, uncommitted over d7fad095ab119ecd95b33c2d98b6a39b276f9ac3 (79-scenario suite, 34 trace)
verdict: green for the tree (no case red against it); strongest finding CONFIRMED/warning — two production mutants survive the 79-scenario suite and are caught only outside it; nothing NEEDS-CHANGE
cases: executed 85→87 cargo tests (red 0) and 79→79 committed scenarios (red 0); one external probe suite executed 79 (red 6 by construction); in the mutated scratch copy my cases went red 2 (Mut-3/4) while the suite stayed 79/79
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory, metaharness-wave-adr0096/trace-ir-domain/adversary-scratch under the operator's cache directory (17 new paths this pass, incl. the recreated mutant-tree/ with a 1.5G target/)
needs-coordinator: none

## 1. What I touched

`git status --short --untracked-files=all` after the pass: the implementor's nine ` M` files and `?? spec/domains/trace.yaml` unchanged by me; my only path in the tree is `?? crates/metaharness-aep/tests/adversary_trace_ir_domain.rs` (pass-1 file, two cases appended at its end under a `pass 2` comment). `git diff d7fad09 --stat -- crates/metaharness-protocol` is empty. Pass-2 lease `adversary-trace-ir-domain-pass2-20261020` acquired (exit 0) and released (exit 0). The mutant copy was recreated in scratch, as the coordinator allowed, because "a mutant that survives" is only shown by running the suite against one.

## 2. Cases added (written before anything ran this pass)

File: crates/metaharness-aep/tests/adversary_trace_ir_domain.rs in the unit tree

| Case | Asserts | Alone, tree | Against Mut-3/4 (copy) |
|---|---|---|---|
| `the_opening_node_carries_the_vendors_timestamp_and_its_millisecond_derivation` | for both recorded runs the opening node's `timestamp` is the file's first `at` verbatim and `timestamp_ms` is 1 787 302 800 000 / 1 787 304 000 000 (literals from trace_ir.rs's own test values) | green | red: `decomposer-clean: the opening node's timestamp is not the vendor's verbatim` (:467) |
| `a_fresh_projection_renders_to_the_committed_golden_document` | a fresh projection of each recorded stream renders byte-for-byte to the committed `evals/aep/runs/<run>.trace-ir.json` | green | red: `10440 vs 10874 bytes; first difference at line 14: fresh "timestamp": null` (:515) |

The five pass-1 cases are kept unchanged; all seven are green against the tree (`cargo-watched test -p metaharness-aep --test adversary_trace_ir_domain`, exit 0, `7 passed`). No case was red against the tree when written; the red outputs above are the mutant copy's and are quoted in part 3.

Scenario-lane probe (derived into scratch by make-probe-suites-pass2.sh from the regenerated suite; six literals mutated, every enum mutation a declared variant): `DeclareProcessWrites/process-explicit-subtrees` count 2→3, `CloseUnsteeredStream/observed-budget-stop` reason budget→completed, `ReadToolOutcome/command-success` numeric_exit true→false, `VerifyStreamCompleteness/inconsistent-duplicated` markers 2→1, `ClassifyEventKind/unk-auth-expired` family unk→opaque, `VerifyStreamCompleteness/inconsistent-miscounting` complete false→true. Result `total 79, passed 73, failed 6`, all six named; official report `failed`; exit 101.

## 3. Suite runs (after the cases existed), verbatim where it matters

Worktree, `cargo-watched test -p metaharness-aep` (PATH with ESS 0.56.0, `RUSTC_WRAPPER=sccache CARGO_INCREMENTAL=0`), exit **0**:

```
     Running unittests src/lib.rs (target/debug/deps/metaharness_aep-1516d2f537f7c6cb)
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.55s
     Running tests/adversary_trace_ir_domain.rs (target/debug/deps/adversary_trace_ir_domain-82467b37cf66a60b)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
cargo-watched: end, 72G free, exit 0, stopped_by_watch=0
```
target/metaharness-ess-report.json `{"total":79,"passed":79,"failed":0,"skipped":0}`; official `conformance_status: passed`, `unsupported 0`.

Scratch copy with **Mut-3/4** (trace_ir.rs:296-297 `timestamp: None`, `timestamp_ms: None`; script make-mutant-copy-pass2.sh; `diff -rq` shows only trace_ir.rs differs from the worktree):

| Run in the copy | Result | Exit |
|---|---|---|
| `cargo-watched test -p metaharness-aep ess_generated_scenarios_drive_production_targets` | `{"total":79,"passed":79,"failed":0,"skipped":0}` — both survive | 0 |
| `cargo-watched test -p metaharness-aep --test adversary_trace_ir_domain` | `5 passed; 2 failed`: the two pass-2 cases, outputs quoted in part 2 | 101 |
| `cargo-watched test -p metaharness-cli --test project` | `the_committed_fixtures_are_what_this_build_produces ... FAILED` (project.rs:339); 13 passed, 1 ignored | 101 |

Scratch copy with **Mut-5** only (trace_ir.rs reset to the worktree's, then :313 `block.stream_complete = true`; script make-mutant-5.sh), `cargo-watched test -p metaharness-aep ess_generated_scenarios_drive_production_targets`, exit **101**, `total 79 passed 75 failed 4`:

```
VerifyStreamCompleteness/outcome/truncated               -> observed outcome unnamed-completeness-shape:(0, false, true)
VerifyStreamCompleteness/outcome/inconsistent-misplaced  -> observed outcome unnamed-completeness-shape:(1, false, true)
VerifyStreamCompleteness/outcome/inconsistent-duplicated -> observed outcome unnamed-completeness-shape:(2, true, true)
VerifyStreamCompleteness/outcome/inconsistent-miscounting-> observed outcome complete
```

## 4. Findings (working tree named in the header; all introduced, since trace.yaml and the arms did not exist at d7fad09)

| # | file:line | What is wrong | What was measured | What reaches it | Verdict | Origin | Severity |
|---|---|---|---|---|---|---|---|
| 1 | spec/domains/trace.yaml:51-54 (view at ess_conformance.rs:1095-1096) | the Node comment states `timestamp` is the vendor's verbatim and `timestamp_ms` is derived from it; the retained view exposes both on `opening_node` and no predicate or payload reads either, so the suite cannot see them change | Mut-3/4 pass 79/79 (exit 0); my pass-2 cases fail on them (exit 101) | `task check` → `cargo test -p metaharness-aep` passes the mutants; the workspace gate catches them only through metaharness-cli tests/project.rs:339 against the committed goldens | CONFIRMED | introduced | warning |
| 2 | spec/domains/trace.yaml:121, :129-130 (target ess_conformance.rs:1099 `fresh == retained`) | `byte_stable` compares two renders by the same build in one process — the self-referential shape — while committed rendered documents `evals/aep/runs/{decomposer-clean,plan-reviewer-clean}.trace-ir.json` exist as the cross-build fixed point P1 names and no scenario reads them | my case `a_fresh_projection_renders_to_the_committed_golden_document` is green on the tree and red on Mut-3/4 (10440 vs 10874 bytes) | the committed goldens are held by metaharness-cli tests/project.rs:334-339, not by the owning specification's suite | CONFIRMED | introduced | note |
| 3 | spec/domains/trace.yaml:125 | `census.unk == families.unk` is evaluated only over `decomposer-clean`, where both sides are 0 (both committed runs carry `unk: 0`), so it cannot fail on the input driven; the family census the view exposes (`tool_call: 6`, `session_start: 1`, …) is read by no predicate | read from the suite's `expect_view` steps and the goldens' `families` | the two `…/invariant/at/metaharness.trace.RetainedProjection/…` scenarios; the unk census is guarded by the classify arm's own asserts instead | CONFIRMED | introduced | note |

Fixes named, not applied: #1 — a RetainedDocument predicate `opening_node.timestamp_ms == 1787302800000` (an integer literal, admitted by `view_matches`; pins the fixture the way `nodes: 16` already does); `timestamp` itself needs a string literal the matcher does not admit, or a `timestamp_present == true` field on the view. #2 — expose `golden_stable` on the view (fresh render equals the committed `<run>.trace-ir.json` bytes) with `golden_stable == true`, so the specification observes P1 across builds. #3 — `families.session_start == 1` and `families.stream_closed == 1` on RetainedDocument, both integer literals.

## 5. Attacked and could not break

| Surface | Result |
|---|---|
| the pinned default `unk-auth-expired` | no hole: it is driven with `auth-expired` because that is the one kind the eighteen guards leave; a 21st kind added to the domain enum would be driven into the default and the target names the outcome from the node's own `event_kind` (ess_conformance.rs:836-843) or family, so `unk-<new>` or `<family>` ≠ `unk-auth-expired` fails by name; a kind added to `EVENT_NAMES` but not to the enum turns my `the_suite_classifies_every_wire_kind_the_domain_declares` red (green now: all 20 driven) |
| a completeness shape still undriven | none: the five verdicts `stream_completeness` returns (Truncated, Complete, Inconsistent ×3) are each driven; further combinations (misplaced and miscounting; two markers then a line) fall to the same checks and add no observable |
| the `unnamed-completeness-shape` fallback masking a failure | cannot: every generated scenario carries `expect_outcome` (run_scenario at :1516) and an `expect_event` payload with `complete`; Mut-5 fails four scenarios by name, three of them printing the unnamed tuple; one scenario fails instead of the whole run, which is the only difference from the old panic |
| regression of the 45 pre-existing scenarios | none: `event_matches` (:1339) and `view_matches` (:1425) are the pass-1 logic unchanged; the payload-vs-shape audit over 79 scenarios shows the same five expected mismatches (three absent record payloads, two generated `record_id`); probe caught 6/6 |
| byte-identical regeneration | `ess verify conform synthesize --path spec --suite-format 5 --strict-requires` exit 0 ("79 selected scenario(s), 0 refusal"); `cmp` exit 0; SHA-256 `269fd796…ecd23ba` both; `ess specify validate --path spec --strict-requires` exit 0 |
| new predicates that cannot fail | `opening_node.index == 0`, `opening_node.source_line == 1`, `census.events_total == counts.nodes`, `counts.nodes == counts.lines` each move under a one-line production mutant; `byte_stable == true` observes determinism (finding 2); NodeCount's `>= 1` bounds hold on 16/14-line runs but are not vacuous by type; only `census.unk == families.unk` is vacuous on the driven input (finding 3) |
| restated counts | 34 = 19 + 5 + 2 + 2 + 2 + 4; "two view-invariant scenarios carrying eight predicates" = NodeCount 2 + RetainedDocument 6; "eighteen guarded" = 11 family guards over 12 kinds + 7 unk guards; README/coverage/CHANGELOG say 79 |
| pass-1 findings 1–4 | closed by program: all 20 kinds driven (case green), miscounting and duplicated shapes driven and the pass-1 count-check mutant class now caught, `>= 0` bounds gone, counts restated |

## 6. Paths written outside the worktree (this pass)

Under metaharness-wave-adr0096/trace-ir-domain/adversary-scratch/ in the operator's cache directory: `regen2.log`, `regen2-conformance.json`, `make-probe-suites-pass2.sh`, `make-mutant-copy-pass2.sh`, `make-mutant-5.sh`, `pass2-matcher-probe-suite.json`, `pass2-probe-report.json`, `pass2-probe-report.results.json`, `pass2-run1-cases-alone.log`, `pass2-run2-matcher-probe.log`, `pass2-run3-package-suite.log`, `pass2-run4-mutant-suite.log`, `pass2-run5-mutant-adversary-cases.log`, `pass2-run6-mutant-cli-project-goldens.log`, `pass2-run7-mutant5-suite.log`, `pass2-mutant-exits.txt`, `mutant-tree/` (6.4M source copy, currently carrying Mut-5 in trace_ir.rs, plus `mutant-tree/target/` 1.5G — delete once recorded). Pass-1 files remain as listed in the pass-1 report. Inside the worktree's ignored `target/`, the package run overwrote `metaharness-ess-report.json`, `ess-conformance-report.json` and `ess-conformance-report.results.json` (showing 79/79). Disk after: 70G free on `/`.

Commands and exit codes, in order: `worktree hook session-start` 0; `ess --version` 0 (0.56.0); `ess verify conform synthesize` 0; `cmp` 0; `ess specify validate` 0; cases alone 0; matcher probe 101; `cargo test -p metaharness-aep` 0; Mut-3/4 conformance 0; Mut-3/4 my cases 101; Mut-3/4 `-p metaharness-cli --test project` 101; make-mutant-5.sh 0; Mut-5 conformance 101; `worktree hook session-end` 0. No commit, staging, store write, paid run or vendor launch; no GitHub/GitLab API call.

```findings
[
  {"file": "spec/domains/trace.yaml", "line": 51, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "the Node comment at :51-54 states timestamp is the vendor's verbatim and timestamp_ms is derived from it, the view exposes both on opening_node, and no predicate or payload reads either, so production mutants setting both to None pass all 79 scenarios and are caught only by metaharness-cli tests/project.rs:339"},
  {"file": "spec/domains/trace.yaml", "line": 121, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "byte_stable compares two renders by the same build in one process (ess_conformance.rs:1099) while the committed evals/aep/runs/*.trace-ir.json documents are the cross-build fixed point P1 names and no scenario reads them; a fresh projection does render to them today"},
  {"file": "spec/domains/trace.yaml", "line": 125, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "census.unk == families.unk is evaluated only over decomposer-clean where both sides are 0, so it cannot fail on the input driven, and the family census the view exposes is read by no predicate; families.session_start == 1 and families.stream_closed == 1 would observe values"}
]
```
