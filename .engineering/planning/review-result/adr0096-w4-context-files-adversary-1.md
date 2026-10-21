---
format: aep.planning-md/3
id: review-result:adr0096-w4-context-files-adversary-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:session-request-carries-context-files: Claude expands @-mentions in inlined content (blocker), 2 warnings, 1 note'
relations:
- reviews: story:session-request-carries-context-files
revision: 1
---
unit: story:session-request-carries-context-files, pass 1 — uncommitted tree wt-f09caf0d3129 (base 32d91e8)
verdict: NEEDS-CHANGE (judgement findings only; no failing case written)
cases: executed 972→972, red 0
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory tree (metaharness-wave-adr0096/w4/u4-adv1 under the operator's cache directory)
needs-coordinator: a decision on how F1 is closed; the remaining attack lines need a second pass (this pass was stopped partway by a safety classifier; its findings are probe measurements, not test-runner records)

## F1 measurement (built driver, stub endpoint, loopback only)

| mention in the inlined file | 2.1.304 | 2.1.305 |
|---|---|---|
| `@` + token at line start or after any whitespace (space, tab, CR, FF, VT, NBSP, U+3000) | expanded | expanded |
| inside a code fence | expanded | expanded |
| `@./f`, `@dir/f`, `@../f` (outside the tree), `@/abs` (outside), `@~/f` (operator HOME) | expanded | expanded |
| `@"quoted name"`, a `#L1-1` suffix, trailing `.` `,` `)` stripped | expanded | expanded |
| a directory (an `ls` result added), an untracked file, a symlink to a file outside the tree | expanded | expanded |
| Python decorator `@property` (only when a file named `property` exists) | expanded | expanded |
| `--hermetic on` and `strict` (HOME is still the operator's) | expanded | expanded |
| `@` after a letter, digit, `_ . [ { ' " , ; = | * - ! < > # + & $ ? ~ / : \ @`, a backtick or a ZWSP | not expanded | not expanded |
| e-mail, `user@host`, the path attribute, `'single quoted'` | not expanded | not expanded |
| a path that does not exist | nothing | nothing |
| the same mention in `--append-system-prompt` (arrives byte-exact) | not expanded | not expanded |
| the same mention in `--system-prompt` | not expanded | not expanded |
| the same mention in a prompt read from stdin | expanded | expanded |
| Codex 0.153.4, 5 shapes | not expanded | – |

In every expanded case `tool.requested` count was 0 and `context_files` listed only the declared file.
Recommendation: render through `--append-system-prompt` on Claude (measured unexpanded, byte-exact; cost:
system-prompt authority for file text, which makes W1 matter more); not refusal by grammar (per-pin, race
window); not neutralising (breaks verbatim and code such as decorators).

## Attacked and not broken

Tracked-check shapes (subdirectory `--cwd`, gitfile indirection, `core.worktree` elsewhere, `core.bare`) answer
consistently; a repo-config `core.fsmonitor` did not run; symlinks, FIFOs, absolute and `..` paths refused on all
kinds; the b10x loop reads `--context` without a workspace restriction, so the scratch copy works. Not checked:
hard links, the race between the tree descriptor and `git -C`, submodule false refusals, intent-to-add, sparse
checkouts, a35 against the changelog, ESS against production.

## Findings

```findings
- file: crates/metaharness-claude/src/launch.rs
  line: 750
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Claude Code 2.1.304/2.1.305 expand @-mentions in the inlined block, adding undeclared, unchecked files (outside the tree and the operator's home included) with no seam event and no record
- file: crates/metaharness-protocol/src/spec.rs
  line: 850
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: content can contain the outer wrapper's closing line, so the wrapper delimiter is not one the content cannot close
- file: crates/metaharness/src/context.rs
  line: 236
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the tracked check can trigger a promisor lazy fetch that runs a repository-configured program; GIT_NO_LAZY_FETCH=1 stopped it in a probe
- file: website/docs/reference/cli.md
  line: 55
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a task over about 60 KiB plus a capped context fails at spawn with an unnamed E2BIG refusal, so the doc's argv-limit claim is false
```

Agent cost: 222,474 tokens, 81 tool uses, 959 s.
