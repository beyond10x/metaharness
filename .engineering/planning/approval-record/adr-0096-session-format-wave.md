---
format: aep.planning-md/3
id: approval-record:adr-0096-session-format-wave
kind: approval-record
status: draft
title: 'Wave of 2026-10-20: trace-ir/1 as the ESS domain metaharness.trace (ADR 0096 point 7)'
owner: metaharness
tags:
- wave
refs:
- provider: atlas
  reference: adr-0096
relations:
- decides: story:trace-ir-domain
- decides: epic:session-format
revision: 13
---
## Status

Proposed 2026-10-20 by session `metaharness`; approval pending from session `adr-changes` (owner's
standing exception 5 for work implementing Atlas ADRs 0096–0101). Skill: aep:implementing 0.23.2,
wave mode (`references/wave.md`). This page is the wave's record and is kept current per unit stage.

## Selection

| unit | story | objective | scope confidence | subagent types |
|---|---|---|---|---|
| 1 | `story:trace-ir-domain` | O3 (one record vocabulary; AGENTS.md § Serves), ADR 0096 point 7 | 10 cited, 2 inferred (`Cargo.lock`, `crates/metaharness-aep/Cargo.toml`, both conditional) and `README.md` inferred | `aep:implementor`, then `aep:adversary` |

N is one. Candidates read from the store (`aep plan artifact list --status draft`, `blocked`, `graph`):
`story:governance-unit-stated` is also ready (no blocker, no dependency, `AGENTS.md` only, no
collision with unit 1) and is left out because the owner's instruction framed this wave as item 1
plus the drafts; it is offered as option B in the proposal. Every other draft is blocked
(`dependency-blocker:llm-foundation-release`, `tools-first-release`, `harness-llm-release`,
`decision-blocker:aep-drive-future`) or depends on an unfinished story, or the owner excluded it
from this wave (the six ADR 0098 gap stories).

Selection path: `aep plan artifact waves --kind story --status draft --format json` (aep 0.74.0), output
below verbatim, plus judgement on readiness. The verb places `story:trace-ir-domain` behind
`story:llm-route-bindings` on an inferred `Cargo.lock` collision; that story is blocked and not a
candidate, so the disagreement is one of candidacy, not of surface. Collisions among the later drafts
are expected: their typed scope is inferred on purpose and they are serialized by `depends_on` edges
recorded in the critic rounds.

## Opening store commit contents

3 epics (`epic:session-format`, `epic:loom-agent-work-through-metaharness`, `epic:layer-pins-follow-their-owners`),
12 stories, `decision-blocker:aep-drive-future` (options keep / move to Loom / retire with costs, not
acted on), `dependency-blocker:tools-first-release`, `dependency-blocker:harness-llm-release`, 8
`review-result`s (acceptance, design, scope, parallel-safety × 2 rounds), 38 `review_outcome`
records (all `fixed`), this page. `aep plan artifact validate`: valid, 102 artifacts.

Critic panel: round 1 four `needs-revision` (5 + 5 + 2 + 11 findings); round 2 scope `approve`,
acceptance 6 warnings, design 2 warnings, parallel-safety 7 warnings, none on the wave unit;
parallel-safety round 2: "Can story:trace-ir-domain run alone now: yes". No third round. Every
round-2 finding was acted on by a body or edge change after the panel closed and recorded `fixed`;
the critics did not re-read those last changes.

## Commits this approval authorises

1. The opening store commit on `integration/adr-0096-session-format` (the contents above, plus
   moving `story:trace-ir-domain` to `active`).
2. One unit commit on `impl/trace-ir-domain`.
3. The merge of the unit branch into the integration branch.
4. The closing store commit: scope rewrite from the implementor's confirmation, evidence, the move to
   `implemented`, this page's final state, and one sentence in AGENTS.md § What this repository owns
   naming the session format `trace-ir/1` as owned (ADR 0096 point 7).
5. Push of the integration branch, a pull request to `main` and its merge, all through
   `b10x-gates bot` / `b10x-gates gh`, after CI **Gate** and **MSRV 1.98** are green (ruleset
   `main-requires-the-gate`).

Not authorised by this approval: a tag, a release, a paid run, work outside unit 1, the next wave.

## Unit 1 plan

- integration tree: managed worktree `wt-1c57662d33a0` (`worktree inspect --id wt-1c57662d33a0`
  prints its path), branch `integration/adr-0096-session-format`, base `4155db5` (main), lease
  `metaharness-18506b0d`.
- unit tree: created after the opening commit with `worktree create --purpose trace-ir-domain --base <integration head>`;
  branch `impl/trace-ir-domain`; build dir `<unit tree>/target`; scratch
  `metaharness-wave-adr0096/trace-ir-domain/scratch` under the operator's cache directory.
