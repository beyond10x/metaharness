---
format: aep.planning-md/3
id: review-result:adr0096-w2-project-from-adversary-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:project-reads-vendor-transcripts: 3 notes, one red case'
owner: metaharness
relations:
- reviews: story:project-reads-vendor-transcripts
revision: 1
---
unit: story:project-reads-vendor-transcripts, pass 2 — working tree wt-62bdbc037f65 (branch impl/project-reads-vendor-transcripts, uncommitted over 83f147860fc1b2e0158e48e1b5b0e802812f52d3, tracked diff 14 files 3459+/40−)
verdict: red — strongest CONFIRMED at severity note; no blocker, no warning; all four pass-1 red cases now green
cases: executed 352→358, red 1 (before = this run minus my six cases; the unit's own 22 in project.rs green)
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory (metaharness-wave-adr0096/w2-project-from/adversary-scratch under the operator's cache directory, part 6)
needs-coordinator: none (disk fell to 17G mid-pass and recovered to 35G+ before any cargo call; every cargo command ran through cargo-watched; no mutant copy was built this pass, so F8 is a grep-and-pin, not a mutant kill)

## 1. Proof of touch

`git --no-pager diff 83f1478 --stat`: the implementor's 14 files, `3459 insertions(+), 40 deletions(-)`; `git status --short` adds exactly one path of mine, unchanged in kind from pass 1:

```
?? crates/metaharness-cli/tests/adversary_project_from.rs   (235 lines, test file; 2 cases added this pass)
```

No implementation, spec, reader, fixture or doc touched. No commits, staging or store writes.

## 2. Cases added this pass — crates/metaharness-cli/tests/adversary_project_from.rs in the unit tree

Both written before any build; each run alone first.

| case (line) | asserts | now | solo run |
|---|---|---|---|
| `a_harness_record_opening_with_an_unknown_kind_is_refused_naming_that_kind` (:112) | `{"kind":"something-new"}` then the b10x golden, `--from harness-json` → exit 2, nothing written, stderr names `line 1` and `something-new` | **red** | exit 101, below |
| `the_viewer_labels_a_vendor_records_digest_by_its_source` (:139) | one-column `--html` over the Codex golden carries `codex-rollout sha256:<record digest>` and no `stream sha256:` | green | `ok. 1 passed` exit 0 (a pin nothing else holds, F8) |

Red output, verbatim (solo run, disk 35G at start):
```
test a_harness_record_opening_with_an_unknown_kind_is_refused_naming_that_kind ... FAILED
thread '…' (3612502) panicked at crates/metaharness-cli/tests/adversary_project_from.rs:128:5:
the refusal names the line's own kind, which the reader put in vendor_subtype: metaharness: project --from harness-json is refused for <scratch>/newer-loop.jsonl: the reader's first event is opaque: line 1 produced an opaque record of the `harness-json` reader, and a record of that format opens with session.started. …
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 5 filtered out
cargo-watched: end, 34G free, exit 101, stopped_by_watch=0
```

## 3. Suite run (after both cases existed)

```
PATH=<ess-0.56.0 path>:$PATH RUSTC_WRAPPER=sccache CARGO_INCREMENTAL=0 \
  cargo-watched test -p metaharness -p metaharness-cli -p metaharness-aep
test a_harness_record_opening_with_an_unknown_kind_is_refused_naming_that_kind ... FAILED
test result: FAILED. 5 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
cargo-watched: end, 42G free, exit 101, stopped_by_watch=0
```
18 binaries, 358 executed; the one red is mine. My four pass-1 cases are green on the corrected tree (F1–F4 answered as the coordinator stated). Then `fmt -p metaharness-cli -- --check` exit 0, `clippy -p metaharness-cli --all-targets -- -D warnings` exit 0. Logs: `adversary-scratch/p2-caseB.log`, `p2-suite.log`, `p2-fmt.log`, `p2-clippy.log`.

## 4. Findings (working tree above)

| # | file:line | what is wrong | what was measured | what reaches it | verdict | origin | sev |
|---|---|---|---|---|---|---|---|
| F7 | `crates/metaharness/src/viewer.rs:270-274` (`not_a_record_of`, the `(None, _)` arm) | the F5 correction filters the b10x reader's `vendor_type` (its adapter id) but then discards `vendor_subtype`, which is where that reader puts the line's own `kind` (`seam.rs:113-120`, `:511`); a harness record opening with an unknown kind is refused without naming it, against a28 point 4's "what the first event was" | new case red: stderr says only "produced an opaque record of the `harness-json` reader"; same wording measured on `unknown-kind.jsonl` via the binary | any harness release whose `--json` record opens with a kind this adapter does not know — the shape `story:b10x-adapter-on-harness-over-llm` expects when it re-records the golden | CONFIRMED | introduced | note |
| F8 | `crates/metaharness/src/viewer.rs:839-847` (`digest_label`) | the vendor half of the F6 correction is pinned by nothing: `grep` for `sha256:` / `digest_label` across `crates/metaharness-cli/tests/`, `viewer.rs` tests and `ess_conformance.rs` finds only the format string itself; the wire half is pinned by the committed `side-by-side.html` | binary: vendor pages carry `claude-code-stream-json sha256:…`, `codex-rollout sha256:…` (correct); no test asserts it | a later edit of the label on the vendor branch passes the gate; my green case :139 now pins it — no mutant run this pass (disk) | CONFIRMED | introduced | note |
| F9 | `crates/metaharness/src/viewer.rs:239-242` (run id = file stem) with `:187-190` (path = file name) | after the F1 correction two different records with one file name produce documents and `--html` columns that differ only in digest, bytes and nodes: both `metaharness.run` and both column headers read `record`; a driven run retains its Claude transcript as `transcript.jsonl` and its Codex rollout as `rollout.jsonl` (`builder.rs:538`, `:1082`), so a28 point 1's own use case — `--html` over two records of one format — labels both columns `transcript` | probe 2: `dirA/record.jsonl` (Claude golden) vs `dirB/record.jsonl` (`c2/session.in`): page exit 0, headers `<div class="run">record</div>` twice; the documents differ only in `transcript_digest`, `vendor_transcript.{digest,bytes}`, `events_total`, `families`, `events`, `requests` | every two-column `--from` page over two runs' retained transcripts | CONFIRMED | introduced | note |

Judgement residue (text only, no case): `crates/metaharness-protocol/src/trace_ir.rs:336-338` — `project_from` accepts a `source_lines` slice shorter than `lines` and silently fills the tail with `position + 1`, mixing record lines and stream positions in one document; the only production caller passes an equal-length vector, so nobody reaches it — INFEASIBLE, note, introduced. Fix to name: refuse or `debug_assert_eq!` the lengths.

Fixes named, not applied: F7 print the subtype in the `(None, Some(subtype))` arm (`line 1 is `kind=something-new`` in whatever spelling); F8 keep my case :139; F9 either disambiguate the column header with the digest prefix already printed below it, or let `--html` take an explicit column name.

## 5. Attacked, not broken

- source_line mapping on all three goldens (probe 1): every node 1 ≤ line ≤ N, non-decreasing, every opaque node equals `kind.source_line`, last node cites line N; my independent check that each cited record line's own `type`/`kind` fits the node's family holds on all 41 nodes (Claude line 3 → `tool_call`+`run_outcome`, line 5 → `assistant_text`+`run_outcome`; Codex line 19 `task_complete` → `unk`+finish-owed `run_outcome`; b10x line 3 cited by nothing, as the reader emits nothing for it). The unit's new case `project.rs:566` pins the same four properties.
- source_line boundaries: CRLF record → same lines, different digest, same families; no trailing newline → same lines; a trailing blank line → the finish-owed node cites that blank line 20 of 20 (consistent with "the last line", but a reader following it finds an empty line — residue, not raised).
- file-name path: identical bytes in two directories → byte-identical documents; relative vs absolute spelling → identical; two different records → differ only where they should (list in F9's row); pinned at `project.rs:527`.
- F5 wording on the 9 cross pairs (probe 3): no pair prints `b10x/` or empty backticks any more; the `(Some, None)` and `(Some, Some)` arms name the line (`session_meta`, `system`, `response_item/brand_new_item`, `system/from_a_later_release`); exit 2 and nothing written on all.
- F6 wire path: `--html` over the two committed event streams is byte-identical to `evals/aep/runs/side-by-side.html` (`cmp` exit 0); the non-ignored `the_committed_fixtures_are_what_this_build_produces` holds it.
- `ess verify conform synthesize` → `cmp` with `spec/conformance.json` exit 0 (87 scenarios); spec unchanged since pass 1.
- D11 unchanged: the CLI diff is the pass-1 parse-and-dispatch; the correction lives in the library.

## 6. Paths written outside the worktree

metaharness-wave-adr0096/w2-project-from/adversary-scratch/ (pass-1 contents as reported, plus): `probe2.sh`, `lines.py`, `conformance.pass2.json`, `p2-caseB.log`, `p2-suite.log`, `p2-fmt.log`, `p2-clippy.log`, `p2-{claude-code-stream-json,codex-rollout,harness-json}.json`, `p2-A.json`, `p2-B.json`, `p2-C.json`, `p2-C-rel.json`, `p2-same-name.html`, `p2-side-by-side.html`, `p2-two-claude.html`, `p2-codex.html`, `p2-codex-b.html`, `p2-crlf.json`, `p2-trailing.json`, `p2-nonl.json`, `dirA/`, `dirB/`, `dirC/` (one `record.jsonl` each), `unknown-kind.jsonl`, `codex-foreign-item.jsonl`, `claude-later-system.jsonl`, `crlf.jsonl`, `trailing-blank.jsonl`, `no-newline.jsonl`, `err.txt`, `err-a.txt`, `x.json`. The `mutant/` source copy from pass 1 remains (no `target/`). The harness wrote one background-task log under the session's tasks directory.

## 7. Findings block

```findings
[
  {"file": "crates/metaharness/src/viewer.rs", "line": 272, "category": "acceptance", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the corrected opaque wording discards vendor_subtype, which is where the b10x reader puts the line's own kind, so a harness record opening with an unknown kind is refused without naming it, against a28 point 4"},
  {"file": "crates/metaharness/src/viewer.rs", "line": 839, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "no shipped test pins the vendor-format digest label on the --html header; only the wire half is held by the committed side-by-side.html"},
  {"file": "crates/metaharness/src/viewer.rs", "line": 239, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "with the path reduced to the file name and the run id the file stem, two driven runs' retained transcripts (both transcript.jsonl) render as two --html columns with identical headers, distinguishable only by the digest line"}
]
```
