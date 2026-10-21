---
format: aep.planning-md/3
id: review-result:adr0096-w5-h11-adversary-1
kind: review-result
status: active
title: Adversary, wave 5 unit 1 (H11 memory walk shapes), pass 1
owner: metaharness
refs:
- provider: atlas
  reference: adr-0096
relations:
- reviews: story:claude-h11-memory-walk-shapes
revision: 1
---
unit: story:claude-h11-memory-walk-shapes, pass 1 — uncommitted tree wt-f6315c5a442d (wave 5 unit 1, base ca40260)
verdict: CONFIRMED (the H11 code holds; two documentation drifts red, warning; no blocker)
cases: executed 1001→1010, red 2
origin: introduced 7 / pre-existing 0 / undecided 2 (three of the introduced are INFEASIBLE)
wrote-outside-worktree: 6 paths (metaharness-wave-adr0096/w5/u1-adv1 under the operator's cache directory)
needs-coordinator: a base build at ca40260 if the golden-transcript float failure (finding 11) needs a measured origin

## Cases added (kept, `crates/metaharness/tests/adversary_w5u1_h11.rs`)

| case | asserts | red |
|---|---|---|
| `adversary_w5u1_research_page_opens_with_its_title` | the research page's first line is its `#` title | a sentence fragment was pasted before the title |
| `adversary_w5u1_hermetic_page_does_not_say_h11_stays_measured_on_claude` | `website/docs/hermetic.md` no longer says "H11 stays measured" for Claude | the page still said it |
| `walk_terminates_over_rules_symlink_loops_and_names_linked_files`, `walk_survives_an_unreadable_rules_directory`, `walk_names_a_non_utf8_rules_file`, `every_shape_at_the_copy_root_is_the_copys_own_and_not_in_h11`, `a_version_banner_is_not_a_pinned_canary_key`, `each_pin_reads_its_own_canary`, `b10x_walk_keeps_two_shapes_and_its_verdict` | walk robustness, version matching, b10x unchanged | green |

## Attacked and not broken

The walk over symlink loops, unreadable directories and non-UTF-8 names; memory files at the copy root
stay out of H11; a version banner is not a canary key; each pin reads its own canary row; the b10x walk
keeps two shapes and its verdict.

## Disposition (coordinator)

- Fixed in the implementor's correction round (merged in `48668eb`): finding 1 (research page title), 2
  (`website/docs/hermetic.md:71`), 3 (`audit.rs` doc placement), 4 (design § 8.1 H11 cell marks a27/a33
  superseded), 5 (a leaking canary row can be recorded and reads `gap`; pin test
  `every_pinned_version_records_its_memory_canary` in `crates/metaharness-claude/tests/adapter.rs`), 6
  (fold test in `crates/metaharness/src/vectors.rs`), 7 (`MEMORY_NOT_PROBED` caveat in
  `crates/metaharness-claude/src/lib.rs`).
- Kept as adversary tests: all nine cases above.
- No change (INFEASIBLE): findings 8, 9, 10.
- Open: finding 11 (golden float under `-p metaharness-claude` alone), no story.
- Became a story: none from this pass; the independent review's H11 findings are
  `story:h11-claims-match-what-was-measured`.

## Findings

```findings
- file: docs/research/2026-10-21-claude-code-2.1.304-adapter-surface.md
  line: 1
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a fragment was prepended to the H1 title; red test adversary_w5u1_research_page_opens_with_its_title; fixed
- file: website/docs/hermetic.md
  line: 71
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "'H11 stays measured.' is stale for Claude after a36; red test adversary_w5u1_hermetic_page_does_not_say_h11_stays_measured_on_claude; fixed"
- file: crates/metaharness/src/audit.rs
  line: 1069
  category: maintainability
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: memory_held_to_canary was inserted under attested's doc comment, leaving attested undocumented; fixed
- file: docs/design/metaharness-protocol-v0.1.md
  line: 1248
  category: contract-drift
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the H11 cell keeps a33/a27 text that a36 contradicts, unmarked as superseded; fixed
- file: crates/metaharness-claude/tests/adapter.rs
  line: 152
  category: contract-drift
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: decision C says a leaking canary turns H11 to gap, but the tests made a leaking row unrecordable, so the gap branch was synthetic-only; fixed
- file: crates/metaharness/src/vectors.rs
  line: 636
  category: test-gap
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the launch_none + launch_loopback fold was untested; dropping launch_loopback survived; fixed with a fold test
- file: crates/metaharness/src/audit.rs
  line: 1076
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: undecided
  message: managed memory (/etc/claude-code/CLAUDE.md) is neither walked nor probed and the ok detail omitted the a36 § 6 caveat; fixed (MEMORY_NOT_PROBED)
- file: crates/metaharness/src/audit.rs
  line: 1076
  category: correctness
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: hermetic_floor with memory_canaries None reads Claude H11 from the attestation alone; only Run::hermetic_floor builds the inputs
- file: crates/metaharness/src/builder.rs
  line: 2085
  category: robustness
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a symlinked .claude/rules is walked without bound and an unreadable directory is skipped silently; information only
- file: crates/metaharness-claude/src/launch.rs
  line: 1927
  category: privacy
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the H11 how names operator memory file paths in session.started; no credential values
- file: crates/metaharness-claude/src/vectors.rs
  line: 430
  category: test-environment
  severity: note
  verdict: CONFIRMED
  origin: undecided
  message: golden-transcript fails on a float only under -p metaharness-claude; serde_json feature unification, untouched by the diff; open
```