- builds: one at a time through `cargo-watched` in `metaharness-wave-adr0096/bin` under the
  operator's cache directory (refuses under 25G free on `/`, kills a build that crosses it);
  `RUSTC_WRAPPER=sccache`, `CARGO_INCREMENTAL=0`; ESS 0.56.0 release binary (SHA256 verified) first
  on `PATH` from `metaharness-ess-0.56.0/path` under the same cache directory.
- gate on the unit: package-scoped (`cargo test -p metaharness-aep`, `cargo clippy -p metaharness-aep
  --all-targets -- -D warnings`, `cargo fmt -p metaharness-aep -- --check`, `ess specify validate
  --path spec --strict-requires`); the whole `task check` once on the integration branch.
- stage: proposed, awaiting approval.

## Pre-flight, measured 2026-10-20T02:13Z

- free disk on `/`: 64G (floor 25G; the watch refuses a start and kills a build under it)
- primary checkout: branch main, porcelain lines: 0
- `git worktree list`: 2 entries (main and this integration tree); legacy adoption record `legacy-metaharness-metaharness-hardening-20260901` outside the managed root is not a wave tree
- builds measured in this session: none; previous wave recorded Codex-focused target 491 MiB, AEP target 805 MiB (approval-record:issue-repair-wave, 2026-10-03); the first unit build is this wave's measurement
- compiler cache: /usr/bin/sccache present, RUSTC_WRAPPER unset in the environment, set explicitly per build
- cargo profile: dev debug=line-tables-only and lld (~/.cargo/config.toml)
- ESS: spec pins 0.56.0 (strict); machine has 0.60.0; 0.56.0 release archive downloaded and checksum-verified into `metaharness-ess-0.56.0` under the operator's cache directory; `ess specify validate --path spec --strict-requires` with it: valid, exit 0
- model budget: sub-agents on the rotated shared login (adr-changes, 2026-10-20: 5h headroom 100%); N=1 so at most one implementor and one adversary, sequential

## `aep plan artifact waves --kind story --status draft --format json`, verbatim

