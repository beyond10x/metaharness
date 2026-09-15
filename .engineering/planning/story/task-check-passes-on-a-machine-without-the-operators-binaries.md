---
format: aep.planning-md/1
id: story:task-check-passes-on-a-machine-without-the-operators-binaries
kind: story
status: implemented
title: task check passes on a machine that has none of the operator's binaries
summary: 'The Gate workflow (gate.yml, 2026-09-15) ran task check on a clean runner four times: run 34911935585 failed because b10x-harness was not on the run''s constructed PATH; run 34912326156 because it was installed into ~/.cargo/bin instead of ~/.local/bin; run 34912870204 because metaharness_preflight never checked the PATH (fixed in aa673e3); run 34913587922 because crates/metaharness-cli/tests/aep_resume.rs:70 (legacy_launch_resumes_without_spending_or_losing_configuration) is refused with ''steps.yaml cannot produce evidence this task''s plan will demand, and --allow-evidence-gap was given: test_result …; static_analysis …'' although the same test passes on the operator''s machine with AEP pinned at 28abe09b. cargo test stops at the first failing target, so later targets are unmeasured. Until every target passes on the runner, the Gate is red on every push.'
revision: 5
---
# Story: task check passes on a machine that has none of the operator's binaries

## Outcome

Anyone pushing to metaharness gets a Gate verdict about their change, because `task check` no longer
depends on binaries that exist only on the operator's machine.

## Context

`gate.yml` (2026-09-15) ran `task check` on a clean runner four times and it went red four times,
each for a different machine dependency:

| run | cause |
|---|---|
| 34911935585 | `b10x-harness` was not on the run's constructed PATH |
| 34912326156 | it was installed into `~/.cargo/bin` instead of `~/.local/bin` |
| 34912870204 | `metaharness_preflight` never checked the PATH — fixed in `aa673e3` |
| 34913587922 | `crates/metaharness-cli/tests/aep_resume.rs:70` refused, although it passes on the operator's machine with AEP pinned at `28abe09b` |

`cargo test` stops at the first failing target, so every later target was unmeasured and the Gate was
red on every push. The fourth cause is the one this story closes: the test launched a child process
whose PATH did not contain the binary under test, so `metaharness_preflight` refused with
"steps.yaml cannot produce evidence this task's plan will demand, and `--allow-evidence-gap` was
given".

## Acceptance

- `crates/metaharness-cli/tests/aep_resume.rs` prepends `CARGO_BIN_EXE_metaharness`'s directory to
  the child's PATH, so the preflight finds the binary on a runner that has none installed.
- The test was observed red-then-green on a PATH holding neither binary.
- A Gate run on a clean runner reaches a conclusion of `success`, which proves every target ran
  rather than only that the fourth one did.

## Out of Scope

- Making `Gate` a **required** status check on `main`. `main`'s rules name no required check today
  (`AGENTS.md`), so a green Gate still does not block a red merge. That is ORG-0080 and needs a
  ruleset, not a code change.
- The AEP pin itself. `docs/ROADMAP.md` records the eight AEP crates pinned at `28abe09b`; moving
  them is a separate decision.
- The installed-binary lag on the operator's own machine (ORG-0009).

## Ambiguities

- `inferable` — which of the three test files is verified where: answered by `README.md:66`, written
  in the same commit.

## Open Questions

None.
