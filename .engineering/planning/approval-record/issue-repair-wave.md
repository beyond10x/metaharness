---
format: aep.planning-md/3
id: approval-record:issue-repair-wave
kind: approval-record
status: draft
title: Approved issue repair wave on one integration branch
relations:
- decides: epic:github-issue-repair
- decides: story:codex-terminal-failure
- decides: story:current-aep-runtime
revision: 2
---
## Authorization and boundary

Interactive operator approval on 2026-10-03: "good, approve $aep:wave", following the issue-repair handoff and the narrow evidence-redaction question. Skill version 0.19.1. Continue the authorized issue repairs without a second wave-approval question. Commit unit changes, integration and planning evidence on integration/metaharness-issue-repair. The operator separately requires one pull request later: do not submit a PR, merge main, tag or release now.

## First implementation stage

story:codex-terminal-failure (#11) and story:current-aep-runtime (#10) have disjoint source scope and are active. Existing test-only #11 changes are this session's retained patch, already reproduced red; the operator approved proceeding from this handoff. story:managed-workspace-admission (#12) follows the AEP unit because both touch drive.rs and drive_tests.rs. Additional #13-15 work remains to be concretely scoped against these results, and newly inventoried #16 maps to the existing scripted-b10x story. The ESS adoption/hardening ticket remains part of the plan; runtime entities newly introduced by later policies require an ESS domain before their stories.

## Resources and roles

Free disk: 13 GiB observed before dispatch. Measured focused build: 491 MiB, removed after retaining evidence. Bound builds to one at a time, two cargo jobs, debug info off and incremental off. Compiler cache /usr/bin/sccache is available and will be set explicitly per build. At most two implementation agents at once; the host supports three workers plus coordinator. No paid successful-model run is authorized by this wave record; offline regressions precede a separately bounded live probe.

Codex collaboration agents execute the procedures of aep:implementor and aep:adversary; this host has no plugin-specific subagent_type selector. The coordinator owns every store mutation. Unit source trees are isolated; all results integrate into the single branch. Unit paths and exact stages are appended before dispatch. No agents may edit planning or commit under personal identity.

## Retained integration state

Integration tree: wt-8e8abedfd902; branch integration/metaharness-issue-repair; opening commit 498e84bb. Scratch root: local cache metaharness-issue-repair. Source regressions pending at dispatch: crates/metaharness-codex/src/bridge.rs and crates/metaharness/tests/stream_closed.rs, both owned by #11. All other working changes are this coordinator's design/plan updates. No unrelated dirty work is present.

## Computed active selection

{
  "waves": [
    {
      "wave": 1,
      "artifacts": [
        {
          "id": "story:codex-terminal-failure",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/metaharness-cli/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/metaharness-codex"
            },
            {
              "confidence": "cited",
              "path": "crates/metaharness/src/audit.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/metaharness/src/run.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/metaharness/src/spawn_codex.rs"
            },
            {
              "confidence": "cited",
              "path": "docs/design/metaharness-protocol-v0.1.md"
            }
          ]
        },
        {
          "id": "story:current-aep-runtime",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "CHANGELOG.md"
            },
            {
              "confidence": "cited",
              "path": "Cargo.lock"
            },
            {
              "confidence": "cited",
              "path": "crates/metaharness-aep"
            },
            {
              "confidence": "cited",
              "path": "crates/metaharness-aep-eval/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "evals"
            }
          ]
        }
      ]
    }
  ],
  "collisions": [],
  "unassessed": [],
  "cycles": []
}