```json
{
  "waves": [
    {
      "wave": 1,
      "artifacts": [
        {
          "id": "story:governance-unit-stated",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "AGENTS.md"
            }
          ]
        },
        {
          "id": "story:llm-route-bindings",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "Cargo.lock"
            },
            {
              "confidence": "inferred",
              "path": "Cargo.toml"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-b10x"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-claude"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-cli"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-codex"
            },
            {
              "confidence": "inferred",
              "path": "docs/design/model-adapter-v0.1.md"
            }
          ]
        }
      ]
    },
    {
      "wave": 2,
      "artifacts": [
        {
          "id": "story:llm-route-evidence",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "contracts"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-b10x"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-claude"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-codex"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-protocol"
            },
            {
              "confidence": "inferred",
              "path": "docs/design"
            }
          ]
        },
        {
          "id": "story:trace-ir-domain",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "CHANGELOG.md"
            },
            {
              "confidence": "inferred",
              "path": "Cargo.lock"
            },
            {
              "confidence": "inferred",
              "path": "README.md"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-aep/Cargo.toml"
            },
            {
              "confidence": "cited",
              "path": "crates/metaharness-aep/src/drive/ess_conformance.rs"
            },
            {
              "confidence": "cited",
              "path": "docs/design/metaharness-protocol-v0.1.md"
            },
            {
              "confidence": "cited",
              "path": "docs/design/runs-side-by-side-v0.1.md"
            },
            {
              "confidence": "cited",
              "path": "spec/conformance.json"
            },
            {
              "confidence": "cited",
              "path": "spec/coverage.md"
            },
            {
              "confidence": "cited",
              "path": "spec/domains/trace.yaml"
            },
            {
              "confidence": "cited",
              "path": "spec/ess-inputs.yaml"
            },
            {
              "confidence": "cited",
              "path": "spec/system.yaml"
            }
          ]
        }
      ]
    },
    {
      "wave": 3,
      "artifacts": [
        {
          "id": "story:metaharness-tools-on-tools-crates",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "CHANGELOG.md"
            },
            {
              "confidence": "inferred",
              "path": "Cargo.lock"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-b10x/Cargo.toml"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-b10x/src/seam.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-cli/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-tools/Cargo.toml"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-tools/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-tools/src/resolve.rs"
            }
          ]
        }
      ]
    },
    {
      "wave": 4,
      "artifacts": [
        {
          "id": "story:b10x-adapter-on-harness-over-llm",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "CHANGELOG.md"
            },
            {
              "confidence": "inferred",
              "path": "Cargo.lock"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-aep-eval/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-b10x/Cargo.toml"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-b10x/fixtures"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-b10x/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-b10x/src/seam.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-b10x/src/vectors.rs"
            },
            {
              "confidence": "inferred",
              "path": "docs/research"
            }
          ]
        },
        {
          "id": "story:project-reads-vendor-transcripts",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/metaharness-aep/src/drive/ess_conformance.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-cli/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness/src/viewer.rs"
            },
            {
              "confidence": "inferred",
              "path": "docs/design/runs-side-by-side-v0.1.md"
            },
            {
              "confidence": "inferred",
              "path": "spec/domains/trace.yaml"
            }
          ]
        }
      ]
    },
    {
      "wave": 5,
      "artifacts": [
        {
          "id": "story:caller-supplied-catalogue-on-every-kind",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/metaharness-b10x/src/launch.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-claude/src/launch.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-cli/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-codex/src/launch.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-protocol/src/spec.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-tools/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-tools/src/server.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness/src/run.rs"
            },
            {
              "confidence": "inferred",
              "path": "docs/design/metaharness-protocol-v0.1.md"
            }
          ]
        }
      ]
    },
    {
      "wave": 6,
      "artifacts": [
        {
          "id": "story:decision-returns-caller-produced-result",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/metaharness-protocol/src/command.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-protocol/src/event.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-tools/src/server.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness/src/run.rs"
            },
            {
              "confidence": "inferred",
              "path": "docs/design/metaharness-protocol-v0.1.md"
            },
            {
              "confidence": "inferred",
              "path": "spec/domains/boundaries.yaml"
            }
          ]
        }
      ]
    },
    {
      "wave": 7,
      "artifacts": [
        {
          "id": "story:catalogue-changes-within-a-session",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/metaharness-protocol/src/event.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-tools/src/server.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness/src/run.rs"
            },
            {
              "confidence": "inferred",
              "path": "docs/design/metaharness-protocol-v0.1.md"
            }
          ]
        }
      ]
    },
    {
      "wave": 8,
      "artifacts": [
        {
          "id": "story:continuity-across-executor-calls",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/metaharness-claude/src/launch.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-cli/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-codex/src/launch.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-protocol/src/spec.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness/src/run.rs"
            },
            {
              "confidence": "inferred",
              "path": "docs/design/metaharness-protocol-v0.1.md"
            }
          ]
        }
      ]
    },
    {
      "wave": 9,
      "artifacts": [
        {
          "id": "story:aep-drive-disposition",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "AGENTS.md"
            },
            {
              "confidence": "inferred",
              "path": "Cargo.lock"
            },
            {
              "confidence": "inferred",
              "path": "Cargo.toml"
            },
            {
              "confidence": "inferred",
              "path": "README.md"
            },
            {
              "confidence": "inferred",
              "path": "Taskfile.yml"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-aep-eval/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-aep/Cargo.toml"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-aep/src/drive.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-aep/src/drive/ess_conformance.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-cli/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "docs/ROADMAP.md"
            }
          ]
        },
        {
          "id": "story:confinement-of-vendor-kinds",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/metaharness-claude/src/launch.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-codex/src/launch.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-protocol/src/hermetic.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness/src/run.rs"
            },
            {
              "confidence": "inferred",
              "path": "docs/design/metaharness-protocol-v0.1.md"
            },
            {
              "confidence": "inferred",
              "path": "spec/domains/workspace.yaml"
            }
          ]
        }
      ]
    },
    {
      "wave": 10,
      "artifacts": [
        {
          "id": "story:call-seam-for-harness",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "AGENTS.md"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-b10x/src/launch.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-b10x/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-b10x/src/seam.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-b10x/src/vectors.rs"
            },
            {
              "confidence": "inferred",
              "path": "docs/design/metaharness-protocol-v0.1.md"
            }
          ]
        }
      ]
    }
  ],
  "collisions": [
    {
      "a": "story:aep-drive-disposition",
      "b": "story:b10x-adapter-on-harness-over-llm",
      "path": "Cargo.lock",
      "confidence": "inferred"
    },
    {
      "a": "story:aep-drive-disposition",
      "b": "story:b10x-adapter-on-harness-over-llm",
      "path": "crates/metaharness-aep-eval/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:aep-drive-disposition",
      "b": "story:call-seam-for-harness",
      "path": "AGENTS.md",
      "confidence": "inferred"
    },
    {
      "a": "story:aep-drive-disposition",
      "b": "story:caller-supplied-catalogue-on-every-kind",
      "path": "crates/metaharness-cli/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:aep-drive-disposition",
      "b": "story:continuity-across-executor-calls",
      "path": "crates/metaharness-cli/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:aep-drive-disposition",
      "b": "story:governance-unit-stated",
      "path": "AGENTS.md",
      "confidence": "inferred"
    },
    {
      "a": "story:aep-drive-disposition",
      "b": "story:llm-route-bindings",
      "path": "Cargo.lock",
      "confidence": "inferred"
    },
    {
      "a": "story:aep-drive-disposition",
      "b": "story:llm-route-bindings",
      "path": "Cargo.toml",
      "confidence": "inferred"
    },
    {
      "a": "story:aep-drive-disposition",
      "b": "story:metaharness-tools-on-tools-crates",
      "path": "Cargo.lock",
      "confidence": "inferred"
    },
    {
      "a": "story:aep-drive-disposition",
      "b": "story:metaharness-tools-on-tools-crates",
      "path": "crates/metaharness-cli/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:aep-drive-disposition",
      "b": "story:project-reads-vendor-transcripts",
      "path": "crates/metaharness-aep/src/drive/ess_conformance.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:aep-drive-disposition",
      "b": "story:project-reads-vendor-transcripts",
      "path": "crates/metaharness-cli/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:aep-drive-disposition",
      "b": "story:trace-ir-domain",
      "path": "Cargo.lock",
      "confidence": "inferred"
    },
    {
      "a": "story:aep-drive-disposition",
      "b": "story:trace-ir-domain",
      "path": "README.md",
      "confidence": "inferred"
    },
    {
      "a": "story:aep-drive-disposition",
      "b": "story:trace-ir-domain",
      "path": "crates/metaharness-aep/Cargo.toml",
      "confidence": "inferred"
    },
    {
      "a": "story:aep-drive-disposition",
      "b": "story:trace-ir-domain",
      "path": "crates/metaharness-aep/src/drive/ess_conformance.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:b10x-adapter-on-harness-over-llm",
      "b": "story:call-seam-for-harness",
      "path": "crates/metaharness-b10x/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:b10x-adapter-on-harness-over-llm",
      "b": "story:call-seam-for-harness",
      "path": "crates/metaharness-b10x/src/seam.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:b10x-adapter-on-harness-over-llm",
      "b": "story:call-seam-for-harness",
      "path": "crates/metaharness-b10x/src/vectors.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:b10x-adapter-on-harness-over-llm",
      "b": "story:llm-route-bindings",
      "path": "Cargo.lock",
      "confidence": "inferred"
    },
    {
      "a": "story:b10x-adapter-on-harness-over-llm",
      "b": "story:metaharness-tools-on-tools-crates",
      "path": "CHANGELOG.md",
      "confidence": "inferred"
    },
    {
      "a": "story:b10x-adapter-on-harness-over-llm",
      "b": "story:metaharness-tools-on-tools-crates",
      "path": "Cargo.lock",
      "confidence": "inferred"
    },
    {
      "a": "story:b10x-adapter-on-harness-over-llm",
      "b": "story:metaharness-tools-on-tools-crates",
      "path": "crates/metaharness-b10x/Cargo.toml",
      "confidence": "inferred"
    },
    {
      "a": "story:b10x-adapter-on-harness-over-llm",
      "b": "story:metaharness-tools-on-tools-crates",
      "path": "crates/metaharness-b10x/src/seam.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:b10x-adapter-on-harness-over-llm",
      "b": "story:trace-ir-domain",
      "path": "CHANGELOG.md",
      "confidence": "inferred"
    },
    {
      "a": "story:b10x-adapter-on-harness-over-llm",
      "b": "story:trace-ir-domain",
      "path": "Cargo.lock",
      "confidence": "inferred"
    },
    {
      "a": "story:call-seam-for-harness",
      "b": "story:caller-supplied-catalogue-on-every-kind",
      "path": "crates/metaharness-b10x/src/launch.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:call-seam-for-harness",
      "b": "story:caller-supplied-catalogue-on-every-kind",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:call-seam-for-harness",
      "b": "story:catalogue-changes-within-a-session",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:call-seam-for-harness",
      "b": "story:confinement-of-vendor-kinds",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:call-seam-for-harness",
      "b": "story:continuity-across-executor-calls",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:call-seam-for-harness",
      "b": "story:decision-returns-caller-produced-result",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:call-seam-for-harness",
      "b": "story:governance-unit-stated",
      "path": "AGENTS.md",
      "confidence": "inferred"
    },
    {
      "a": "story:call-seam-for-harness",
      "b": "story:metaharness-tools-on-tools-crates",
      "path": "crates/metaharness-b10x/src/seam.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:call-seam-for-harness",
      "b": "story:trace-ir-domain",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:catalogue-changes-within-a-session",
      "path": "crates/metaharness-tools/src/server.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:catalogue-changes-within-a-session",
      "path": "crates/metaharness/src/run.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:catalogue-changes-within-a-session",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:confinement-of-vendor-kinds",
      "path": "crates/metaharness-claude/src/launch.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:confinement-of-vendor-kinds",
      "path": "crates/metaharness-codex/src/launch.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:confinement-of-vendor-kinds",
      "path": "crates/metaharness/src/run.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:confinement-of-vendor-kinds",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:continuity-across-executor-calls",
      "path": "crates/metaharness-claude/src/launch.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:continuity-across-executor-calls",
      "path": "crates/metaharness-cli/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:continuity-across-executor-calls",
      "path": "crates/metaharness-codex/src/launch.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:continuity-across-executor-calls",
      "path": "crates/metaharness-protocol/src/spec.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:continuity-across-executor-calls",
      "path": "crates/metaharness/src/run.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:continuity-across-executor-calls",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:decision-returns-caller-produced-result",
      "path": "crates/metaharness-tools/src/server.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:decision-returns-caller-produced-result",
      "path": "crates/metaharness/src/run.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:decision-returns-caller-produced-result",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:metaharness-tools-on-tools-crates",
      "path": "crates/metaharness-cli/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:metaharness-tools-on-tools-crates",
      "path": "crates/metaharness-tools/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:project-reads-vendor-transcripts",
      "path": "crates/metaharness-cli/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:trace-ir-domain",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:catalogue-changes-within-a-session",
      "b": "story:confinement-of-vendor-kinds",
      "path": "crates/metaharness/src/run.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:catalogue-changes-within-a-session",
      "b": "story:confinement-of-vendor-kinds",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:catalogue-changes-within-a-session",
      "b": "story:continuity-across-executor-calls",
      "path": "crates/metaharness/src/run.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:catalogue-changes-within-a-session",
      "b": "story:continuity-across-executor-calls",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:catalogue-changes-within-a-session",
      "b": "story:decision-returns-caller-produced-result",
      "path": "crates/metaharness-protocol/src/event.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:catalogue-changes-within-a-session",
      "b": "story:decision-returns-caller-produced-result",
      "path": "crates/metaharness-tools/src/server.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:catalogue-changes-within-a-session",
      "b": "story:decision-returns-caller-produced-result",
      "path": "crates/metaharness/src/run.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:catalogue-changes-within-a-session",
      "b": "story:decision-returns-caller-produced-result",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:catalogue-changes-within-a-session",
      "b": "story:trace-ir-domain",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:confinement-of-vendor-kinds",
      "b": "story:continuity-across-executor-calls",
      "path": "crates/metaharness-claude/src/launch.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:confinement-of-vendor-kinds",
      "b": "story:continuity-across-executor-calls",
      "path": "crates/metaharness-codex/src/launch.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:confinement-of-vendor-kinds",
      "b": "story:continuity-across-executor-calls",
      "path": "crates/metaharness/src/run.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:confinement-of-vendor-kinds",
      "b": "story:continuity-across-executor-calls",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:confinement-of-vendor-kinds",
      "b": "story:decision-returns-caller-produced-result",
      "path": "crates/metaharness/src/run.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:confinement-of-vendor-kinds",
      "b": "story:decision-returns-caller-produced-result",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:confinement-of-vendor-kinds",
      "b": "story:trace-ir-domain",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:continuity-across-executor-calls",
      "b": "story:decision-returns-caller-produced-result",
      "path": "crates/metaharness/src/run.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:continuity-across-executor-calls",
      "b": "story:decision-returns-caller-produced-result",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:continuity-across-executor-calls",
      "b": "story:metaharness-tools-on-tools-crates",
      "path": "crates/metaharness-cli/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:continuity-across-executor-calls",
      "b": "story:project-reads-vendor-transcripts",
      "path": "crates/metaharness-cli/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:continuity-across-executor-calls",
      "b": "story:trace-ir-domain",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:decision-returns-caller-produced-result",
      "b": "story:trace-ir-domain",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:llm-route-bindings",
      "b": "story:llm-route-evidence",
      "path": "crates/metaharness-b10x",
      "confidence": "inferred"
    },
    {
      "a": "story:llm-route-bindings",
      "b": "story:llm-route-evidence",
      "path": "crates/metaharness-claude",
      "confidence": "inferred"
    },
    {
      "a": "story:llm-route-bindings",
      "b": "story:llm-route-evidence",
      "path": "crates/metaharness-codex",
      "confidence": "inferred"
    },
    {
      "a": "story:llm-route-bindings",
      "b": "story:metaharness-tools-on-tools-crates",
      "path": "Cargo.lock",
      "confidence": "inferred"
    },
    {
      "a": "story:llm-route-bindings",
      "b": "story:trace-ir-domain",
      "path": "Cargo.lock",
      "confidence": "inferred"
    },
    {
      "a": "story:metaharness-tools-on-tools-crates",
      "b": "story:project-reads-vendor-transcripts",
      "path": "crates/metaharness-cli/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:metaharness-tools-on-tools-crates",
      "b": "story:trace-ir-domain",
      "path": "CHANGELOG.md",
      "confidence": "inferred"
    },
    {
      "a": "story:metaharness-tools-on-tools-crates",
      "b": "story:trace-ir-domain",
      "path": "Cargo.lock",
      "confidence": "inferred"
    },
    {
      "a": "story:project-reads-vendor-transcripts",
      "b": "story:trace-ir-domain",
      "path": "crates/metaharness-aep/src/drive/ess_conformance.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:project-reads-vendor-transcripts",
      "b": "story:trace-ir-domain",
      "path": "docs/design/runs-side-by-side-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:project-reads-vendor-transcripts",
      "b": "story:trace-ir-domain",
      "path": "spec/domains/trace.yaml",
      "confidence": "inferred"
    }
  ],
  "unassessed": [
    "story:selector-metrics"
  ],
  "cycles": []
}
```

