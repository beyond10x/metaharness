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
revision: 21
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
| integration | `wt-4d5f44e5ec8f` | `integration/adr-0096-wave-2` | `205125c` | opening commit `972a580`; cheap gate green (ess ×2 exit 0, fmt exit 0, store valid) |
| 1 `story:strict-hermetic-with-embedder-scratch-cwd` | `wt-5168648c7324` (finished with `--discard-cache --archive` and removed after the merge; archive kept), scratch `metaharness-wave-adr0096/w2-strict-cwd/scratch` | `impl/strict-hermetic-with-embedder-scratch-cwd` (deleted after the merge) | `972a580` | unit commit `7dfc119e`, bot merge `83f14786` (parents `972a580a`, `7dfc119e`) |
| 2 `story:project-reads-vendor-transcripts` | `wt-62bdbc037f65`, build dir `target/` inside it, scratch `metaharness-wave-adr0096/w2-project-from/scratch` | `impl/project-reads-vendor-transcripts` | `83f14786` | `aep:implementor` dispatched |

## Unit 1 progress

- `aep:implementor` returned red on two cases only: the committed contract-result goldens
  `crates/metaharness/fixtures/golden/contract-result-{claude,codex}.json` move `checked` 27 → 28
  and 17 → 18 because the story adds one C1 vector per kind; the brief had put every committed
  fixture outside the implementor's reach, so it left a two-line patch. Coordinator decision: that
  is the deliberate count movement the adapter contract expects (`docs/ROADMAP.md` § 1); the patch
  was applied in the unit tree and the `contract_golden` lane re-run: 5 passed, 0 failed, 1 ignored,
  exit 0. The unit is green.
- Numbers: five-package lane 634 → 647 cases; conformance 79 → 83 (`metaharness.workspace.
  DeclareScratchWorkingDirectory`, 4 scenarios); mutation 55/55 killed; planted spec and production
  defects fail by scenario name and were restored; ess ×2, fmt, clippy, cli lane and docs check each
  exit 0; `git diff 972a580 --stat -- 'crates/*/fixtures' evals` empty before the goldens patch.
- a27 written first: `--scratch-cwd`, `HermeticAttestation.declared` (H7 only, serde-optional),
  H11 always measured, the floor's H7 arm reads the attestation. Production correction found and
  made: the base's H7 arm printed `ok` over a `--cwd` directory metaharness did not create
  (`audit.rs:620-624`); now a6 `unavailable` reads `gap`, exit code unchanged.
- Four one-line test edits outside the brief's list were applied by the implementor and reported
  (struct literals in `protocol/tests/wire.rs`, `metaharness/tests/audit.rs`, `tests/run_loop.rs`;
  vector-count pin in `claude/tests/adapter.rs`). `website/docs/reference/library.md` row left as a
  patch, not applied.
- Build measured: unit `target/` 3.2G; free disk after 51G. Agent cost: 417,055 tokens, 256 tool
  uses, 2,707 s. Report: `metaharness-wave-adr0096/w2-strict-cwd/implementor-report.md`.
- `aep:adversary` pass 1 dispatched.

## Rule received during the wave

`adr-changes`, 2026-10-20: "when a branch protection, ruleset or gates policy refuses a merge or push, stop and report to me; do not try another endpoint or route. Your wave-1 fast-forward of bot-made commits is the model." Applied here: the merge to `main` is the fast-forward push of the checked head through the bot after CI is green; any refusal from a ruleset, protection or the gates policy stops the wave at that point and is reported, with no second route tried.

- Budget hold received from `adr-changes` (weekly headroom 6–18% on the usable accounts): finish wave 2 to its close and release 0.10.0 as approved, then open no wave 3 until Timo is back. Adversary pass 1 on unit 1 was cut once by HTTP 429 and resumed on the same model.

- `aep:adversary` pass 1 on unit 1 (cut once by HTTP 429, resumed): NEEDS-CHANGE, 2 red cases in `crates/metaharness/tests/adversary_scratch_cwd.rs`; findings: (1) blocker, introduced: `start_b10x`'s a6 branch never attests H7, so the floor still prints the old `ok` wording on the b10x arm; (2) warning, pre-existing: the b10x opening record echoes the planned cwd (`seam.rs:163`), filed as `story:b10x-record-carries-the-loops-cwd`, the adversary's case 2 to be rewritten to today's state naming it; (3) note, INFEASIBLE: the non-H7 declaration arm is unpinned. Recorded as `review-result:adr0096-w2-strict-cwd-adversary-1`; findings 2 and 3 recorded `no-op`. Agent cost: 185,099 tokens, 14 tool uses, 240 s. Correction round 1 dispatched to the same implementor.

