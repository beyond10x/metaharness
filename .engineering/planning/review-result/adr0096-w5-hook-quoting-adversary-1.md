---
format: aep.planning-md/3
id: review-result:adr0096-w5-hook-quoting-adversary-1
kind: review-result
status: active
title: Adversary, wave 5 unit 2 (hook program quotes channel paths), pass 1
owner: metaharness
refs:
- provider: atlas
  reference: adr-0096
relations:
- reviews: story:hook-program-quotes-channel-paths
revision: 1
---
unit: story:hook-program-quotes-channel-paths, pass 1 — uncommitted tree wt-fbf42e0f34b2 (wave 5 unit 2, base 4a72d8c)
verdict: NEEDS-CHANGE (2 red cases, both low severity; quoting, TOML escaping and the refusal hold)
cases: executed 1023→1027, red 2
origin: introduced 3 / pre-existing 1 / undecided 0
wrote-outside-worktree: 14 paths (metaharness-wave-adr0096/w5/u2-adv1 under the operator's cache directory)
needs-coordinator: finding 1 (refuse a root naming a plugin variable by name, or leave it INFEASIBLE)

## Cases added (kept)

| case | crate | result |
|---|---|---|
| `adversary_w5u2_generated_roots_keep_one_word_and_their_channel` | metaharness-claude | held: 300 generated roots, each `command` splits to one `/bin/sh` word equal to the hook program path, the program passes `sh -n`, the channel paths evaluate back |
| `adversary_w5u2_config_round_trips_every_awkward_root` | metaharness-codex | held: control characters, C1, `"`, `\`, `A`, astral characters and a TOML-injection root round-trip through `config.toml` |
| `adversary_w5u2_a_root_naming_a_plugin_variable_never_reaches_the_vendor_command` | metaharness-claude | red |
| `adversary_w5u2_the_refusal_names_printable_ascii_as_written` | metaharness | red |

Suite with the cases: 1025 passed, 2 failed (the two red cases), 49 ignored.

## Attacked and not broken

The quoting property over 300 generated roots and a live channel run; the pinned Claude 2.1.304 runs the
shell form through `/bin/sh -c`; the Codex TOML round trip; refusal ordering (`guard_scratch_path` right
after the scratch directory is created at all three starts, exit 2, no credential in the message); the four
CHANGELOG and API claims; the spawn runners render no path as text; b10x is covered by a37 § 4.

## Disposition (coordinator)

- Fixed in the correction round (fix 1, merged in `aff22dc`): finding 1, by a refusal by name before the
  spawn (`scratch-path-names-plugin-variable`, `crates/metaharness-claude/src/launch.rs:1619-1628`); finding
  2, by rewording a37 § 3; finding 3, a37 cites the installed pins and labels the Codex shell unverified;
  finding 4, the a37 sentence is scoped to paths under the scratch root. Fix gate: 86 test binaries, 1029
  passed, 0 failed, 49 ignored.
- Kept as adversary tests: the four cases above, both red cases green after fix 1.
- Became a story: none from this pass.

## Findings

```findings
- file: crates/metaharness-claude/src/launch.rs
  line: 1578
  category: contract-drift
  severity: warning
  verdict: INFEASIBLE
  origin: pre-existing
  message: a scratch root containing the literal ${CLAUDE_PLUGIN_ROOT} or ${CLAUDE_PLUGIN_DATA} was planned into the settings hook command, which the pinned Claude 2.1.304 refuses to run (read from the binary's strings); refuse such a root by name before the spawn; red test adversary_w5u2_a_root_naming_a_plugin_variable_never_reaches_the_vendor_command; fixed in fix 1
- file: crates/metaharness/src/refusal.rs
  line: 557
  category: contract-mismatch
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: escape_ascii also escapes the printable bytes ', " and \, while a37 section 3 promised escaping only outside printable ASCII; red test adversary_w5u2_the_refusal_names_printable_ascii_as_written; a37 section 3 reworded
- file: docs/design/metaharness-protocol-v0.1.md
  line: 3209
  category: evidence-label
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the pinned Claude 2.1.304 and Codex 0.153.4 are installed and agree with the readings taken from newer binaries; cite the pins; the shell Codex runs -lc with is unrecorded, and under fish the POSIX quote escape does not parse; fixed
- file: docs/design/metaharness-protocol-v0.1.md
  line: 3238
  category: overclaim
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: '"wherever it would happen" overclaimed: operator paths (--cwd into the b10x and MCP --workspace, --hooks, --driver, --plugin-dir) are still rendered through display(); scoped to paths under the scratch root; fixed'
```