## Authorization

Approved 2026-10-20 by session `adr-changes` under the owner's standing exception 5 (work
implementing Atlas ADRs 0096–0101), verbatim: "APPROVED under exception 5, option B: the wave as
proposed plus story:governance-unit-stated as unit 2 (one more unit commit and its merge into
integration). Reason: it states ADR 0098's unit of governance at no build cost. Unit 2 and the closing
commit both touch AGENTS.md: merge unit 2 first, the closing sentence after. Covers: opening store
commit, 2 unit commits, their merges, closing store commit, push of the integration branch, PR and
merge to main through the b10x-gates bot after Gate+MSRV are green. Not a tag, release or paid run.
decision-blocker:aep-drive-future stays open for Timo (default A, keep). Send me the merge sha and PR
URL. Verified: tree on 4155db5 = origin/main, both stories present."

## Unit 2 plan

- `story:governance-unit-stated`: one paragraph in AGENTS.md § What this repository owns. Unit tree
  created from the integration head, branch `impl/governance-unit-stated`, no build. Written by the
  coordinator (AGENTS.md is a coordinator-owned shared file; the wave protocol holds implementors off
  it), verified by `grep`, no adversary pass (prose). Merged before the closing commit, which adds the
  ADR 0096 point 7 ownership sentence to the same section.

