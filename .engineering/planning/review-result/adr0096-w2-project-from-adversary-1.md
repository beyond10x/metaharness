---
format: aep.planning-md/3
id: review-result:adr0096-w2-project-from-adversary-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:project-reads-vendor-transcripts: red, 2 warnings, 4 notes'
owner: metaharness
relations:
- reviews: story:project-reads-vendor-transcripts
revision: 1
---
unit: story:project-reads-vendor-transcripts — working tree wt-62bdbc037f65 (branch impl/project-reads-vendor-transcripts, uncommitted over 83f147860fc1b2e0158e48e1b5b0e802812f52d3)
verdict: red — strongest CONFIRMED at severity warning; no blocker; the acceptance statement holds
cases: executed 352→356, red 2 (before = this run with my one file deselected; the implementor's report says 434 for the same three lanes, counted in a way I don't know)
origin: introduced 6 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory (metaharness-wave-adr0096/w2-project-from/adversary-scratch under the operator's cache directory, contents in part 6)
needs-coordinator: whether `vendor_transcript.path` "as given" (a28 point 2) is the intended contract or a replay should carry a canonical name (F1 is a design call); the 434-vs-352 count discrepancy

## 1. Proof of touch

`git --no-pager diff 83f1478 --stat` from the worktree: the implementor's 14 files, `3304 insertions(+), 33 deletions(-)`, identical to the brief's numbers before and after my `cargo fmt -p metaharness-cli` (which therefore reflowed only my file). `git status --short` adds exactly one path of mine:

```
?? crates/metaharness-cli/tests/adversary_project_from.rs   (179 lines, test file)
```

No implementation, spec, reader, fixture or doc touched. No commits, no staging, no store writes.

## 2. Cases added — crates/metaharness-cli/tests/adversary_project_from.rs in the unit tree

All four drive the built binary (`CARGO_BIN_EXE_metaharness`) so stderr is read, not assumed. Each was written before any build, then run alone.

| # | case (line) | asserts | now | solo run |
|---|---|---|---|---|
| 1 | `a_record_with_a_foreign_first_line_is_refused_at_line_one_not_projected_behind_it` (:47) | one Codex line + the whole Claude golden, `--from claude-code-stream-json` → exit 2, nothing written, stderr names `line 1`, `session_meta`, the format | green | `test result: ok. 1 passed` exit 0 (kills mutant A, part 4) |
| 2 | `a_harness_replay_claims_no_version_or_cwd_the_verb_was_not_told` (:79) | b10x golden opening node: `harness_version` null, `cwd` null, `model == "b10x-emulated"` | green | `ok. 1 passed` exit 0 (kills mutant B) |
| 3 | `the_same_record_projects_to_the_same_bytes_however_its_path_is_spelled` (:145) | Claude golden by absolute and by relative path → same bytes | **red** | see below, exit 101 |
| 4 | `a_node_of_a_replayed_record_cites_the_record_line_it_came_from` (:113) | Codex golden: every opaque node's `source_line == kind.source_line` | **red** | see below, exit 101 |

Case 3 red output, verbatim (solo run; line numbers predate the fmt reflow):
```
thread 'the_same_record_projects_to_the_same_bytes_however_its_path_is_spelled' (2289790) panicked at crates/metaharness-cli/tests/adversary_project_from.rs:127:5:
assertion `left == right` failed: the document is a function of the record, not of how its path was typed; the fields that moved: vendor_transcript {"bytes":6624,"digest":"ff0d5eeb…179a","path":"<worktree>/crates/metaharness-claude/fixtures/golden/transcript.jsonl"} vs {"bytes":6624,"digest":"ff0d5eeb…179a","path":"../metaharness-claude/fixtures/golden/transcript.jsonl"}
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2 filtered out
cargo-watched: end, 65G free, exit 101, stopped_by_watch=0
```
Case 4 red output, verbatim (solo run):
```
thread 'a_node_of_a_replayed_record_cites_the_record_line_it_came_from' (2354081) panicked at crates/metaharness-cli/tests/adversary_project_from.rs:116:9:
assertion `left == right` failed: node 3 cites stream line 4 but the reader read it off record line 3
  left: Number(4)
 right: Number(3)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3 filtered out
cargo-watched: end, 66G free, exit 101, stopped_by_watch=0
```

## 3. Suite run (after the cases existed)

```
PATH=<ess-0.56.0 path>:$PATH RUSTC_WRAPPER=sccache CARGO_INCREMENTAL=0 \
  cargo-watched test -p metaharness -p metaharness-cli -p metaharness-aep
…
test a_node_of_a_replayed_record_cites_the_record_line_it_came_from ... FAILED
test the_same_record_projects_to_the_same_bytes_however_its_path_is_spelled ... FAILED
test result: FAILED. 2 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.65s
cargo-watched: end, 62G free, exit 101, stopped_by_watch=0
```
18 test binaries, 356 cases executed, 17 binaries green including the unit's `tests/project.rs` (21 passed, 1 ignored) and the ESS drive (87 scenarios). The two reds are mine. Log: adversary-scratch/suite.log.

Mutant copy (`adversary-scratch/mutant`, `.git` and `target` excluded, two edits to `crates/metaharness/src/viewer.rs` only — A: `:230` `emissions.first()` → `emissions.iter().map(..).find(SessionStarted)`; B: `:213` `B10xSeams::new(None, None, None)` → `new(Some("0.9.1"), None, Some("/invented/cwd"))`):

| target on the mutant | result | exit |
|---|---|---|
| `-p metaharness-cli --test project` (the unit's) | `ok. 21 passed; 0 failed; 1 ignored` | 0 |
| `-p metaharness --lib viewer` | `ok. 8 passed` | 0 |
| `-p metaharness-aep ess_generated_scenarios_drive_production_targets` | `ok. 1 passed` (87/87) | 0 |
| `--test adversary_project_from` | case 1 `left: 0 right: 2` (A survives the unit's suite, dies here); case 2 red at :92 (B likewise); cases 3, 4 red as on the tree | 101 |

Both mutants survive everything the unit shipped. `mutant/target` (1.8G) deleted; the 6.8M source copy remains in scratch.

## 4. Findings (working tree above)

| # | file:line | what is wrong | what was measured | what reaches it | verdict | origin | sev |
|---|---|---|---|---|---|---|---|
| F1 | `crates/metaharness/src/viewer.rs:187` | `vendor_transcript.path` is the argv spelling, so one record yields two documents by how it was named | case 3 red; probe: Codex golden cwd-relative vs absolute differs at two `"path"` fields | any two invocations from different cwds or spellings; `recorded_runs.rs:60` pins a fixed relative string for the committed streams, the verb does not | CONFIRMED | introduced | warning |
| F2 | `crates/metaharness-protocol/src/trace_ir.rs:319` (doc at :82-88) | under `--from` a node's `source_line` indexes a stream that exists nowhere, while the only file the document names is the record; the reader's own `kind.source_line` disagrees | case 4 red: node 3 says line 4, record line 3 | every `--from` document; the field doc promises a total mapping "back to the file" | CONFIRMED | introduced | note |
| F3 | `crates/metaharness/src/viewer.rs:230` | nothing pins that the rule reads the **first** emission; a28 point 4's own scenario (foreign opening line, real record behind it) projects with exit 0 under mutant A | case 1 green on tree, red on mutant; unit suite + ESS green on mutant | a transcript with a wrapper's log line or garbage first line | CONFIRMED | introduced | warning |
| F4 | `crates/metaharness/src/viewer.rs:213` | nothing pins a28 point 2 / invariant 4: a b10x replay that invents `harness_version`/`cwd` passes every shipped check | case 2 red on mutant B; unit suite + ESS green | any later edit "filling in" the version from `version.txt` or a pin | CONFIRMED | introduced | note |
| F5 | `crates/metaharness/src/viewer.rs:241-244` | the opaque detail prints the reader's opaque convention, not the line: `--from harness-json` over any foreign record says ``line 1 is `b10x/` `` (`seam.rs:115` stamps `vendor_type = ADAPTER_ID`); `--from codex-rollout` over a line without `type` says ``line 1 is `` `` (`rollout.rs:117` yields `Some("")`, which `(no type)` does not catch) | probe 1 stderr, 5 of 9 pairs | every mis-declared `harness-json`, and `codex-rollout` over a record without top-level `type` | CONFIRMED | introduced | note |
| F6 | `crates/metaharness/src/viewer.rs:794` | `--html` header labels a vendor record's digest `stream sha256:` | probe 3: `two-claude.html` headers carry `transcript.jsonl`'s digest under that label | every `--html` with `--from` | CONFIRMED | introduced | note |

Fixes named, not applied: F1 canonicalize or carry only the file name (or state in a28 that stability holds per argv); F2 let the writer take the record line where the event carries one, or document the field's meaning under `--from`; F3/F4 keep cases 1 and 2 (green on the tree, they kill the mutants); F5 treat `Some("")` as no type and do not print the adapter id as the line's type; F6 label by `metaharness.source`.

## 5. Attacked, not broken

- (a) all 9 cross pairs: exit 2, nothing written; empty and blank-only records refused ("produced no event"); a blank line before a real record projects.
- (b) the opening-node rule is the only guard and each (format, reader) pair is pinned at `project.rs:548-575`; only its "first" qualifier is not (F3).
- (c) `transcript_digest` is SHA-256 of the raw record bytes in all three documents and equals `vendor_transcript.digest`.
- (d) committed `evals/aep/runs/*.trace-ir.json` tests and ESS `golden_stable` green in the suite; wire path still `metaharness.event/1` (`project.rs:150`).
- (e) D11: `metaharness-cli/src/lib.rs:261-273` only `str::parse::<VendorSource>` and a dispatch; no format knowledge.
- (f) `ess_conformance.rs:1351-1420` names the outcome from the rendered opening node's `adapter` and the refusal from the production `FromStr`; the input picks only the fixture path.
- (g) `ess verify conform synthesize` → `cmp` exit 0, sha256 `442f4a69…` both sides; `ess specify validate` ×2 exit 0.
- (h) `--html` over two different Claude records exit 0 (columns `transcript`, `session.in`); Codex and b10x pairs exit 0; mixed formats refused, nothing written.
- No `stream_closed`, `stream_complete: false` in all three documents. Tests-that-cannot-fail: none among the unit's seven (`html.contains("transcript")` is weak but not vacuous: 0 occurrences in the Codex and b10x pages). `clippy -p metaharness-cli --all-targets -D warnings` 0; `fmt -p metaharness-cli -- --check` 0 after formatting my file.

## 6. Paths written outside the worktree

metaharness-wave-adr0096/w2-project-from/adversary-scratch/ (7.9M): `probe.sh`, `mutate.sh`, `mutant-run.sh`, `suite.log`, `case2.log`, `case3.log`, `case4.log`, `fmt.log`, `clippy.log`, `mutant-run.log`, `mutant-unit-project.log`, `mutant-viewer-lib.log`, `mutant-ess.log`, `mutant-adversary.log`, `conformance.regenerated.json`, `doc-{claude-code-stream-json,codex-rollout,harness-json}.json`, `two-{claude,codex,b10x}.html`, `abs.json`, `rel.json`, `x.json`, `err.txt`, `empty.jsonl`, `blank.jsonl`, `leading-blank.jsonl`, `rollout.jsonl`, and `mutant/` (source copy; its `target/` deleted). The harness itself wrote two background-task logs under the session's tasks directory.

## 7. Findings block

```findings
[
  {"file": "crates/metaharness/src/viewer.rs", "line": 187, "category": "property", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "vendor_transcript.path carries the argv spelling of the record's path, so the same record projected by an absolute and a relative path yields two different documents"},
  {"file": "crates/metaharness-protocol/src/trace_ir.rs", "line": 319, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "under --from a node's source_line is a position in a stream that exists nowhere and disagrees with the reader's own kind.source_line, while the field's doc promises a total mapping back to the file the document names"},
  {"file": "crates/metaharness/src/viewer.rs", "line": 230, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "no shipped test pins that the refusal rule reads the first emission: replacing first() with any-session.started survives project.rs, the viewer tests and the ESS drive, and a record with a foreign first line then projects with exit 0"},
  {"file": "crates/metaharness/src/viewer.rs", "line": 213, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "no shipped test pins a28 point 2 that the b10x replay claims no harness_version or cwd: a factory handed an invented version and cwd survives every shipped check"},
  {"file": "crates/metaharness/src/viewer.rs", "line": 244, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the opaque refusal detail prints the reader's convention rather than the line: harness-json reports `b10x/` for any foreign record and codex-rollout reports an empty type in backticks for a line without a top-level type"},
  {"file": "crates/metaharness/src/viewer.rs", "line": 794, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the --html column header labels a vendor record's digest as `stream sha256:` although under --from no stream exists and the digest is the record's"}
]
```
