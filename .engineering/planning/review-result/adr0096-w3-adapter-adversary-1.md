---
format: aep.planning-md/3
id: review-result:adr0096-w3-adapter-adversary-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:b10x-adapter-on-harness-over-llm: 1 warning, 4 notes, three red cases'
relations:
- reviews: story:b10x-adapter-on-harness-over-llm
revision: 1
---
unit: story:b10x-adapter-on-harness-over-llm, pass 1 — uncommitted working tree of wt-1f0ad046d7f7 (branch impl/b10x-adapter-on-harness-0-15-0, base 04ab7bed)
verdict: NEEDS-CHANGE
cases: executed 839→844, red 3
origin: introduced 5 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory (metaharness-wave-adr0096/w3/u1-adv1 under the operator's cache directory, assigned)
needs-coordinator: none (cut twice by HTTP 429 and once by the coordinator to save quota; resumed each time on the same model)

## 1. Proof of touch

The tracked diff is the implementor's: `25 files changed, 372 insertions(+), 90 deletions(-)`. Added,
untracked, formatted with `rustfmt` only:

- `crates/metaharness-tools/tests/adversary_served_reply_changes.rs`
- `crates/metaharness-tools/tests/adversary_served_containment.rs`
- `crates/metaharness-b10x/tests/adversary_golden_capture_text.rs`

No implementation file changed.

## 2. Cases

| test | asserts | now |
|---|---|---|
| `a29_names_every_served_entry_whose_reply_moved_from_0_13_3` | every served reply that differs from 0.13.3 is for an entry a29 names | red |
| `the_changelog_names_every_field_a_capped_find_changed` | `[Unreleased]` names every field a capped `find` changed | red |
| `the_adapter_describes_the_golden_capture_as_the_version_it_was_recorded_from` | comments in `src/` calling the golden "a <version> capture" use the version in `version.txt` | red |
| `no_served_write_or_edit_lands_outside_the_workspace`, `no_served_read_list_find_or_search_answers_from_outside_the_workspace` | the served surface stays contained against `..`, absolute paths, links to a directory and a file, a dangling link, a trailing `/` or `/.` | green |

Red output, each case alone before the suite:

```
file_edit, `old` that overlaps itself: 0.13.3 answered {"isError":false,"output":{"bytes":2,"path":"overlap.txt"}} and 0.15.0 answers {"isError":true,"output":"that text appears 2 times in `overlap.txt`; include enough surrounding text to name one place (edit 0)"}
file_edit, `old` that matches nothing: ... "that text appears nowhere in `plain.txt`" ... "that text appears nowhere in `plain.txt` (edit 0)"
file_edit, empty `old`: ... "... where it lands" ... "... where it lands (edit 0)"
file_read, `limit: 0`: 0.13.3 answered {"isError":false,...,"text":"","truncated":true,...} and 0.15.0 answers {"isError":true,"output":"`limit` is how many lines to read and 0 names none. `plain.txt` was not read."}
---
CHANGELOG [Unreleased] describes the capped `find` change and does not name:
`truncated`: 0.13.3 answered true and 0.15.0 answers false
---
src/lib.rs: "0.9.1 capture in `fixtures/golden/` does not carry. `"
src/seam.rs: "0.9.1 capture in `fixtures/golden/` does not. So the "
src/seam.rs: "0.9.1 capture the golden fixture holds carries no suc"
```

Origin: the same calls against `b10x-harness-tools` at `798325f0` (the base pin) in a scratch crate match
every 0.13.3 literal, so the reply changes arrive with this unit's pin move.

## 3. Suite

`cargo test --workspace --no-fail-fast` exit 101: 841 passed, 3 failed (the cases above), 31 ignored.
`cargo fmt --check` 0, clippy 0, both `ess specify validate` 0, `metaharness-docs check` 0.

## 4. Attacked and not broken

Pins (`0.15.0` → `1589aa1a` → `dd5c2a66`, tools `0.2.0` → `60d4d0e3`; one harness and one tools
revision; the acceptance grep 0 hits; tools Apache-2.0; MSRV 1.98); golden provenance (binary digest
matches the research page; a fresh loopback re-record equals `loop.jsonl` with `captured_at` and the
workspace digest masked; the emulator saw no authorization header); native fixture 7/7; exact-equality
version checks (`builder.rs:566`, `validate_native_pin`); catalogue pairing (`operation_of`/`subjects_of`
unchanged; the no-subject path at `run.rs:1025` is pre-existing and b10x carries no frame); containment of
every served entry; the research page's source claims; enforcement excerpts carry no `started` line.
Not checked: that the binary was built from `dd5c2a66` (the build tree is gone; banner, build log and
golden match are consistent).

## 5. Findings

```findings
- file: docs/design/metaharness-protocol-v0.1.md
  line: 2414
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a29 says two served replies change, but file_edit (an overlapping old is now refused, refusal text changed) and file_read (limit 0 is now refused) also changed, and the research page and CHANGELOG repeat harness's count of two
- file: CHANGELOG.md
  line: 30
  category: contract-drift
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the capped-find entry names only depth_bound_reached, while truncated also flips from true to false when only non-matching files follow the cap
- file: crates/metaharness-b10x/src/seam.rs
  line: 230
  category: contract-drift
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: comments at lib.rs:44, seam.rs:230 and seam.rs:329 still describe a 0.9.1 golden without an mcp field, while the re-recorded golden is 0.15.0 and carries mcp []
- file: crates/metaharness-b10x/src/lib.rs
  line: 163
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the lock test skips git sources with no query string or another URL spelling, passes with zero tools packages, and checks only two manifests
- file: .github/workflows/gate.yml
  line: 11
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the workflow and .cargo/config.toml list the fetched repositories without beyond10x/tools, which now arrives behind harness
```

Agent cost: 276,615 tokens over the final resume (earlier segments cut by HTTP 429 not counted).