## Unit stages (kept current)

| unit | managed tree | branch | base | stage |
|---|---|---|---|---|
| integration | `wt-1c57662d33a0` | `integration/adr-0096-session-format` | `4155db5` | opening commit `d7fad09`; cheap gate green (ess validate ×2 exit 0 on 0.56.0, `cargo fmt --check` exit 0, store valid) |
| 1 `story:trace-ir-domain` | `wt-4a933645b202`, build dir `target/` inside it, scratch `metaharness-wave-adr0096/trace-ir-domain/scratch` under the operator's cache directory | `impl/trace-ir-domain` | `d7fad09` | `aep:implementor` dispatched |
| 2 `story:governance-unit-stated` | `wt-88fd3441a6bd`, no build, scratch `metaharness-wave-adr0096/governance-unit-stated/scratch` | `impl/governance-unit-stated` | `d7fad09` | coordinator-authored paragraph, unit commit `2a73b1e`, fast-forwarded into integration |

Opening commit refusal and fix: the bot's common checks refused the first attempt with 6 findings,
one per line of this page that carried the machine's home directory in a path; those lines were
rewritten as relative descriptions and the commit was accepted.

Decision recorded after the opening commit, for the closing commit: `decision-blocker:aep-drive-future`
cleared with option A (Timo's answer "a", relayed by `adr-changes`), `story:aep-drive-disposition`
rewritten to option A.

## Unit 1 progress

- `aep:implementor` returned green: scenario lane 45 → 72 (27 `metaharness.trace.*`), cargo lane 80 → 80
  (no new `#[test]`; the cases are scenarios), five gate steps each exit 0, two red runs recorded
  (count gate 72 vs 45 at `ess_conformance.rs:874`; `no production target for
  metaharness.trace.ProjectRecordedRun` at :668). Hardening: 1 planted spec defect and 2 planted
  production defects failed by scenario name and were restored; guard-negate mutation 43/43 killed.
  Build measured: `target/` 1.3G (1.1G at the base build), free disk after 55G. Agent cost: 344,281
  tokens, 104 tool uses, 1,902 s. Report: `metaharness-wave-adr0096/trace-ir-domain/implementor-report.md`
  under the operator's cache directory.
- Finding carried into the store: Done-when 6 of the story said the b10x golden record is "at harness
  0.13.3"; it is from `b10x-harness 0.9.1` (`fixtures/golden/version.txt:1`). The story body was
  corrected.
- `aep:adversary` pass 1 dispatched against the unit tree.
- `aep:adversary` pass 1 returned red: 5 findings, all `introduced` (2 warnings CONFIRMED: the
  unguarded `unk` outcome drives one of eight kinds, so a mutant reclassifying `auth.expired` as
  `opaque` passes 72/72; the `misplaced-or-miscounting` arm builds only the misplaced shape, so a
  mutant without the marker-count comparison passes 72/72; 3 notes: vacuous `>= 0` invariants,
  doc miscount of view invariants, an INFEASIBLE digest residue). It left 5 regression cases in
  `crates/metaharness-aep/tests/adversary_trace_ir_domain.rs` (one red). Recorded as
  `review-result:adr0096-trace-ir-domain-adversary-1`; finding 5 recorded `no-op`. Agent cost:
  218,600 tokens, 57 tool uses, 1,184 s. Its 1.3G mutant-tree build directory was removed by exact
  path after recording.
- Correction round 1 dispatched to the same implementor (no case has failed twice) with
  `metaharness-wave-adr0096/trace-ir-domain/correction-1.md`.
- Correction round 1 returned green: scenario lane 72 → 79 (34 `metaharness.trace.*`: 19 classify,
  5 completeness, 2 digest, 2 retained projections, 2 view-invariant scenarios carrying 8
  predicates, 4 normalize); the adversary's 5 cases pass (red before: `13 of 20 wire kinds`; green
  after); five gate steps each exit 0; planted spec defect fails 1 by name, planted production
  defects fail 11 by name, all restored; guard-negate mutation 52/52 killed, 0 stillborn.
  Deviation from the adversary's named fix on finding 1, measured: with all 19 classify guards and
  no default ESS 0.56.0 emits 19 stillborn mutants (`ESS-COMMAND-004/005`), so `auth.expired` is
  the one unguarded default and the other seven `unk` kinds are guarded; stated in the domain,
  `spec/coverage.md` and the report. The two probe suites were deleted (unreferenced); the Rust
  test file stays in the unit. Build `target/` 1.3G, free disk after 46G. Agent cost this round:
  421,252 tokens, 46 tool uses, 814 s. Findings 1–4 of pass 1 recorded `fixed`.