- Correction round 1 on unit 1 returned green: finding 1 fixed (`start_b10x`'s a6 branch attests H7 unavailable; the floor reads `gap`), finding 2 stated in a27, `hermetic.md` and the changelog with the adversary's case 2 rewritten to today's state naming `story:b10x-record-carries-the-loops-cwd`, finding 3 pinned by a unit test in `audit.rs`; five-package lane 647 → 650 cases, seven gate steps each exit 0, conformance 83/83, spec unchanged (mutation 55/55 from round 0 stands); `git diff 972a580 --stat -- crates/*/fixtures evals` shows only the two contract-result goldens (the implementor corrected its round-0 check, which had quoted the glob and matched nothing). Agent cost this round: 446,159 tokens, 24 tool uses, 387 s. Free disk 43G. Finding 1 recorded `fixed`; adversary pass 2 dispatched.

- `aep:adversary` pass 2 on unit 1 (the last): NEEDS-CHANGE on the test file only: finding A (introduced, warning) the rewritten case 2 could pass silently when `story:b10x-record-carries-the-loops-cwd` lands; finding B (pre-existing, warning) the b10x seam ignores the loop line's cwd, the story already filed; one red case asserting the story's future behaviour. Ledger between the passes: carried 0, new 2, resolved 3. Recorded as `review-result:adr0096-w2-strict-cwd-adversary-2`; B recorded `no-op`. Agent cost: 225,892 tokens, 18 tool uses, 376 s. Correction round 2 (test file only: delete case 2, rewrite case 3 to today's state naming the story so it goes red under either landing shape) dispatched; the coordinator verifies it from the diff and the two exit codes, no third attack.

- Correction round 2 returned green: test file only (`adversary_scratch_cwd.rs` 210 → 186 lines; the weaker case deleted, the red case rewritten to today's state naming `story:b10x-record-carries-the-loops-cwd` and goes red under either landing shape); `--test adversary_scratch_cwd` 2 passed exit 0, clippy exit 0. Coordinator verification: read the diff in the report, no assertion relaxed; finding A recorded `fixed`. Claim verified: the strict declared-copy run exits 0 with H7 `declared` and H11 measured where the base exits 1 (`verification` evidence recorded). Agent cost this round: 464,720 tokens, 11 tool uses, 133 s. Unit 1 committed and merged below.

- Hold lifted by `adr-changes` for the ADR path (2026-10-20): after 0.10.0, propose wave 3 with `story:metaharness-tools-adopts-tool-contract`, the b10x adapter onto harness 0.14.1 when tagged, plus what fits without collision, with a release if takt or loom pins it. Disk reported at 22G free at that moment; measured 40G free a minute later; the watch refuses any build under 25G.

- Unit 2 `aep:implementor` returned green: `project --from` over the three golden records, suite 83 → 87 (`ProjectVendorRecord`: three vendor outcomes and a default refusal), 58/58 mutants, planted spec and production defects fail by name, six red cases turned green, fixture bytes unchanged; amendment a28. Its gate's fmt step was red only on unit 1's merged test file, fixed on the integration branch as formatting-only bot commit `68b1d85`. Two coordinator patches left: the website `cli.md` rows (applied at close if the docs check stays green) and the fmt patch (applied). Agent cost: 359,890 tokens, 132 tool uses, 2,094 s. `aep:adversary` pass 1 dispatched.
- `adr-changes` (2026-10-20): tag 0.10.0 as soon as wave 2 is green without waiting for a further relay; then propose wave 3 (tool-contract adoption; b10x adapter onto harness 0.14.1 or 0.15.0 if out; tool-contract only while the licence is with Timo).

- Store updates during unit 2, for the closing commit: tools 0.1.0 recorded and `dependency-blocker:tools-first-release` cleared; `decision-blocker:tools-implementation-licence` filed and then cleared with option A (Timo, about 10:10, relayed: the crates moved from harness become Apache-2.0 in tools 0.2.0); `dependency-blocker:tools-0-2-0-release` holds both tools stories until that tag (at 0.1.0 the contract documents are embedded only by the implementation crates); harness 0.14.0 recorded and `dependency-blocker:harness-llm-release` cleared, 0.14.1 (tag `9a3735d1` on `f109612e`) named as the pin target; `story:b10x-record-carries-the-loops-cwd` filed from unit 1's adversary.

- Monitor renamed by Timo: the session that was `takt-monitor`, then `adr-changes`, is `b10x-chef-orchestrator` (same session, ref 28b114; CLAUDE.md exception 5 names it). Reports, proposals and the tag sha go there from here on; approvals already given stand.

- `aep:adversary` pass 1 on unit 2: red (2 of its 4 cases): F1 `vendor_transcript.path` carries the argv spelling (coordinator decision: file name only), F2 `source_line` under `--from` indexes a stream that does not exist (decision: the record line the reader carries, a28 states it); F3/F4 two mutants (first-emission rule, b10x replay inventing version/cwd) survive the shipped suite and are pinned by its cases 1–2; F5 opaque refusal detail prints the reader's convention; F6 `--html` header label. All `introduced`. Recorded as `review-result:adr0096-w2-project-from-adversary-1`. Agent cost: 188,023 tokens, 61 tool uses, 1,209 s (plus 175,151 tokens on the interim run). Correction round 1 dispatched to the same implementor.

- Timo via `b10x-chef-orchestrator`: "do not reduce work based on quota, we can add more anytime"; no budget holds from now on. After wave 3, wave 4 is proposed from the six ADR 0098 gap stories in their `depends_on` order rather than parked.

- Correction round 1 on unit 2 returned green: F1 `vendor_transcript.path` is the file name only; F2 `source_line` on a replay is the record line the reader read the event from (events owed at `finish` cite the record's last line, a reasoned deviation from the literal fallback, accepted); F3/F4 pinned by the adversary's cases 1–2; F5 the opaque detail no longer prints the reader's convention; F6 the `--html` header is labelled by `metaharness.source` for a vendor record and keeps `stream` on the wire path because `evals/aep/runs/side-by-side.html` pins it (accepted). Cases 434 → 439, the adversary's 4 cases and the unit's 22 green, clippy 0, docs check 0, conformance 87/87 unchanged, fixture bytes unchanged; the only fmt diff is unit 1's file already fixed on the integration branch. Agent cost this round: 417,196 tokens, 35 tool uses, 889 s. Six outcomes recorded `fixed`. Disk 29G free at the end of the run; the next builds watch the 25G floor.

- `aep:adversary` pass 2 on unit 2 (the last): all four pass-1 cases green; 3 notes, all `introduced`: F7 an unknown harness `kind` is not named in the refusal (one red case), F8 the vendor header label pinned by its own green case, F9 same-named records render identical `--html` headers; ledger carried 0, new 3, resolved 6. Recorded as `review-result:adr0096-w2-project-from-adversary-2`; F8 recorded `no-op`. Agent cost: 239,958 tokens, 21 tool uses, 431 s (plus 232,688 tokens on the run interrupted by the disk hold). Correction round 2 dispatched (F7 print the subtype; F9 digest suffix on colliding headers, committed page byte-identical; a debug_assert on `project_from`); the coordinator verifies it from the diff and exits, no third attack. Disk hold of 2026-10-20 10:0x–10:20: / fell to 17–19G, builds refused by the watch, `b10x-chef-orchestrator` had other sessions free space (34G at 10:20), then the pass resumed.

- Correction round 2 on unit 2 returned green: F7 the refusal names the line's own kind (`(None, Some(kind))` arm), F9 two same-named columns carry an 8-hex digest suffix while differing names keep today's headers (the committed page byte-identical, asserted by the non-ignored fixture test), the `project_from` length assertion as its own fn; adversary lane 6/6 and `project.rs` 23/23 green, protocol 75, clippy 0, docs check 0; spec untouched, 87 stands. Coordinator verification from the diff: the nine removed production lines are the refactor seams, no assertion removed. F7 and F9 recorded `fixed`; claim verified (baseline: `--from` unknown, exit 101; treatment: 29 green cases, census equality). Agent cost this round: 443,148 tokens, 20 tool uses, 354 s.

- Whole gate on `integration/adr-0096-wave-2` at `7076ac9b` plus the coordinator's website `cli.md` patch (the `project` rows stale since a15), one step at a time: `ess specify validate` over `spec` and `spec/monetary` exit 0, `cargo fmt --check` exit 0, clippy `--workspace --all-targets -D warnings` exit 0, `cargo test --workspace` exit 0 (61 test binaries, 836 passed, 0 failed, 31 ignored), `metaharness-docs check` exit 0; conformance 87/87 on ESS 0.56.0 (official report: passed, 0 failed, 0 skipped). Unit 2's tree finished and archived, its branch deleted. Both stories moved to `implemented` on that evidence in the closing commit.
