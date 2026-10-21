---
format: aep.planning-md/3
id: review-result:adr0096-w5-catalogue-adversary-1
kind: review-result
status: active
title: Adversary, wave 5 unit 4 (caller-supplied catalogue), pass 1
owner: metaharness
refs:
- provider: atlas
  reference: adr-0096
relations:
- reviews: story:caller-supplied-catalogue-on-every-kind
revision: 1
---
unit: story:caller-supplied-catalogue-on-every-kind, pass 1 — uncommitted tree wt-86f9c2d5026c (wave 5 unit 4, base 6ad241c, amendment a40)
verdict: NEEDS-CHANGE (2 red cases, 4 surviving mutants, no blocker)
cases: executed 1081→1086, red 2
origin: introduced 8 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory (metaharness-wave-adr0096/w5/u4-adv1 under the operator's cache directory, with a scratch tree for the mutant probe)
needs-coordinator: findings 6 (refuse `--allow-program` beside `--catalogue`?) and 7 (does Loom need two tool-contract families in one session?)

## Cases added (kept, `crates/metaharness-tools/tests/adversary_w5u4.rs`, `crates/metaharness/tests/adversary_w5u4.rs`)

| case | asserts | result |
|---|---|---|
| `every_document_the_parent_accepts_reads_back_from_its_scratch_copy` | a document the parent accepts reads back unchanged from its JSON scratch copy | red (DEL, C1, U+FFFE written raw) |
| `the_scratch_catalogue_the_claude_vector_pins_is_one_the_server_serves` | the scratch file in `c1-caller-catalogue` is one `mcp-serve` accepts | red (missing `profile`) |
| `a_rendering_of_exactly_the_limit_is_published_and_one_byte_more_is_not` | 46-byte name published, 47 refused | green, kills mutant M3 |
| `a_hyphenated_rendering_is_published_and_resolves` | a name with `-` is published | green, kills mutant M4 |
| `under_a_catalogue_a_call_named_like_a_catalogue_question_is_denied_by_the_frame` | `tool_search`, `tool_describe` are `Deny` under a catalogue | green, kills mutants M1 and M2 |

Mutants, in a scratch copy, each surviving the shipped suite: M1 `run.rs:1009` publishes-verbs guard
dropped; M2 `run.rs:951` catalogue term dropped; M3 `caller.rs:366` `>` to `>=`; M4 `caller.rs:360` `-`
dropped.

## Attacked and not broken

Every refusal precedes the spawn, the frame and any scratch write; vendor built-ins stay off (`--tools ""`,
`--strict-mcp-config`, one server, `--allowedTools mcp__metaharness`, hook matcher `""`); only an exact
`mcp__metaharness__<name>` resolves; duplicate names and renderings are refused by the pinned validator;
without a catalogue the `RunSpec` field is skipped and `frame/1` is untouched; run and server read the
subject from the same `path`; `expected_sha256` still applies; `aep drive` is unchanged; the Codex refusal
matches a40.

## Disposition (coordinator)

- Fixed in fix round 1: finding 1 (`render_scratch_copy`, `crates/metaharness-tools/src/caller.rs:60`,
  escapes what the reader refuses, and the copy is read back before the spawn,
  `crates/metaharness/src/builder.rs:1546`); finding 2 (the vector uses a valid document,
  `crates/metaharness-claude/src/vectors.rs:874`, `:945`, fixture regenerated); finding 6 (refused by name,
  `catalogue-with-allow-program`: `builder.rs:1838`, `crates/metaharness-claude/src/launch.rs:1364`,
  `crates/metaharness-cli/src/lib.rs:337`, `crates/metaharness/src/refusal.rs:273`, `:692`); finding 8 (4 MiB
  cap, `caller.rs:40`, `:207-240`). Fix-1 gate: 94 test binaries, 1100 passed, 0 failed, 49 ignored.
- Kept as adversary tests: findings 3 and 4 (the five cases above).
- Became stories: finding 5 is `story:codex-catalogue-on-app-server`; finding 7 is
  `story:several-tool-contract-families-per-session`.

## Findings

```findings
- file: crates/metaharness-tools/src/caller.rs
  line: 215
  category: property
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the JSON scratch copy wrote DEL, C1 and U+FFFE raw, so a catalogue the parent accepted was refused by mcp-serve and the session had no tools; fixed in fix 1
- file: crates/metaharness-claude/src/vectors.rs
  line: 936
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: c1-caller-catalogue pinned a scratch catalogue mcp-serve refuses (missing profile) and checked offered_tools against names derived from the same document; fixed in fix 1
- file: crates/metaharness/src/run.rs
  line: 1009
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: dropping the publishes_verbs guard here or the catalogue.is_none term at 951 let tool_search through under a catalogue and the shipped suite stayed green; kept as an adversary test
- file: crates/metaharness-tools/src/caller.rs
  line: 366
  category: mutant
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the 46-byte boundary and the hyphen at 360 were untested; > to >= and dropping '-' both survived; kept as adversary tests
- file: docs/design/metaharness-protocol-v0.1.md
  category: acceptance
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: the story's every-kind acceptance is unmet for Codex, which refuses the catalogue until an app-server route and a live probe exist (a40 section 2); now story:codex-catalogue-on-app-server
- file: crates/metaharness/src/builder.rs
  line: 1789
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: --allow-program beside --catalogue was accepted and had no effect; refused by name in fix 1 (catalogue-with-allow-program, a27)
- file: crates/metaharness-tools/src/caller.rs
  line: 165
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: one tool-contract/1 document is one family (tool-contract validate.rs at 60d4d0e) and --catalogue takes one file, so a session cannot be given fs reads and code writes together; now story:several-tool-contract-families-per-session
- file: crates/metaharness-tools/src/caller.rs
  line: 148
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the catalogue was read with no size limit (inferred, not run); capped at 4 MiB in fix 1
```