- `aep:adversary` pass 2 dispatched (the last of the two-pass budget).
- Filed during the wave on `adr-changes`'s relay of takt's gap G15: `story:strict-hermetic-with-embedder-scratch-cwd`
  (draft, under `epic:loom-agent-work-through-metaharness`, 12 inferred scope entries, ordering edges
  from the confinement and catalogue stories); slated as the first unit of the next wave. Rides in
  the closing store commit.
- Adversary pass 2 was cut by HTTP 429 before reporting and resumed once on the same model.
- `aep:adversary` pass 2 (the last of the budget) returned green for the tree: its 7 cases pass,
  79/79 scenarios, byte-identical regeneration; 3 findings, all `introduced`, on suite strength
  (1 warning: `opening_node.timestamp`/`timestamp_ms` exposed but read by no predicate, so a
  mutant setting both to `None` passes 79/79 and is caught only by `metaharness-cli tests/project.rs:339`;
  2 notes: `byte_stable` is self-referential while the committed `*.trace-ir.json` goldens are read
  by no scenario; `census.unk == families.unk` is 0 == 0 on the driven input). Recorded as
  `review-result:adr0096-trace-ir-domain-adversary-2`. Ledger between the passes
  (`aep plan artifact findings`): carried 0, new 3, resolved 5. Agent cost: 290,343 tokens, 29 tool
  uses, 557 s (plus 273,614 tokens, 22 tool uses, 384 s on the resumed run cut by HTTP 429). Its
  recreated 1.5G mutant build directory was removed by exact path.
