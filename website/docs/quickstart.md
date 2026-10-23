---
id: quickstart
title: Quickstart
sidebar_position: 2
---

# Quickstart

`doctor`, `capabilities` and `conformance` need **no model, no network and no credential**. Start
with those: they tell you whether the seam is wired before you spend anything on a live run.

## Build and install {#build}

```bash
git clone https://github.com/beyond10x/metaharness.git
cd metaharness
cargo install --path crates/metaharness-cli --locked
metaharness --version
```

It needs Rust 1.98 or newer (`rust-version` in `Cargo.toml`), and `--locked` builds against
the checked-in `Cargo.lock`. `cargo install` puts the `metaharness` binary, the `[[bin]]` of
`crates/metaharness-cli`, in `$CARGO_HOME/bin` (`.cargo/bin` in your home directory unless you set
`CARGO_HOME`); every step below calls it by name, so that directory has to be on your `PATH`. To
use the checkout without installing, `cargo run -p metaharness-cli -- <VERB> …` takes the same
arguments.

## The gate

The workspace gate is one task, run from the checkout root:

```bash
task check
```

It needs [go-task](https://taskfile.dev), Rust 1.98 or newer and the `ess` CLI of ESS 0.68.0 on
your `PATH`: `spec/ess-inputs.yaml` declares `requires: ess 0.68.0` and the gate validates with
`--strict-requires`. It runs these steps, in this order (`Taskfile.yml`, task `check`):

```text
ess specify validate --path spec --strict-requires
ess specify validate --path spec/monetary --strict-requires
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run --locked --quiet -p metaharness-docs -- check
```

Nothing under `evals/` runs in `task check`. A paid run is never part of a gate.

## 1. Is the installed vendor the one the adapter was written against?

```bash
metaharness doctor claude
metaharness doctor codex
```

Every adapter claim about a vendor binary is pinned to a version. `doctor` compares the installed
binary against that pin. During a run, `--strict-version` turns a mismatch from a warning into a
refusal before anything spawns.

## 2. What does the adapter say it can do?

```bash
metaharness capabilities claude
metaharness capabilities claude --render
```

Declared tiers, pinned versions, and — with `--render` — the neutral-operation → vendor-tool
table. This exists so an embedder can refuse *early* rather than discovering mid-run that a tier
is absent.

## 3. Does the seam actually hold?

```bash
metaharness conformance claude    # 30 vectors
metaharness conformance codex     # 24 vectors, 4 of which run a real process and the real hook
metaharness conformance b10x      # 8 vectors
```

Free conformance vectors: no model, no network, no credential. Each count is
`metaharness::conformance_vectors(kind).len()` for this release, and
`crates/metaharness-cli/tests/website_claims.rs` fails when a page states another.

## 4. A real run

This step starts Claude Code with your own login, and the model call it makes is billed to it.
Run it from the checkout root:

```bash
metaharness run claude --hermetic --cwd "$PWD" -p "list the crates in this workspace"
```

`--cwd "$PWD"` names the checkout as the child's working directory. Without it the child starts in
an empty scratch directory and has no crates to list. Naming a directory trades two hermetic rows
for real work, and the child can write to it; see
[running over a real repository](#running-over-a-real-repository).

What happens:

1. `claude` is spawned into a **scratch config home** — not yours.
2. A blocking `PreToolUse` hook is installed, and metaharness answers it **per call**.
3. The session streams out of stdout as [protocol events](./protocol/events.md), one JSON object
   per line.
4. Steering [commands](./protocol/commands.md) are read from stdin, one JSON object per line.
5. The raw vendor transcript is retained for the auditor.
6. The process exits when the stream completes: `0` when it closed complete on the vendor's
   terminal record, `3` otherwise. No hermetic row decides that exit.

To have the floor decide it, ask for an audit:

```bash
metaharness run claude --hermetic --audit --cwd "$PWD" -p "list the crates in this workspace"
```

This run exits on the [hermetic floor's](./hermetic.md) verdict, folded with the stream's own
completeness and any external auditor's exit, and prints the floor's rows on stderr. `--hermetic
strict` asks for the same audit. (`verdict` in `crates/metaharness-cli/src/lib.rs` reads
`Run::wants_audit`; without it the exit is `Run::exit(None)`, in `crates/metaharness/src/audit.rs`.)

### Deciding each call yourself

```bash
metaharness run claude --hermetic --decisions ask -p "…"
```

With `--decisions ask`, every tool call arrives as a `tool.requested` event carrying
`decision_required: true`, and the run blocks until you answer with `tool.decide`. See
[the control seam](./control-seam.md).

### Governing a run from outside the process

```bash
metaharness run claude --hermetic --frame step.frame.json -p "…"
```

The [frame](./frames.md) is a sealed `metaharness.frame/1` document: digest-verified on load and
enforced from the first turn. This is the seam an external driver integrates through — it writes
the frame as a file and never links this workspace.

### Running over a real repository

```bash
metaharness run claude --hermetic --cwd ~/projects/thing -p "…"
```

`--cwd` is the declaration that **trades two hermetic rows for real work**: the child runs in your
tree **and can write to it**, so H7 and H11 stop being impositions and are attested *unavailable*,
naming the directory. The vendor sandbox is widened to that tree for the run — on codex,
`sandbox_mode = "workspace-write"` — and the H7 attestation row states the grant in words, so the
run's own record says it could change your repository. `--hermetic strict` therefore refuses such a
run, plain `--hermetic` reports it honestly. A scratch-cwd run writes nothing, anywhere.

## AEP-governed workflows

Since 0.7.0, metaharness hosts model-backed AEP workflows through `metaharness aep drive`. AEP
supplies the governor, planning commands and retained run storage; Harness supplies the native
agent loop. The AEP CLI refuses model-backed maps before allocating a run and names this host.

From an AEP project with its task and step map already configured, inspect the command and run
state first:

```bash
metaharness aep drive run --help
aep drive status
```

Starting a model is an explicit paid operation. Supply the plugin directory you intend to load
and both spend bounds:

```bash
METAHARNESS_LIVE=1 metaharness aep drive run --project . --map development/default \
  --plugin-dir /path/to/agentplugins/plugins/aep-plan --pause-on-approval \
  --budget-usd 10 --assume-usd-per-run 1
```

Use `metaharness aep drive resume RUN_ID` with the same live opt-in for a compatible paused
model-backed run. A resume may narrow its recorded budget down to the remembered assumed charge
and not below it, and may never raise it. Use `--aep-binary`
when the planning executable must be selected explicitly. Live evaluation is
`metaharness aep drive eval run`; `aep drive eval run --stream` reads recorded evidence offline.

`METAHARNESS_LIVE=1` is checked by `metaharness aep drive run` and `metaharness aep drive resume`
for a step map with an `llm` step, and by `metaharness aep drive eval run` without `--stream`. A
plain `metaharness run` does not check `METAHARNESS_LIVE`: it spawns the harness with the
operator's credentials whatever the variable holds.
See the [AEP integration guide](https://beyond10x.github.io/docs/aep/guides/integrate-a-harness/)
for step maps, operator pauses and native transition hooks.

## Exit codes

Every verb exits `0`, `1`, `2` or `3`. What each code means, and the three lines an audited run
ends on, are in [the CLI reference](./reference/cli.md#exit-codes).
