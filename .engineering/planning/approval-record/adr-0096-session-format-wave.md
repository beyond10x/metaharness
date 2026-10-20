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
revision: 3
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