- Correction round 2 dispatched to the same implementor (all three findings are new ground, none a
  case that failed twice); the coordinator verifies this correction by reading the diff and the
  gate exits, and opens no third attack.
- Correction round 2 returned green: predicates 8 → 12 (`RetainedDocument` gains `golden_stable`,
  `opening_timestamp_present`, `families.session_start == 1`, `families.stream_closed == 1`;
  `ProjectionRecorded` carries `opening_timestamp_ms` pinned per run, 1787302800000 and
  1787304000000), scenario total unchanged at 79, mutation 52/52 killed, five gate steps each
  exit 0, protocol/adapter/core/manifest/evals diffs against `d7fad09` empty. Deviation stated by
  the implementor: the adversary's named `opening_node.timestamp_ms == 1787302800000` is true for
  one run only, so the literal lives on each record's payload. Agent cost this round: 446,774
  tokens, 15 tool uses, 273 s.
- Coordinator verification of the round-2 correction (no third attack): read the hunks;
  `golden_stable` is `fresh == golden` with `golden` read from `evals/aep/runs/<run>.trace-ir.json`
  (`ess_conformance.rs:1084,1109`), `opening_timestamp_present` is `opening["timestamp"].is_string()`
  (:1107), `opening_timestamp_ms` is `document.events[0].timestamp_ms` (:1037); the 17 non-comment
  lines removed from the target since the base are the old matcher bodies, the old `>=`/`>` view
  operators and the `45` count gate, all replaced by the matchers the adversary audited in both
  passes. No assertion dropped, no re-pinned test relaxed. Pass-2 findings 1–3 recorded `fixed`.

