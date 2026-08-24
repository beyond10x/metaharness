# metaharness — working agreements

## What this repository is

The unified outward-facing interface to agent harnesses (Claude Code, Codex, …): emit events,
receive steering commands, run hermetically, control the tool surface per call. Library first
(`metaharness` crate, builder API), binary second (`metaharness-cli`, verb `metaharness run
<kind>`). Everything harness-specific lives in `metaharness-<kind>` adapter crates; the protocol
crate (`metaharness-protocol`) depends on no adapter.

## Consumers that keep this honest

- `~/beyond10x/engineering-protocols` — its eval harness and reference driver replace direct
  `claude …` invocations with `metaharness run claude --hermetic …`; its workflows (planning as
  request → classify → route-to-entities) must be drivable deterministically through this
  interface. That repo's `integrations/` carries the harness-specific residue that migrates here.
- `~/former organization/former organization/runtime/agent` — prior art for adapter classes, approvals, steering;
  not a dependency, a reference.

## What lives in `evals/`

The paid and recorded evaluation machinery migrated out of `engineering-protocols` (its
`epic:metaharness-migration`): eval scripts, recorded transcripts, contracts and result records.
Nothing under `evals/` runs in `task check` — a paid run is never part of a gate. The subject
checkout is `EP_REPO` (default `~/beyond10x/engineering-protocols`).

## What lives in `website/`

The **public** documentation site (Docusaurus, published to GitHub Pages at
`https://beyond10x.github.io/metaharness/`). Written for a reader who does not have this
checkout: what metaharness is, how to run one, the wire, the contracts, the adapters.

The split with `docs/` is deliberate and one-way:

- `docs/design/` and `docs/research/` carry the **reasoning** — open questions, the amendment
  record, per-claim evidence labels. They are not published.
- `website/docs/` states the **conclusion** and cites where the reasoning lives.
- Where the two disagree, the design document is right and the site is stale.

`task docs` runs it, `task docs:build` builds it, and a broken link fails the build
(`onBrokenLinks: 'throw'`). Nothing under `website/` runs in `task check`; CI builds it on every
PR that touches it (`.github/workflows/docs.yml`). The built site is never committed — publishing
goes through the Pages deployment API, not a `gh-pages` branch, so `build/` never becomes history.

A claim on the site obeys the same rule as a claim in the code: pinned to a version and verified
against it, or labelled unverified.

## Rules

- Rust CLIs use clap (derive). Hand-rolled argv parsing is banned.
- Task runner is `Taskfile.yml` (go-task). `task check` = fmt + clippy -D warnings + test; green
  before any push.
- Releases: cut `CHANGELOG.md` under a version heading. The tag is the bare version — `0.1.0`,
  the version and nothing else — annotated, at a fully gated `main` commit.
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
