---
format: aep.planning-md/3
id: review-result:adr0096-w5-hook-quoting-adversary-2
kind: review-result
status: active
title: Adversary, wave 5 unit 2 (hook program quotes channel paths), pass 2
owner: metaharness
refs:
- provider: atlas
  reference: adr-0096
relations:
- reviews: story:hook-program-quotes-channel-paths
revision: 1
---
unit: story:hook-program-quotes-channel-paths, pass 2 (the last) — correction round 1, uncommitted tree wt-fbf42e0f34b2
verdict: PASS (no red case; two introduced notes, one pre-existing warning and one pre-existing note, one note INFEASIBLE)
cases: executed 1029→1031, red 0
origin: introduced 3 / pre-existing 2 / undecided 0
wrote-outside-worktree: 7 paths (metaharness-wave-adr0096/w5/u2-adv2 under the operator's cache directory)
needs-coordinator: finding 2, pre-existing, routed outside this unit

## Cases added (kept, `crates/metaharness-cli/tests/adversary_w5u2_adv2.rs`)

| case | checks | result |
|---|---|---|
| `adversary_w5u2_adv2_the_cli_refuses_a_plugin_variable_scratch_root_with_exit_2` | the built binary, with a scratch `TMPDIR` naming `${CLAUDE_PLUGIN_ROOT}`: exit 2, stderr opens with `metaharness: scratch-path-names-plugin-variable:`, nothing left under `TMPDIR` | green |
| `adversary_w5u2_adv2_the_same_cli_run_under_a_plain_tmpdir_is_not_that_refusal` | control: a plain `TMPDIR` does not produce that refusal | green |

Both run with a cleared environment and no vendor binary on `PATH`. Suite: 87 test binaries, 1031 passed,
0 failed, 49 ignored.

## Attacked and not broken

The fix-1 refusal is complete for the hook command at both pins and runs before every side effect (the
only production caller of `plan_launch` is the builder; CLI `run`, `aep drive` and the builder API all pass
through it); refusing in every decision mode is not too broad; Codex and b10x left unrefused matches a37 § 6
and the CHANGELOG; the CHANGELOG Breaking line matches the code; the three a37 fish examples reproduce under
fish 4.9.1.

## Disposition (coordinator)

- Fixed by the coordinator before the merge (`aff22dc`): finding 1 (a37 fish sentence narrowed to `\\` and
  `\'`), finding 3 (a37 § 6 scope: the "whole class" holds for the hook command only), finding 4 (CHANGELOG
  Added line for the new public items).
- Withheld: one pre-existing warning, routed outside this unit; its detail is withheld until release 0.12.0.
- No change (INFEASIBLE): finding 5.
- Kept as adversary tests: the two cases above.

## Findings

```findings
- file: docs/design/metaharness-protocol-v0.1.md
  line: 3217
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the fish sentence overgeneralised; under fish 4.9.1 only \\ and \' differ from sh, a lone backslash does not; fixed by the coordinator
- file: crates/metaharness-claude/src/launch.rs
  line: 1290
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: --mcp-config is environment-expanded by Claude (read from strings, not driven), so a TMPDIR naming ${...} would reach the owned tool server's --workspace; a37 section 6 "whole class" holds for the hook command only; a37 section 6 scoped by the coordinator
- file: CHANGELOG.md
  line: 14
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: new public functions and constants (shell_quote, shell_quote_path, PathNotUtf8::escaped, HOOK_COMMAND_PLUGIN_VARIABLES and the refusal code constants) were not listed under Added; fixed by the coordinator
- file: crates/metaharness/src/refusal.rs
  line: 271
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: Claude LaunchRefusal::PathNotUtf8 maps to Refusal::Launch without the scratch-path-not-utf8 code; unreachable through the builder, whose guard refuses first
```

One further pre-existing warning from this pass was routed outside this unit and is withheld until release
0.12.0.
