---
format: aep.planning-md/1
id: story:aep-eval-checks-green-at-baseline
kind: story
status: implemented
title: The AEP eval checks are green at baseline and judge the decomposition edge, not every edge
summary: Point the three document checks at the canonical charter expectations and make E4 assert the decomposes edge of the new-story example; run-checks.sh exits 0 against agentplugins 0.7.0 and aep 0.51.0.
scope:
- confidence: cited
  path: evals/aep/README.md
- confidence: cited
  path: evals/aep/checks
revision: 6
---
# Story: The AEP eval checks are green at baseline and judge the decomposition edge, not every edge

## Context

`evals/aep/checks/run-checks.sh` is red on `origin/main` for two reasons that predate the plugin
rename (measured 2026-09-03: 3 pass / 66 fail, 8 of 9 checks `red_all`):

1. `check-runner-verdict.sh`, `check-trace-documents.sh` and `check-live-evidence.sh` read
   `expectations.decomposer.trace.yaml` and `expectations.plan-reviewer.trace.yaml` under
   `evals/aep/`, which do not exist in this repository; the per-agent charter expectations live in
   aep at `conformance/eval/decomposer-charter/expectations.trace.yaml` and
   `conformance/eval/plan-reviewer-charter/expectations.trace.yaml` (aep 0.51.0).
2. `check-decomposes-edge-examples.sh` E4 greps every `--relate <rel>:epic:` token in the
   decomposer charter and the planning skill and requires exactly one; agentplugins 0.7.0 teaches
   `decomposes:epic:` in the `aep artifact new story` example and `blocks:epic:` in a blocker
   example (`plugins/aep-plan/agents/decomposer.md:93`), which is legitimate.

## Acceptance

The three document checks (T1–T8) read the charter expectations from their canonical AEP location
through `AEP_REPO`, with a named `red_all` reason when it is unset or the documents are unreadable;
E4 asserts that the `aep artifact new story` example's relation is `decomposes` and stays green
beside a `blocks:epic:` blocker example; these nine rows go FAIL → PASS against agentplugins 0.7.0
and aep 0.51.0 and no row goes backward. `run-checks.sh` still exits 1: the remaining 58 rows verify
`evals/aep/run-agents.sh`, its prompts and fixtures, and three paid live-run recordings that were
never produced (37 + 11 + 7 rows), a pre-task blob `b83c623` no repository still carries (2 rows),
and the subject checkout's cleanliness (1 row); `evals/aep/README.md` enumerates each red row by
what it waits for. Building the runner and recording the runs is a separate story.

## Notes

`evals/aep/README.md` describes how the checks are run; keep it true. Recorded transcripts under
`evals/aep/runs/` and `checks/transcripts/` are not rewritten.
