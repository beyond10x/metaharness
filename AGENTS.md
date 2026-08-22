# metaharness — working agreements

## What this repository is

The unified outward-facing interface to agent harnesses (Claude Code, Codex, …): emit events,
receive steering commands, run hermetically, control the tool surface per call. Library first
(`metaharness` crate, builder API), binary second (`metaharness-cli`, verb `metaharness run
<kind>`). Everything harness-specific lives in `metaharness-<kind>` adapter crates; the protocol
crate (`metaharness-protocol`) depends on no adapter.

## Consumers that keep this honest

- `~/projects/engineering-protocols` — its eval harness and reference driver replace direct
  `claude …` invocations with `metaharness run claude --hermetic …`; its workflows (planning as
  request → classify → route-to-entities) must be drivable deterministically through this
  interface. That repo's `integrations/` carries the harness-specific residue that migrates here.
- `~/former organization/former organization/runtime/agent` — prior art for adapter classes, approvals, steering;
  not a dependency, a reference.

## What lives in `evals/`

The paid and recorded evaluation machinery migrated out of `engineering-protocols` (its
`epic:metaharness-migration`): eval scripts, recorded transcripts, contracts and result records.
Nothing under `evals/` runs in `task check` — a paid run is never part of a gate. The subject
checkout is `EP_REPO` (default `~/projects/engineering-protocols`).

## Rules

- Rust CLIs use clap (derive). Hand-rolled argv parsing is banned.
- Task runner is `Taskfile.yml` (go-task). `task check` = fmt + clippy -D warnings + test; green
  before any push.
- Never write work files to `/tmp`; use `~/.cache/claude-tmp` or the repo.
- Commit style: semantic type(scope), body with bullets, `git commit -F` (never `-m` with
  backticks). Author email is the GitHub noreply address.
- Every adapter claim about a vendor binary is pinned to a version and verified against it, or
  labelled unverified. A vendor surface we have not driven is documented as such.
- Absence of evidence is not a property: hermeticity, tool restriction and denial behaviour are
  asserted from the run's own record, never from configuration.

## Design decisions land in docs/design/, reviewed before build

The protocol wire (events out, commands in), the control seam (per-call tool decisions), the
hermetic contract and the adapter obligations are `docs/design/metaharness-protocol-v0.1.md`.
Proposed until a review accepts them; the review's corrections are recorded in the document.
