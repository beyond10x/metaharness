# AGENTS.md — metaharness

The contract for changing **this** repository. Org-wide rules — the naming convention, the
former-brand rule (atlas ADR 0001) and its four exemption categories, and the rule that renaming
anything another repo verifies is a coordinated migration with an ADR — live in
[`atlas/AGENTS.md`](https://github.com/beyond10x/atlas) and are not restated here.

`README.md` says what metaharness is and how to run one. This file says what must not break.

## Serves

The objectives of the collection this repository moves, by id from `atlas/ROADMAP.md` — the only
cross-repository roadmap, and the page that says what each id means and which evidence closes it:

- **O3 — any harness, observed and compared.** One interface to many harnesses, one record vocabulary, and the eval that scores two arms on the same work under the same governor.
- **O6 — self-improvement, built into all of it.** That eval is the measurement a self-improvement loop reads; a column that cannot fail measures nothing.

A change here that moves none of these is a question for the operator, not a task.
`atlas/scripts/check-map.sh` fails a repository whose `AGENTS.md` names no objective.

## What this repository owns

One interface to many agent harnesses: emit events, receive steering commands, run hermetically,
decide the tool surface per call. Library first (`metaharness`, builder API), binary second
(`metaharness-cli`, `metaharness run <kind>`).

## Invariants

Each is a claim that can be checked. Breaking one is a design change, not a refactor.

1. **`metaharness-protocol` depends on no adapter crate.** Its `[dependencies]` are `clap`
   (optional), `serde`, `serde_json`, `sha2` and nothing else. An adapter reaching into the protocol
   crate's dependency list inverts the seam the workspace exists to hold.
2. **Everything harness-specific lives in `metaharness-<kind>`.** A vendor field name, a vendor
   binary's flag, a vendor's transcript shape: in the adapter crate, nowhere else.
3. **Absence of evidence is not a property.** Hermeticity, tool restriction and denial behaviour are
   asserted from the run's own record — never from configuration, a directory layout or a flag that
   was passed. A run that pinned nothing is *nobody found out*, not *nothing moved*; that confusion
   made `--hermetic strict` unpassable once and is amendment a4.
4. **Every adapter claim about a vendor binary is pinned to a version and verified against it, or
   labelled unverified.** A vendor surface nobody has driven is documented as undriven.
5. **Nothing under `evals/` runs in `task check`.** A paid run is never part of a gate. The subject
   checkout is `EP_REPO` (default `~/beyond10x/engineering-protocols`).
6. **Nothing under `website/` runs in `task check`**, and the built site is never committed —
   publishing goes through the Pages deployment API, so `build/` never enters history. CI builds the
   site on every PR that touches it (`.github/workflows/docs.yml`); a broken link fails that build
   (`onBrokenLinks: 'throw'`).
7. **The `docs/` → `website/` split is one-way.** `docs/design/` and `docs/research/` carry the
   reasoning — open questions, the amendment record, per-claim evidence labels — and are not
   published. `website/docs/` states the conclusion and cites where the reasoning lives. Where the
   two disagree, **the design document is right and the site is stale.**
8. **A design decision is written before it is built.** The protocol wire (events out, commands in),
   the control seam, the hermetic contract and the adapter obligations are
   `docs/design/metaharness-protocol-v0.1.md`. A change to any of them amends that document, and the
   review's corrections are recorded in it.
9. **The b10x adapter decides nothing, and asserts that it decides nothing.** `metaharness-b10x`
   runs in `DecisionMode::Observe` only; every `tool.requested` it emits carries
   `decision_required: false` and `Seam::None`. Giving it a seam would put the driven arm's
   treatment on top of the observed arm and make the two differ in name only.
10. **harness is taken by git revision, never by `path`.** `metaharness-tools` and `metaharness-b10x`
   depend on `b10x-harness-tools`, `b10x-harness-wire` and `b10x-harness-loop` with
   `git = "https://github.com/beyond10x/harness"` and a `rev` (`.cargo/config.toml` makes cargo
   fetch it with the system git). A `path` into the sibling checkout builds against whatever is
   checked out there, `--locked` cannot lock it, and the gate is green against a tree nobody named —
   which is how it stood until 2026-08-29. Re-pin deliberately, on a harness commit reachable from
   its `main` (harness invariant 13), and say which in the changelog.

## Safety envelope

A run spawns a **vendor binary holding the operator's real credentials**. Everything below is that
boundary; changing any of it is its own change with its own review.

- **The hermetic floor.** A run shares credentials with the operator and nothing else: no ambient
  plugins, no account-level MCP servers, no inherited environment. A scratch config home is how it
  is achieved; the transcript is how it is proven. Never soften a floor verdict to make a run pass —
  that is what invariant 3 exists to stop.
- **Per-call tool decisions.** Which tools the harness may call is decided per call, by the
  embedder, through the protocol — never once at launch. A decision path that answers without
  consulting the embedder is a silent allow.
- **`metaharness.frame/1` is a cross-repository contract.** The frame is minted by
  `engineering-protocols`' driver and consumed here: digest-verified on load, and refused **by name**
  when unreadable, untagged, misshapen or edited after sealing. Never widen that reader to accept a
  document failing any of the four checks, and never reorder tag → shape → digest. The other side
  pins the same bytes; changing the format is a coordinated migration under the atlas rule, not an
  edit.
- **Credentials are copied per spawn, never logged.** The Codex adapter copies the operator's
  `auth.json` into a scratch `CODEX_HOME` per spawn. Nothing in an event, a refusal message or a
  retained transcript may carry a credential value.
- **A retained transcript is evidence and may contain anything the vendor wrote.** Treat it as
  sensitive; never commit one that came from a real session.

## Out of scope

| Belongs elsewhere | Repo |
|---|---|
| An agent loop of our own — turn assembly, tool round trips, budgets | `harness` |
| Sandboxed execution, confinement, the operation ledger | `substrate` |
| Terminating LLM requests, model routing, backends | `llmgw` |
| The methodology specification, workflows, evidence semantics | `engineering-protocols` |

metaharness drives a loop; it does not have one. The `metaharness-b10x` adapter *observes* the b10x
harness and does not implement it.

## The gate

```console
task check
```

`cargo fmt --check`, then `cargo clippy --workspace --all-targets -- -D warnings`, then
`cargo test --workspace`. Green before any push.

**A green local gate does not guarantee a green CI.** The steps mirror each other; the toolchain
does not. CI installs whatever `stable` is that day, and a newer clippy can fail a commit that
passed locally on an older one. Run `rustup update` before pushing anything you will not get a
second chance at, and read the gate's own exit status — never a pipeline's. `task check 2>&1 | tail`
reports `tail`'s status, not the gate's.

## Live-evaluating our own harness

`evals/engineering-protocols/run-driven.sh` drives a real `protocol drive run` through metaharness
against a scratch copy of engineering-protocols, on either arm, and scores the transcripts. It is
how the native harness is compared against a vendor one on the same work.

```console
EVAL_ARM=b10x   bash evals/engineering-protocols/run-driven.sh   # the native loop
EVAL_ARM=claude bash evals/engineering-protocols/run-driven.sh   # the vendor, and the default
```

**This spends real money on a real model.** It is not part of `task check` and must never be put
there. Do not run two at once — they are isolated by scratch directory, but both build in the same
target directories and both consume the same budget, and two sessions ran it concurrently on
2026-08-29 for one answer.

### Before a b10x run, reinstall the binary

The eval resolves `$HOME/.local/bin/b10x-harness` — `EVAL_B10X_BINARY` overrides it — and **refuses
to start if that file is older than the harness repository's newest commit**, printing the
`cargo install` line to fix it. It refuses rather than correcting, because the install directory is
the operator's and a debug build silently replacing a release one changes what every other caller
on this machine gets. Expect the refusal whenever anyone has pushed to harness; it fired three
times in one afternoon, once over a docs-only commit, which is the guard being right rather than
noisy — it cannot know what is in a commit.

The script re-execs itself under `systemd-run --user --scope`. That is not cosmetic: substrate's
`probe_cgroup` requires the **calling process's own cgroup** to sit inside the configured root, so
from an ordinary login shell the machine reports no exec facts and the loop publishes six tools
instead of seven, with no error anywhere.

### Reading the result

`EVAL_EXIT` is 0 only when nothing failed; `unk` counts as a failure. The run prints its scratch
directory — keep it, it is the whole record:

```
<scratch>/ws_project/.engineering/runs/EVAL-1/1/transcripts/*.jsonl   metaharness.event/1 streams
<scratch>/trace-honest.txt, trace-denial.txt                          the per-row verdicts
<scratch>/drive.log, drive.err                                        the driver's own output
```

Never read the census by piping the script's stdout through `tail` or `head` — the verdict block is
long and the interesting lines are in the middle. Redirect to a file and grep it.

### Things that were true and cost a paid run each

- **A flag must be forwarded by every link in the chain**, and the chain is
  `protocol drive` → `metaharness run <arm>` → the harness binary. `--plugin-dir` was wired through
  metaharness and the loop and still arrived empty, because engineering-protocols'
  `b10x_argv` never emitted it. Reading the code did not show this; a paid run did. When a flag
  does not arrive, check **every** link before suspecting the one you changed.
- **The b10x adapter writes `content: null` on every `tool.result`.** Any census keyed on
  `.content` reads 0 on that arm whatever happened. Refusals cross as `warning` with a `code`.
- **A row that names only vendor tool names decides nothing on the native arm.** Expectation rows
  union `tools:` with `operations:`; the native arm spells the entry `run` and the operation
  `shell`, never `command.execute`.
- **A mechanism row and an outcome row cannot both pass once the mechanism lands.**
  `the-planning-guidance-was-loaded` asks whether the model ran the CLI's own `skill load`; a
  harness that *offers* skills hands it over and the call never happens, so the row went `ok`
  before that landed and `unk` after, on a strictly better-informed run. It is advisory now. A row
  that fails a run for improving is not a gate.
- **A census that depends on what a model happens to do is not a census.** The surface-denial
  column read 1 and 0 on two runs of the same three commits, because nothing in either prompt asked
  for a program outside the declared set — the 1 came from an unasked-for `rm` cleanup. The
  `specify` prompt now asks for `run ["env"]` deliberately.
- **`unk` is not a pass.** An empty selection reads `unk` and never `ok`, on purpose: a step that
  never made the call cannot satisfy a row by having nothing to judge.

### What the four enforcement tiers are, as flags

The native arm is only comparable when all four are declared; three of them were declared nowhere
the loop could see until 2026-08-29, and the column measured one tier while reading like four.

| tier | how it reaches the loop |
|---|---|
| publication | the machine's own facts — a tool it cannot confine is never published |
| ceiling | `--approve-up-to <RISK>` (never `--yes`, which approves the destructive class and does not combine with a ceiling) |
| approver | `--approve auto\|prompt\|deny\|all` |
| content hook | `--hooks <FILE>`, written per step by the driver |

## Releases

Cut `CHANGELOG.md` under a version heading at a fully gated `main` commit, then write an annotated
tag whose name is the bare version — `0.1.0`, the version and nothing else (atlas § *Naming*).
The full gate comes first; component steps alone are not enough.

## Where work is tracked

| What | Where |
|---|---|
| Directions the operator has scheduled | `docs/ROADMAP.md` |
| Binding designs and their amendment records | `docs/design/` |
| Investigations behind a design | `docs/research/` |
| What shipped | `CHANGELOG.md`, and `git tag -n99` |
| The evaluation machinery and its results | `evals/` (never in the gate) |

## Conventions

- Rust CLIs use `clap`'s derive API. Hand-rolled argv parsing is not accepted.
- Task runner is `Taskfile.yml` (go-task). Do not add a Makefile.
- A claim on the public site obeys the same rule as a claim in the code: pinned to a version and
  verified against it, or labelled unverified.
