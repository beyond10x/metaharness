---
format: aep.planning-md/3
id: review-result:codex-terminal-adversary-1
kind: review-result
status: active
title: Terminal boundary attack, coordinator fallback
relations:
- reviews: story:codex-terminal-failure
revision: 1
---
unit: story:codex-terminal-failure, f4a87dd55772141821cef040f581512b9df145f5
verdict: nothing found
cases: executed 311→313, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths under assigned codex-adversary scratch
needs-coordinator: native probe and integration gate

1. Tests-only addition: crates/metaharness-codex/tests/terminal_contract.rs. No production edit. The root coordinator took over this bounded role after the assigned worker failed on a session usage limit; this is not a claim of independent agent review.
2. Added nonnull-error truthiness boundaries (false, zero, empty string/list/object, followed by a successful completion) and explicit-null versus absent-error/nonstring/blank message boundaries. The real RolloutReader ran, including the assertion that no price was invented. Both tests were green on first execution; no red finding is claimed. Command: cargo test --locked -p metaharness-codex --test terminal_contract. Exit0, 2 passed; focused.log retains output.
3. Only after adding and running those cases, cargo test --locked -p metaharness-codex -p metaharness --no-fail-fast exited0: 313 passed, 7 existing ignored. Full output: full.log. Target remained in the assigned Codex worktree; jobs1, debug0, incremental0, sccache.
4. No judgement finding. Read complete unit diff, terminal design, Run::next_event/wind_up and HarnessProcess::wait callers, adapter finish, CLI shared exit path, and full test reports.
5. Could not break sticky nonnull failure, null/missing distinction or monetary absence. No live claim; no native subprocess regression was added by this pass.
6. Outside-worktree files: local cache metaharness-issue-repair/codex-adversary/focused.log, full.log, report-1.md. Managed lease commands updated registry through CLI.

```findings
[]
```
