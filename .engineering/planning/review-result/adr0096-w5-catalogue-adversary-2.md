---
format: aep.planning-md/3
id: review-result:adr0096-w5-catalogue-adversary-2
kind: review-result
status: active
title: Adversary, wave 5 unit 4 (caller-supplied catalogue), pass 2
owner: metaharness
refs:
- provider: atlas
  reference: adr-0096
relations:
- reviews: story:caller-supplied-catalogue-on-every-kind
revision: 1
---
unit: story:caller-supplied-catalogue-on-every-kind, pass 2 (the last) — fix round 1, uncommitted tree wt-86f9c2d5026c
verdict: NEEDS-CHANGE (3 red cases, one warning, no blocker; all eight pass-1 findings resolved or routed)
cases: executed 1148→1155, red 3
origin: introduced 5 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory (metaharness-wave-adr0096/w5/u4-adv2 under the operator's cache directory; the pass-1 scratch tree reused)
needs-coordinator: whether the line-separator warning gets a fix round (it got one: fix round 2)

## Cases added (appended to `crates/metaharness-tools/tests/adversary_w5u4.rs`; pass-1 cases kept)

| case | result |
|---|---|
| `a_line_or_paragraph_separator_beside_a_space_survives_the_scratch_copy` | red: U+2028 and U+2029 beside a space read back without the space |
| `a_summary_with_a_line_separator_between_spaces_is_accepted` | red: the parent refused valid documents, "its scratch copy reads back as another catalogue" |
| `a_catalogue_file_starting_with_a_byte_order_mark_is_read` | red: refused as "more than one document" |
| `a_document_whose_copy_is_exactly_the_cap_is_accepted_and_one_byte_more_is_not` | green, kills mutant m6 |
| `a_link_to_a_device_and_a_file_that_is_not_utf8_are_refused_as_unreadable` | green, kills mutant m7 |
| `every_yaml_context_a_caller_writes_reads_back_from_the_copy` | green: CRLF, block scalar, single-quoted `\`, escape in a comment, astral characters, JSON input |

Suite with the cases: 1103 passed, 3 failed, 49 ignored. Mutants m8, m9, m10 (the allow-program refusals)
and m12 (DEL escape) are killed by the shipped suite; m5 (`builder.rs:935`) and m11 (`caller.rs:231`)
cannot be killed by any input.

## Attacked and not broken

The copy is valid in every caller YAML context; the read-back compares the whole contract structurally
before anything is accepted; refusal order holds on every path; the 4,194,304-byte cap is exact for file and
copy; a symlink to a device, a directory and non-UTF-8 text are refused; refusals carry only the caller's
own path; without `--catalogue` the output is byte-identical apart from the golden count (29→30).

## Disposition (coordinator)

- Fixed in fix round 2: finding 1 (U+2028/U+2029 escaped, `crates/metaharness-tools/src/caller.rs:77`, docs
  corrected); finding 2 (leading BOM stripped, `caller.rs:354`); finding 4's message (the nested scratch path
  and the repeated "Nothing was started", `crates/metaharness/src/refusal.rs:287`, `:708`). Fix-2 gate: 94
  test binaries, 1112 passed, 0 failed, 49 ignored.
- Kept as adversary tests: finding 3 (m6, m7) and the six cases above.
- Became a story: finding 5 (the post-open check reachable only by a race) is in
  `story:catalogue-road-leftovers` with the review's s7-3.

## Findings

```findings
- file: crates/metaharness-tools/src/caller.rs
  line: 75
  category: correctness
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: U+2028 and U+2029 were not escaped; the reader folds an adjacent space, so the parent refused a valid catalogue; a40 sections 3 and 6, CHANGELOG.md:18 and caller_catalogue.rs:194 overclaimed; fixed in fix 2
- file: crates/metaharness-tools/src/caller.rs
  line: 296
  category: correctness
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a leading UTF-8 BOM was refused as "more than one document"; stripped in fix 2
- file: crates/metaharness-tools/src/caller.rs
  line: 268
  category: test-gap
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the copy-cap >= mutant (m6) and the lossy UTF-8 mutant (m7, :244) survived the shipped suite; killed by the added green cases
- file: crates/metaharness/src/builder.rs
  line: 935
  category: test-gap
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the verify_catalogue_copy call is unreachable by any input (from_text already reads the same copy back); its message would nest a scratch path and repeat "Nothing was started"; message fixed in fix 2
- file: crates/metaharness-tools/src/caller.rs
  line: 231
  category: test-gap
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the post-open regular-file re-check is reachable only by a race (mutant m11 survives); story:catalogue-road-leftovers
```
