---
format: aep.planning-md/3
id: approval-record:adr-0096-wave-2
kind: approval-record
status: draft
title: 'Wave 2 of 2026-10-20: strict hermetic over an embedder''s scratch cwd (takt G15), project --from, release 0.10.0'
owner: metaharness
tags:
- wave
refs:
- provider: atlas
  reference: adr-0098
- provider: takt
  reference: G15
relations:
- decides: story:strict-hermetic-with-embedder-scratch-cwd
- decides: story:project-reads-vendor-transcripts
- decides: release-plan:release-0-10-0
revision: 2
---
## Status

Proposed 2026-10-20 by session `metaharness` on `adr-changes`'s request ("propose the G15 wave,
plus what fits without collision, 3–5 stories; a release that takt can pin belongs in the proposal").
Approval pending from `adr-changes` under the owner's standing exception for ADR 0096–0101 work.
Skill: aep:implementing 0.23.2, wave mode. Integration tree `wt-4d5f44e5ec8f`, branch
`integration/adr-0096-wave-2`, base `205125c` (main after the wave of 2026-10-20).

## Candidates and selection

Ready drafts (status `draft`, `blocked_by` empty, every `depends_on` terminal), read from the store
on 2026-10-20:

| story | ready | taken | why |
|---|---|---|---|
| `story:strict-hermetic-with-embedder-scratch-cwd` (takt G15) | yes | unit 1 | the wave's purpose; takt S2 waits on it |
| `story:project-reads-vendor-transcripts` | yes (`story:trace-ir-domain` implemented) | unit 2, sequential after unit 1 | closes `epic:session-format`'s Done-when; shares `crates/metaharness-cli/src/lib.rs`, `crates/metaharness-aep/src/drive/ess_conformance.rs` and `spec/conformance.json` with unit 1 (both add `execute` arms, regenerate the suite and move the count gate), so it runs on the integration branch after unit 1 merges, not beside it |
| `story:selector-metrics` | nominally (no edges, no blocker) | no | its inputs are Loom `SelectionRecord`s and session records from beyond10x/loom `story:selection-telemetry` (`loom:taskboard:L-009`); no fixture exists in this tree; unassessed scope; not implementable tonight |
| everything else | no | no | the six ADR 0098 gap stories and `story:aep-drive-disposition` depend on `story:metaharness-tools-on-tools-crates` (blocked by `dependency-blocker:tools-first-release`) or on each other; the two pin stories and the two llm stories are blocked by dependency-blockers |

N is two, sequential, and the request's 3–5 is not reachable from this store: every other draft is
blocked or depends on an unfinished story. No story was padded in.

## Release

`release-plan:release-0-10-0`, drafted with this proposal: after the wave closes on `main`, the
release commit (changelog cut, `Cargo.toml` 0.9.3 → 0.10.0, `Cargo.lock`, README status line) on a
release branch, gate, fast-forward to `main` through the bot, annotated tag `0.10.0`, the Release
workflow's four archives verified and the two Linux executables scanned, publication through
`b10x-gates api` / `gh -- release upload` with the trusted policy, ownership verified. takt pins
`0.10.0`.

## Commits this approval would authorise

1. The opening store commit on `integration/adr-0096-wave-2`: this page, the two stories moved to
   `active`, their scope from the scopers, `release-plan:release-0-10-0` (draft), the wave-1 page's
   merge-to-main record.
2. One unit commit per unit (2), each merged into the integration branch after its adversary pass.
3. The closing store commit.
4. Push of the integration branch, a pull request to `main`, and the merge as a fast-forward push of
   the checked head through the bot after CI is green.
5. The release 0.10.0 as `release-plan:release-0-10-0` describes: the release commit, its merge to
   `main`, the annotated tag and its push, the Release workflow, artifact verification and
   publication through the bot.

Not authorised: a paid run, a vendor pin move, any registry publishing, the next wave.

## Pre-flight

- free disk on `/`: measured at proposal time (below); floor 25G with the watch; one build at a time.
- builds measured in the previous wave: unit tree `target/` 1.3G, integration tree after the whole
  gate 3.4G; whole gate wall time about 25 minutes on this machine with sccache.
