---
format: aep.planning-md/3
id: story:aep-eval-native-arm-removed
kind: story
status: implemented
title: aep-eval stops calling b10x-harness workflow
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/metaharness-aep-eval/src/lib.rs
- confidence: cited
  path: crates/metaharness-cli/tests/live_scope_documented.rs
- confidence: cited
  path: evals/aep/README.md
- confidence: cited
  path: website/docs/status.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-22T02:24:10Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-22T02:24:10Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-22T07:46:31Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
# Story: aep-eval stops calling `b10x-harness workflow`

## Scheduling

Approved 2026-10-22 by `b10x-chef-orchestrator` for wave 8, built directly after
`story:b10x-api-key-from-keychain` and ahead of the rest of the wave. Harness 0.18.0 removes
`b10x-harness workflow plan|run`; every caller here fails against it until this story merges. The
orchestrator needs the merge commit so harness can ship the removal.

## Outcome

`aep-eval` keeps `preflight`, `driven --arm b10x` and `driven --arm claude`, and nothing in this
repository invokes `b10x-harness workflow`.

## Decisions (coordinator, 2026-10-22)

- D32: drop the native arm. Converting it to a `metaharness aep drive` run with harness as the harness
  would duplicate `driven --arm b10x` (`crates/metaharness-aep-eval/src/lib.rs:129-134`), which already
  compares harness with Claude Code on the same work under the same governor. The eval has no Codex arm
  today (same lines); this story does not add one. The hidden `metaharness aep drive hook` and
  `transition` verbs and their tests (`crates/metaharness-cli/tests/aep_native.rs`) stay.
- D34: `preflight_operator_boundary` (`lib.rs:980`) is removed. It checks the workflow `operator` step
  kind, which no remaining arm uses.
- D35: `preflight_confined` (`lib.rs:893`), which `preflight` (:759) and the driven b10x arm (:385) call,
  keeps its proof (the staged driver creates `decision-blocker:confined-preflight` at `open` inside
  substrate, no model contacted) without `workflow run`: `b10x-harness run` with the same confinement
  flags against a scripted endpoint that `aep-eval` serves on `127.0.0.1`, answering one `run` tool call
  of the driver and then a final message. If the pinned harness cannot complete that (cite the refusal),
  the confined preflight is removed instead and `evals/aep/README.md` states that the driven b10x arm no
  longer proves lifecycle visibility under confinement before it spends.
- D36: `evals/aep/opus-5-prices.json` stays (cited by
  `docs/research/2026-09-15-b10x-harness-0.12.1-adapter-surface.md:27`). The native sections of
  `evals/aep/README.md` are kept as history under a line saying the arm was removed in this release, not
  deleted.

## Context

- `b10x-harness workflow` callers in `crates/metaharness-aep-eval/src/lib.rs`: `workflow plan` :260 and
  `workflow run` :297 (native arm), `workflow run` :907 (`preflight_confined`) and :996
  (`preflight_operator_boundary`). :1185 runs AEP's `workflow flow`, not harness (story scoper,
  2026-10-22).
- Native-only symbols: `EvalCommand::Native` :46, `NativeArgs` :74-125, `native()` :237-366,
  `authorize_native` :483-485, `project_flow` :1175-1199, `rewrite_driver_paths` :1201-1209,
  `write_context` :1211-1221, `write_hooks` :1223-1252, `probe_governor` :1260-1297,
  `print_native_census` :1576-1603, `format_microunits` :1677-1679, `run_to_file` :1773. Shared with the
  driven arm and kept: `paid_lock` :1623, `run_to_files` :1788, `check_native_binary` :1127,
  `validate_native_pin` :1159, `ensure_cgroup_scope` :1085, `preflight_host` :821.

## Acceptance

- expected_before: clap accepts `aep-eval native` (`lib.rs:46`), and `lib.rs` passes `workflow` to the
  harness binary at :260, :297, :907 and :996. After: clap refuses `native` as an unknown subcommand, and
  no source file in the workspace passes `workflow` to the harness binary.
- `preflight` and `driven --arm b10x|claude` keep their free checks; the confined lifecycle check holds
  per D35, and a test exercises the scripted endpoint without a network or a model.
- `AGENTS.md`, `README.md`, `evals/aep/README.md` and `website/docs/status.md` no longer describe a
  native arm as runnable; the lessons that apply to the driven b10x arm stay.
- No paid run.

## Scope

- `crates/metaharness-aep-eval/src/lib.rs`: the symbols above and the test module (:2148, :2155;
  `REGROUPED_VERBS` :1938 if only the native arm needs `workflow` there).
- `crates/metaharness-cli/tests/live_scope_documented.rs` doc comment :5-6.
- `AGENTS.md` :193-212, :263-276, :289-296, :307; `README.md` :75-76, :134; `website/docs/status.md` :28;
  `evals/aep/README.md` :14-115; `CHANGELOG.md` `### Removed` under `## [Unreleased]`.
