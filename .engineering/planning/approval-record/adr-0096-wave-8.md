---
format: aep.planning-md/3
id: approval-record:adr-0096-wave-8
kind: approval-record
status: approved
title: 'Wave 8 of 2026-10-22: field-report and first-use stories, after the 0.13.0 release'
relations:
- decides: story:codex-command-outcomes-reach-tool-result
- decides: story:narrowed-cap-refused-before-persist
- decides: story:exit-status-separates-task-control-audit
- decides: story:mcp-arguments-checked-against-declared-types
- decides: story:first-user-documents-agree-with-cli
- decides: story:b10x-api-key-from-keychain
revision: 11
transitions:
- {from: "draft", to: "approved", at: "2026-10-21T22:46:16Z", actor: "human:timo", revision: 2}
---
## Status

Approved 2026-10-22 by `b10x-chef-orchestrator` under the owner's standing grant for class-C items:
wave 8 in this repository, its commits, push, pull request and the bot fast-forward of `main`. Not a
release. Skill: aep:implementing 0.23.2, wave mode.

## Order

1. Wave 7 closes and release 0.13.0 completes first (`approval-record:adr-0096-wave-7`).
2. Before wave 8 starts, the per-repository retirement drafts that
   `story:retire-trace-formats-beside-trace-ir` routes (harness `harness events`, takt, loom) go to the
   orchestrator as story text, so it hands them to those repositories' sessions.
3. Then wave 8: each draft below is scoped (`aep:story-scoper`), moved to accepted, and implemented in
   its own managed tree.

## Units

| story | source |
|---|---|
| `story:codex-command-outcomes-reach-tool-result` | field report 2026-10-21, F1 |
| `story:narrowed-cap-refused-before-persist` | field report 2026-10-21, F3 |
| `story:exit-status-separates-task-control-audit` | field report 2026-10-21, F6 |
| `story:mcp-arguments-checked-against-declared-types` | first-use report on 0.12.0 |
| `story:first-user-documents-agree-with-cli` | first-use report on 0.12.0; field report F5 folded in |
| `story:b10x-api-key-from-keychain` | orchestrator, 2026-10-22, for a takt run; replaces `story:b10x-api-key-from-file` (archived), operator decision: the key is named by a secret name, `--api-key-secret` |
| `story:aep-eval-native-arm-removed` | orchestrator, 2026-10-22: harness 0.18.0 removes `b10x-harness workflow`; built after the keychain story, ahead of the rest |

Commits, review passes and the merge are recorded here as they happen.

## Opening (2026-10-22)

Tree `mh-w8-int`, base `a34551e` (release 0.13.0), branch `integration/adr-0096-wave-8`. Each story was
scoped by one `aep:story-scoper` at that base and moved draft → proposed → active; its `## Scope` section
carries the scoper result and the coordinator decisions D1–D13. `story:b10x-api-key-from-keychain` waits
for harness 0.17.1, which carries `--api-key-secret`.

`aep plan artifact waves` orders the five one per wave, because every scope names `CHANGELOG.md` and most
name the design document and `spec/conformance.json`. Those three are merged by the coordinator: the
CHANGELOG under `[Unreleased]`, the amendments numbered in advance, `spec/conformance.json` regenerated
after each merge. Code surfaces are disjoint except `website/docs/reference/cli.md`, so the units run
in two batches:

| batch | unit | story | amendment |
|---|---|---|---|
| 1 | u1 | `story:mcp-arguments-checked-against-declared-types` | a45 |
| 1 | u2 | `story:narrowed-cap-refused-before-persist` | a46 |
| 1 | u3 | `story:codex-command-outcomes-reach-tool-result` | a47 |
| 2 | u4 | `story:exit-status-separates-task-control-audit` | a48 |
| 2 | u5 | `story:first-user-documents-agree-with-cli`, after u4 merges | a49 if needed |

Units 1–4: `aep:implementor` at xhigh, then `aep:adversary`. Unit 5: documentation and count tests.

## Disk hold and go (2026-10-22)

| time (UTC+2) | from | message | action here |
|---|---|---|---|
| 03:5x | `b10x-chef-orchestrator` | `/` at 14G free: stop cargo, no new runs until go at 30G; clean the W8 trees not built next | the pending u2 docs check was killed; the three running agents held cargo; `cargo clean` in mh-w8-u1 (5.6 GiB) and mh-w8-u3 (6.5 GiB), their ESS report files copied out first; `/` at 81G free afterwards |
| 04:1x | `b10x-chef-orchestrator` | go at 81G free: run the u2 gate, then the next unit, one tree at a time; `cargo clean` each unit tree after it merges | builds run in one tree at a time, u2 first |

## Harness 0.17.1 and the keychain story (2026-10-22)

`b10x-chef-orchestrator`, 04:3x: harness 0.17.1 is released with `--api-key-secret`
(https://github.com/beyond10x/harness/releases/tag/0.17.1). Approved: pin it and put
`story:b10x-api-key-from-keychain` into the next wave 8 build slot, one tree at a time. The story's
fixture secret name is `llm/opencode-go-api-key`; its value is never read or printed. No live call until
the orchestrator says so, so the pin's one-turn `--strict-version` run waits for that word.

Read back by the coordinator: tag `0.17.1` is an annotated tag by `b10x-bot[bot]` on `c5cf4de5`, which
is harness `main`; that commit's author and committer are `b10x-bot[bot]`; it is 43 commits ahead of the
current pin `f0870131` and 0 behind; the release has 3 assets (two Linux archives and `SHA256SUMS`).

The story is scoped by an `aep:story-scoper` at the integration head and implemented as unit u6, from the
integration branch, in its own tree.

## Harness workflow removal (2026-10-22)

`b10x-chef-orchestrator`, 04:4x and 05:0x, relaying the operator: harness removes
`b10x-harness workflow plan|run` (0.18.0, without waiting for this repository). The only caller here is
the `aep-eval` native arm. Approved: `story:aep-eval-native-arm-removed`, built directly after
`story:b10x-api-key-from-keychain` and ahead of the rest of wave 8. The orchestrator receives the merge
commit so harness can ship the removal.

Coordinator decision D32: the native arm is dropped; `driven --arm b10x` already runs harness under
`metaharness aep drive run` beside `driven --arm claude`, so a converted native arm would duplicate it.

## Merges and gate (2026-10-22)

| unit | story | unit commit | merge |
|---|---|---|---|
| u6 | `story:b10x-api-key-from-keychain` (a50, harness pin 0.18.0) | `4905534` | `304168d` |
| u7 | `story:aep-eval-native-arm-removed` | `496cd0e` | `7332266` |
| u5 | `story:first-user-documents-agree-with-cli` (a49) | `b79f8d3` | `0a546a6` |

Units u1 to u4 merged before u6; their merges are on the integration branch below `304168d`. Every unit had
one `aep:adversary` pass and one revision; u6 also had an invariant review. Each review is a `review-result`.

The harness pin moved once, from `f0870131` to 0.18.0 (`330e2cde`), not to 0.17.1, because 0.18.0 had shipped
by the time unit u6 was built.

Gate on the integration branch at `9595d5d`: `task check` one step at a time, 6 steps exit 0, 120 test
binaries, 1354 passed, 0 failed, 50 ignored; ESS conformance 116/116.

No release in this wave.
