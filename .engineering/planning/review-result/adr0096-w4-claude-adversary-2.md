---
format: aep.planning-md/3
id: review-result:adr0096-w4-claude-adversary-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:claude-code-2-1-304-declared: one warning (managed settings vs H4), notes'
relations:
- reviews: story:claude-code-2-1-304-declared
revision: 1
---
unit: story:claude-code-2-1-304-declared, pass 2 (the last) — uncommitted tree wt-4cbd346b7b19 after correction rounds 1 and 2 (diff stat 18 files, +1663/-136)
verdict: INFEASIBLE (1 new warning: the round-1 H4 fix reads `ok` when managed settings put an organization key on the wire); nothing found that holds the unit; no new red case
cases: executed 918→918, red 0
origin: introduced 5 / pre-existing 3 / undecided 0
wrote-outside-worktree: 1 directory (metaharness-wave-adr0096/w4/u2-adv2 under the operator's cache directory)
needs-coordinator: should H4 under `none` with an endpoint read `ok`, `unk`, or `ok` only after the launch has checked managed settings

## Ledger, pass 1 → pass 2

| pass-1 finding | state |
|---|---|
| F1 `--strict-version` ignored on Claude | resolved (real driver with 2.1.303: exit 2 before spawn, no request; 2.1.305 runs, H9 `ok`) |
| F2 H4 `gap` on the placeholder | resolved as asked; see N1 |
| F3 H11 walk misses 3 shapes | carried to `story:claude-h11-memory-walk-shapes` (option C); case ignored, still red with `--ignored` |
| F4 installed 2.1.305 off the pin | resolved; native lane 7/7 on each binary |
| F5 "admitted by name" wording | resolved |
| F6 list not tied to the pin | resolved (`every_pinned_version_declares_the_plugins_its_binary_compiles_in`) |
| F7 `--cwd` H11 text | resolved; one sentence of residue (N4) |
| F8 D2 vector | resolved by explanation in a33 § 3 |
| F9 float formatting, crate alone | carried |

Suite: 918 passed, 0 failed, 43 ignored, exit 0.

## Attacked and not broken

`--strict-version`: dangling or non-executable PATH entries, symlinked binary, malformed, suffixed or
stderr-only `--version` (refused); `--version` writes nothing to HOME or the scratch config home. The H4
sentence cannot be forged by a vendor record (read from the plan's attestation) and is produced only for
`none` with an endpoint. An operator login in HOME did not reach the stub under `none`. The version-keyed list
gives `gap` for missing, unpinned or suffixed versions. The round-2 revert left only wording. Under managed
settings `cc-plugin-sec-default` appears in a credential-free record and H1a admits it by name and source.

## Findings

```findings
- file: crates/metaharness/src/audit.rs
  line: 543
  category: contract-drift
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: H4 reads ok from the launch's placeholder sentence, which managed-settings env or apiKeyHelper falsify while the record stays identical, so a strict none-with-endpoint run passes with an organization key on the wire
- file: crates/metaharness/src/builder.rs
  line: 2007
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the strict-version probe runs --version with no deadline, so a binary that never answers hangs the run before any event
- file: crates/metaharness/src/builder.rs
  line: 2057
  category: concurrency
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the guard checks the resolved path but the spawn looks up claude by name again, so a symlink swap between the two goes unchecked until H9
- file: crates/metaharness-claude/src/launch.rs
  line: 196
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: wording left over from the reverted walk says the extra memory shapes are found and not counted, though no code looks for them (also builder.rs:1959, design doc :1247)
- file: crates/metaharness-claude/src/lib.rs
  line: 7
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the crate doc still names only 2.1.304 as the pin
- file: crates/metaharness/src/builder.rs
  line: 1079
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the Codex launch never reads --strict-version, the same gap F1 closed for Claude
- file: crates/metaharness/src/builder.rs
  line: 1961
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: the H11 walk still misses .claude/CLAUDE.md, CLAUDE.local.md and .claude/rules, deferred to story:claude-h11-memory-walk-shapes
- file: crates/metaharness-claude/src/vectors.rs
  line: 418
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: golden-transcript fails on float formatting only when metaharness-claude is tested alone, which no CI path does
```

Agent cost: 398,833 tokens, 63 tool uses, 725 s.