- ESS 0.56.0 release binary on `PATH` for every gate; the spec pin is unchanged.
- sub-agents: `aep:story-scoper` (2, run before this proposal), `aep:implementor` and
  `aep:adversary` per unit, sequential; the critic panel is not run again (G15 is one story added to
  an epic whose set the panel judged twice; project-reads was in that set).

## Pre-flight, measured 2026-10-20T05:02Z

- free disk on `/`: 53G (floor 25G, watch)
- primary checkout: branch main at 4155db5c, porcelain lines 0; `git worktree list`: 2 entries (primary and this tree)
- scopes: unit 1 19 entries, 13 cited; unit 2 13 entries, 8 cited
- both scopers reported that Bash was refused in their threads by the worktree isolation hook, so their claims rest on the files they read, not on a tree-wide grep

## `aep plan artifact waves --kind story --status draft --format json`, verbatim

```json
{
  "waves": [
    {
      "wave": 1,
      "artifacts": [
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
        },
        {
          "id": "story:project-reads-vendor-transcripts",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "CHANGELOG.md"
            },
            {
              "confidence": "inferred",
              "path": "README.md"
            },
            {
              "confidence": "cited",
              "path": "crates/metaharness-aep/src/drive/ess_conformance.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/metaharness-cli/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/metaharness-cli/tests/project.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-protocol/src/trace_ir.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness/src/refusal.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/metaharness/src/viewer.rs"
            },
            {
              "confidence": "inferred",
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
              "confidence": "inferred",
              "path": "spec/coverage.md"
            },
            {
              "confidence": "cited",
              "path": "spec/domains/trace.yaml"
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
      "wave": 3,
      "artifacts": [
        {
          "id": "story:strict-hermetic-with-embedder-scratch-cwd",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "CHANGELOG.md"
            },
            {
              "confidence": "cited",
              "path": "crates/metaharness-aep/src/drive/ess_conformance.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-claude/fixtures/c1"
            },
            {
              "confidence": "cited",
              "path": "crates/metaharness-claude/src/launch.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/metaharness-claude/src/vectors.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-codex/fixtures/c1"
            },
            {
              "confidence": "cited",
              "path": "crates/metaharness-codex/src/launch.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/metaharness-codex/src/vectors.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/metaharness-protocol/src/hermetic.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness-protocol/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/metaharness-protocol/src/spec.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/metaharness/src/audit.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/metaharness/src/builder.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/metaharness/src/scripted.rs"
            },
            {
              "confidence": "cited",
              "path": "docs/design/metaharness-protocol-v0.1.md"
            },
            {
              "confidence": "cited",
              "path": "spec/conformance.json"
            },
            {
              "confidence": "cited",
              "path": "spec/domains/workspace.yaml"
            },
            {
              "confidence": "inferred",
              "path": "website/docs/hermetic.md"
            },
            {
              "confidence": "cited",
              "path": "website/docs/reference/cli.md"
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
      "wave": 5,
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
      "wave": 6,
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
      "wave": 7,
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
      "wave": 8,
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
      "wave": 9,
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
      "path": "README.md",
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
      "b": "story:strict-hermetic-with-embedder-scratch-cwd",
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
      "b": "story:project-reads-vendor-transcripts",
      "path": "CHANGELOG.md",
      "confidence": "inferred"
    },
    {
      "a": "story:b10x-adapter-on-harness-over-llm",
      "b": "story:strict-hermetic-with-embedder-scratch-cwd",
      "path": "CHANGELOG.md",
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
      "b": "story:metaharness-tools-on-tools-crates",
      "path": "crates/metaharness-b10x/src/seam.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:call-seam-for-harness",
      "b": "story:project-reads-vendor-transcripts",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:call-seam-for-harness",
      "b": "story:strict-hermetic-with-embedder-scratch-cwd",
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
      "b": "story:project-reads-vendor-transcripts",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:strict-hermetic-with-embedder-scratch-cwd",
      "path": "crates/metaharness-claude/src/launch.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:strict-hermetic-with-embedder-scratch-cwd",
      "path": "crates/metaharness-codex/src/launch.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:strict-hermetic-with-embedder-scratch-cwd",
      "path": "crates/metaharness-protocol/src/spec.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:caller-supplied-catalogue-on-every-kind",
      "b": "story:strict-hermetic-with-embedder-scratch-cwd",
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
      "b": "story:project-reads-vendor-transcripts",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:catalogue-changes-within-a-session",
      "b": "story:strict-hermetic-with-embedder-scratch-cwd",
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
      "b": "story:project-reads-vendor-transcripts",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:confinement-of-vendor-kinds",
      "b": "story:strict-hermetic-with-embedder-scratch-cwd",
      "path": "crates/metaharness-claude/src/launch.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:confinement-of-vendor-kinds",
      "b": "story:strict-hermetic-with-embedder-scratch-cwd",
      "path": "crates/metaharness-codex/src/launch.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:confinement-of-vendor-kinds",
      "b": "story:strict-hermetic-with-embedder-scratch-cwd",
      "path": "crates/metaharness-protocol/src/hermetic.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:confinement-of-vendor-kinds",
      "b": "story:strict-hermetic-with-embedder-scratch-cwd",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:confinement-of-vendor-kinds",
      "b": "story:strict-hermetic-with-embedder-scratch-cwd",
      "path": "spec/domains/workspace.yaml",
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
      "b": "story:project-reads-vendor-transcripts",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:continuity-across-executor-calls",
      "b": "story:strict-hermetic-with-embedder-scratch-cwd",
      "path": "crates/metaharness-claude/src/launch.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:continuity-across-executor-calls",
      "b": "story:strict-hermetic-with-embedder-scratch-cwd",
      "path": "crates/metaharness-codex/src/launch.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:continuity-across-executor-calls",
      "b": "story:strict-hermetic-with-embedder-scratch-cwd",
      "path": "crates/metaharness-protocol/src/spec.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:continuity-across-executor-calls",
      "b": "story:strict-hermetic-with-embedder-scratch-cwd",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:decision-returns-caller-produced-result",
      "b": "story:project-reads-vendor-transcripts",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:decision-returns-caller-produced-result",
      "b": "story:strict-hermetic-with-embedder-scratch-cwd",
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
      "a": "story:metaharness-tools-on-tools-crates",
      "b": "story:project-reads-vendor-transcripts",
      "path": "CHANGELOG.md",
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
      "b": "story:strict-hermetic-with-embedder-scratch-cwd",
      "path": "CHANGELOG.md",
      "confidence": "inferred"
    },
    {
      "a": "story:project-reads-vendor-transcripts",
      "b": "story:strict-hermetic-with-embedder-scratch-cwd",
      "path": "CHANGELOG.md",
      "confidence": "inferred"
    },
    {
      "a": "story:project-reads-vendor-transcripts",
      "b": "story:strict-hermetic-with-embedder-scratch-cwd",
      "path": "crates/metaharness-aep/src/drive/ess_conformance.rs",
      "confidence": "cited"
    },
    {
      "a": "story:project-reads-vendor-transcripts",
      "b": "story:strict-hermetic-with-embedder-scratch-cwd",
      "path": "docs/design/metaharness-protocol-v0.1.md",
      "confidence": "inferred"
    },
    {
      "a": "story:project-reads-vendor-transcripts",
      "b": "story:strict-hermetic-with-embedder-scratch-cwd",
      "path": "spec/conformance.json",
      "confidence": "cited"
    }
  ],
  "unassessed": [
    "story:selector-metrics"
  ],
  "cycles": []
}
```

## Authorization

Approved 2026-10-20 by session `adr-changes` under the owner's standing exception 5, verbatim:
"APPROVED under exception 5: A, wave 2 as proposed (2 sequential units) and release 0.10.0 per
release-plan:release-0-10-0 (bot tag, GitHub Release; no registry). Covers exactly the list in your
message. Reason for A over B: takt needs 0.10.0 only at its wave 22 (wave 21 is a takt metric fix,
then S1), so unit 2 costs takt no time and closes epic:session-format. Send me the merge sha and the
tag sha."

## Unit stages (kept current)

| unit | managed tree | branch | base | stage |
|---|---|---|---|---|
| integration | `wt-4d5f44e5ec8f` | `integration/adr-0096-wave-2` | `205125c` | opening commit pending |
| 1 `story:strict-hermetic-with-embedder-scratch-cwd` | created after the opening commit | `impl/strict-hermetic-with-embedder-scratch-cwd` | the opening commit | pending |
| 2 `story:project-reads-vendor-transcripts` | created after unit 1 merges | `impl/project-reads-vendor-transcripts` | unit 1's merge | pending |
