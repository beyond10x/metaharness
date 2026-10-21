---
format: aep.planning-md/3
id: review-result:adr0096-w4-context-files-adversary-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:session-request-carries-context-files: 2 warnings, 5 notes, pass-1 findings resolved'
relations:
- reviews: story:session-request-carries-context-files
revision: 1
---
unit: story:session-request-carries-context-files, pass 2 (the last) — uncommitted tree wt-f09caf0d3129 (base 32d91e8) after correction round 1
verdict: NEEDS-CHANGE (2 warnings, 5 notes, no blocker)
cases: executed 980→989, red 3
origin: introduced 6 / pre-existing 1 / undecided 0
wrote-outside-worktree: 1 directory tree (metaharness-wave-adr0096/w4/u4-adv2 under the operator's cache directory)
needs-coordinator: how finding 2 closes (b10x's channel becomes system-prompt, or a35 decision 7 is amended)

## Ledger, pass 1 → pass 2

| pass 1 | status | evidence |
|---|---|---|
| F1 blocker | resolved | 21 mention shapes × 2.1.304 and 2.1.305 inside `--append-system-prompt`: the marker in 0 requests; 13 odd-byte contents arrive verbatim in `system` |
| W1 | resolved | the wrapper's closing line inside content arrives verbatim, not closed early |
| W2 | resolved | promisor test green |
| N1 | resolved | oversized-argument test green |

Carried 0, new 7, resolved 4.

## Cases added (untracked)

`crates/metaharness/tests/adversary_context_files_pass2.rs` and `crates/metaharness-b10x/tests/adversary_context_files_pass2.rs`:

| case | state |
|---|---|
| `every_refusal_production_raises_for_a_declaration_has_an_outcome_in_the_workspace_domain` | red (production refuses for 12 reasons, the domain models 8) |
| `a_file_tracked_in_an_initialised_submodule_of_the_cwd_tree_is_not_refused_as_untracked` | red |
| b10x `a_context_file_the_pinned_loop_puts_in_its_standing_instruction_is_recorded_as_system_prompt` | red (`left: HarnessContext right: SystemPrompt`) |
| codex opening record held until stream end / released by a turn_context names its context files | green (the first catches a mutant) |
| 4 characterisation cases: modified file, `add -N`, sparse cone, hard link | green |

Suite: 986 passed, 3 failed (above), 50 ignored.

## Attacked and not broken

Digested and rendered bytes are the work-tree bytes in every case (modified tracked file, `add -N`, a file
outside a sparse cone, a tracked hard link to a file outside the tree); the record carries the sha256 of the
bytes read. The window between the descriptor read and the `git -C` check cannot be built deterministically
(fixed git PATH, nothing to inject). A pin added without a canary row fails the canary test. The channel per kind
is correct on Claude and Codex. Not checked: a tree owned by another uid, b10x natively, Codex natively with
context.

## Findings

```findings
- file: spec/domains/workspace.yaml
  line: 148
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: production refuses a --context declaration for 12 reasons and DeclareContextFiles models 8; empty path, non-UTF-8 path, absent file and non-text content have no outcome and the ESS target panics on them
- file: crates/metaharness-b10x/src/lib.rs
  line: 73
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the pinned loop renders a --context file into its standing instruction under trust="operator", yet the record says harness-context, contradicting a35 decision 7's claim that the channel shows which files gained system-prompt authority
- file: crates/metaharness/src/context.rs
  line: 301
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a file tracked by an initialised submodule of the --cwd tree is refused as not tracked by git, because ls-files runs in the superproject only
- file: crates/metaharness/src/run.rs
  line: 1289
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: dropping with_context_files from wind_up left every existing metaharness test green; only the added codex held-opening case catches it
- file: docs/design/metaharness-protocol-v0.1.md
  line: 3050
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a pin moved with a copied, unmeasured canary row passes every unit test, so "a pin moved without measuring the new binary fails before it lands" is false
- file: CHANGELOG.md
  line: 220
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the entry calls --context widened, but on b10x it now refuses absolute, outside, untracked, symlinked, non-text and over-cap files and requires --cwd, which the base passed to the loop
- file: crates/metaharness-claude/tests/adapter.rs
  line: 308
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: cargo test -p metaharness-claude alone fails the golden-transcript vector on float rendering (0.0010919999999999999 vs 0.001092) at base and tree, while the workspace run passes
```

Agent cost: 339,739 tokens, 141 tool uses, 2,392 s.
