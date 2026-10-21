---
format: aep.planning-md/3
id: review-result:adr0096-w4-claude-adversary-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:claude-code-2-1-304-declared: 4 warnings (pre-existing), 5 notes, three red cases'
relations:
- reviews: story:claude-code-2-1-304-declared
revision: 1
---
unit: story:claude-code-2-1-304-declared, pass 1 — uncommitted tree wt-4cbd346b7b19 (branch impl/claude-code-2-1-304-declared, base 5eacbd4)
verdict: NEEDS-CHANGE (one acceptance-level outcome gap that predates the unit); 3 CONFIRMED red cases, all pre-existing; nothing found in the unit's own H1a, traffic-switch or wording code
cases: executed 911→914, red 3
origin: introduced 3 / pre-existing 7 / undecided 0
wrote-outside-worktree: 1 directory (metaharness-wave-adr0096/w4/u2-adv1 under the operator's cache directory)
needs-coordinator: the installed `claude` is 2.1.305 since 2026-10-20 23:13; re-pin or ship 2.1.304 and say so (finding 4)

## Cases added (untracked, `crates/metaharness/tests/adversary_claude_2_1_304_pass1.rs`)

| case | asserts | red |
|---|---|---|
| `strict_version_refuses_a_claude_launch_before_the_spawn` | a Claude launch with `--strict-version` is refused before the spawn | "--strict-version let a Claude launch through without reading the vendor's version first; the runner spawned 1 child(ren)…" |
| `h4_does_not_call_the_plans_own_endpoint_placeholder_an_undeclared_key` | H4 reads `ok` under `none` with an endpoint, where the key is the plan's own placeholder | `left: Gap right: Ok` |
| `h11_is_not_ok_over_a_memory_file_the_pinned_loader_reads_and_the_walk_does_not` | with `.claude/CLAUDE.md`, `CLAUDE.local.md` or `.claude/rules/r.md` above a declared scratch copy, H11 is not `ok` (controls: nothing planted reads `ok`; a planted `CLAUDE.md` is caught) | each shape: "H11 ok … found no CLAUDE.md and no AGENTS.md above it" |

All three fail the same way at base `5eacbd4`. Suite with the cases: 911 passed, 3 failed (this file), 41 ignored.

## Attacked and not broken

The H1a rule (foreign source, no source, another built-in's source, an uncompiled name, nameless, non-object;
case variants err toward `gap`); `@builtin` cannot be forged (2.1.304 refuses a marketplace named `builtin`);
the 12 compiled names and order match the registration code in 2.1.303-2.1.305; the traffic switch is set
exactly under `none` and `loopback`; the H11 canaries through the driver: 0 of 9 arrived (cwd and depths 1-4,
four file shapes) where the vendor run directly with `project,local` sent 9 of 9; the c1 vector pins the whole
refusal message; native lane 6/6. Not checked: operator-login or api-key sessions; a positive control for
`AGENTS.md`; managed memory under `/etc/claude-code`.

## Findings

```findings
- file: crates/metaharness/src/builder.rs
  line: 819
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: --strict-version is never read on the Claude arm, so an off-pin binary is spawned and paid for before H9 can say so
- file: crates/metaharness/src/audit.rs
  line: 520
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: H4 reads gap under credentials none with an endpoint on the placeholder key the launch itself attests as no operator credential
- file: crates/metaharness/src/builder.rs
  line: 1948
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: the H11 walk misses .claude/CLAUDE.md, CLAUDE.local.md and .claude/rules, which 2.1.304 loads from ancestors, so H11 reads ok with the operator's ~/.claude/CLAUDE.md above every scratch cwd under HOME
- file: crates/metaharness-claude/src/lib.rs
  line: 147
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: the installed claude is 2.1.305, so the 2.1.304 pin leaves H9 gap on every default run and the outcome of removing takt's H9 workaround is not met
- file: crates/metaharness-claude/src/lib.rs
  line: 159
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: docs say built-ins are admitted by name while the rule requires name and source, in lib.rs, launch.rs, the design doc and the website
- file: crates/metaharness-claude/tests/adapter.rs
  line: 62
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the BUILTIN_PLUGINS literal test claims a pin move without a re-read shows up there, but nothing ties the list to PINNED_VERSIONS
- file: crates/metaharness-claude/src/launch.rs
  line: 1605
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the --cwd H11 attestation says the tree is the run's context while a33 observes its CLAUDE.md does not reach the model at 2.1.304
- file: crates/metaharness/tests/audit.rs
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: D2 asked for takt's four-built-in list pinned as a vector; it is integration tests and conformance claude has no built-in H1a case
- file: crates/metaharness-claude/src/vectors.rs
  line: 418
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: golden-transcript fails on float formatting only when metaharness-claude is tested alone, which no CI path does
```

Agent cost: 288,198 tokens, 155 tool uses, 1,497 s.
