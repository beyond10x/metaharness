---
format: aep.planning-md/3
id: review-result:adr0096-w3-adapter-adversary-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:b10x-adapter-on-harness-over-llm: one note, all pass-1 findings resolved'
relations:
- reviews: story:b10x-adapter-on-harness-over-llm
revision: 1
---
unit: story:b10x-adapter-on-harness-over-llm, pass 2 (the last) — uncommitted working tree of wt-1f0ad046d7f7 after correction round 1 (base 04ab7bed)
verdict: CONFIRMED (note only); all five pass-1 findings resolved
cases: executed 845→846, red 0
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory (metaharness-wave-adr0096/w3/u1-adv2 under the operator's cache directory, assigned)
needs-coordinator: none (cut once by HTTP 429, resumed on the same model)

## Ledger, pass 1 → pass 2

| pass-1 finding | status | checked by |
|---|---|---|
| a29 left out served reply changes | resolved | a29, the changelog and the research table list 17 moved replies over 70 calls; each of the 17 re-read in the implementor's probe output |
| the changelog left out `truncated` on a capped `find` | resolved | named in the changelog and a29 with the cause |
| stale "0.9.1 capture" comments | resolved | `seam.rs` changes are comments and the import line; the new `withheld` claim checks out |
| lock test blind spots | resolved | any git URL spelling, exactly one tools revision, parser test over the spellings |
| outdated repository lists | resolved | deleted; nothing points at the deleted text |
| new | 1 note | below |

Carried 0, new 1, resolved 5. The pass-1 files are unedited (modification times, assertion line
numbers, tracked diff sizes of the existing test files unchanged).

## Case added

`a_missing_path_outside_the_workspace_is_refused_exactly_as_an_existing_one`
(`crates/metaharness-tools/tests/adversary_served_outside_existence.rs`), green: `file_read`,
`dir_list`, `find` and `file_edit` give the same outside refusal for `../absent.txt`, `../exists.txt`
and both absolute forms, and the outside file is unchanged. At base it would fail: 0.13.3 answered
*No such file or directory (os error 2)* for a missing outside path.

## Gate

`cargo test --workspace --no-fail-fast` exit 0: 846 passed, 0 failed, 31 ignored. fmt 0, clippy 0, both
`ess specify validate` 0, `metaharness-docs check` 0.

## Attacked and not broken

"Catalogue unchanged" (verb and flat specs, `tool_describe` for all 7 entries and `tool_search`
identical on both releases); the implementor's probe (70 cases, 17 moved, every row matches); the new
lock test (a path dependency or a same-revision duplicate harness under another spelling fails it).
Left open as worded: the same tools revision under two source spellings would pass "one tools
revision".

## Findings

```findings
- file: docs/research/2026-10-20-b10x-harness-0.15.0-adapter-surface.md
  line: 128
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: more served replies move outside the 70 probed calls, among them a missing outside path now refused like an existing one (0.13.3 revealed whether it exists) and file_edit or file_write through a trailing slash now applied, and the bounded table names none of them
```

Agent cost: 337,244 tokens, 9 tool uses, 374 s on the final resume.