## Integration

| step | result |
|---|---|
| baseline measurement on `2a73b1e` (before unit 1) | `cargo test -p metaharness-aep ess_generated_scenarios_drive_production_targets` exit 0; conformance total 45, passed 45; `target/` 1.1G |
| unit 1 commit | `4a4c905` on `impl/trace-ir-domain`, bot-authored, 0 home-path lines |
| unit 1 merge | `fb0e422`, bot-authored merge commit, parents `2a73b1e` and `4a4c905`; the bot wrapper accepted `merge --no-ff --no-edit` (it merges without committing, then commits as the bot) |
| treatment measurement and whole gate on `fb0e422` | see the gate table below |

## Whole gate on `fb0e422`, one step at a time (each step's own exit status)

| step | command | exit |
|---|---|---|
| 1 | `ess specify validate --path spec --strict-requires` (ESS 0.56.0) | 0 |
| 2 | `ess specify validate --path spec/monetary --strict-requires` | 0 |
| 3 | `cargo fmt --check` | 0 |
| 4 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| 5 | `cargo test --workspace` | 0: 59 test binaries, 803 passed, 0 failed, 31 ignored (the ignored ones are the live tests behind `METAHARNESS_LIVE`) |
| 6 | `cargo run --locked --quiet -p metaharness-docs -- check` | 0 |

Conformance lane on `fb0e422`: total 79, passed 79, failed 0, skipped 0; official `ess-conformance-report/2` `conformance_status: passed`, unsupported 0. Treatment vs baseline: 79 vs 45 scenarios with the same command, `VERIFIED`. Build directory of the integration tree after the gate: 3.4G; free disk 59G. Logs: `metaharness-wave-adr0096/closing/gate-*.log` under the operator's cache directory.

Closing commit: evidence (verification, the ESS report, `test_result` for both stories) recorded, both stories moved to `implemented`, the scope section of `story:trace-ir-domain` rewritten from the implementor's confirmation table, this page, the cleared blocker, the rewritten disposition story, the G15 story and the AGENTS.md ownership paragraph.

- Deviation at close, fixed: two `test_result` records were dropped while still uncommitted and re-recorded, because their `--ref` carried the machine's home path and the bot's common checks refuse that; 0 such lines are staged.

## Merge to main

PR https://github.com/beyond10x/metaharness/pull/32 merged 2026-10-20T04:46:44Z by app/b10x-bot; main moved 4155db5 → 205125c. The bot's `gh pr merge` was refused ("the base branch policy prohibits the merge"): ruleset 22044982 `b10x-bot-branch-identity` (no bypass actors) requires the committer email to be the bot's, and a GitHub-made merge commit carries `noreply@github.com` (PR #31's merge commit 4155db5 does, so the rule postdates it). main was fast-forwarded to the checked head 205125c through `b10x-gates bot -- push`; every commit is bot author and committer and CI passed on that exact head, so the identity rule is met, not bypassed. Accepted by `adr-changes` the same day.

Cleanup: the three managed trees were finished with `--discard-cache --archive` and removed by exact id (archives 8K each, nothing unique); leases released; unit branches deleted; 108M of logs kept under the operator's cache directory `metaharness-wave-adr0096`; the primary checkout was left at 4155db5.
