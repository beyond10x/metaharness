# metaharness

One interface to many agent harnesses. A harness — Claude Code, Codex, the next one — keeps its own
loop, its own tools and its own credentials; metaharness drives it from outside and makes the run
**observable, steerable and hermetic**, the same way regardless of which harness is inside.

The problem it removes: every vendor harness has a different transcript format, a different way to
approve or refuse a tool call, and a different set of things it silently inherits from the machine
it runs on. Writing a workflow against one of them means writing it again for the next one, and
being unable to prove what a run was actually allowed to do.

```text
              events out (JSONL) ─────────▶  your process / your workflow engine
metaharness
              commands in (steering) ◀─────  approve / deny a tool call, inject, halt
```

- **As a binary:** `metaharness run claude --hermetic -p "…"` — events on stdout, steering on
  stdin. Swap `claude` for `codex` and the protocol does not change.
- **As a library:** `Metaharness::new(Kind::Claude).with_hermetic(Strict).with_decisions(Ask)
  .start(prompt)` — the same run, embedded, answering `tool.requested` events as they arrive.

## The three promises

1. **Unified**: one event stream and one command set; everything harness-specific lives in that
   harness's adapter crate and nowhere else.
2. **Hermetic**: a run shares credentials with the operator and nothing else — no ambient plugins,
   no account-level MCP servers, no inherited environment. Hermeticity is asserted from the
   transcript, not assumed from a directory.
3. **In control at every step**: which tools the harness may call is decided per call, by the
   embedder, through the protocol — not once at launch.

## Where it sits

| direction | repo | relationship |
|---|---|---|
| drives | Claude Code, Codex | vendor binaries, spawned into a scratch config home |
| drives | [harness](https://github.com/beyond10x/harness) | the b10x agent loop, via the `b10x` adapter — observed rather than driven, because its published toolset already is its policy |
| scored by | [engineering-protocols](https://github.com/beyond10x/engineering-protocols) | supplies the workflows and trace expectations the evals under `evals/` judge a run against |
| mapped in | [atlas](https://github.com/beyond10x/atlas) | how this repo fits the rest of `beyond10x` |

Nothing consumes metaharness as a dependency yet. An external driver integrates through the sealed
frame document (`--frame step.frame.json`) and the event/command protocol — it writes a file and
never links this workspace.

## Status

**Pre-v1. Tagged `0.1.0` (2026-08-24).** The design in `docs/design/` is binding: where this code
and that document disagree, the document is amended rather than the disagreement left in the code.

| verb | state |
|---|---|
| `run claude` | drives the real binary end to end; verified against a paid run |
| `run codex` | drives real `codex exec`; verified against a paid run |
| `run b10x` | observes the b10x loop |
| `capabilities`, `conformance`, `doctor` | work with no model and no credential |
| `mcp-serve` | serves the owned tool surface over MCP on stdio |
| `project` | refuses with exit 2 — `trace-ir/1` has no reader yet |
| `audit` over a foreign transcript | refuses with exit 2 |

The live runs cost money, sit behind `METAHARNESS_LIVE=1`, and are never part of `task check`. The
per-change record of what was verified and what it cost to learn is in
[`CHANGELOG.md`](CHANGELOG.md).

## Build, test, run

The gate is **`task check`** — `cargo fmt --check`, `cargo clippy --workspace --all-targets -D
warnings`, `cargo test --workspace`. Green before any push.

| command | what it does |
|---|---|
| `task check` | the full gate |
| `task fmt` | format the workspace |
| `task docs` | the documentation site in dev mode, hot reload |
| `task docs:build` | build the site; a broken link fails the build |

Rust 1.98, edition 2024. To exercise the binary without a credential:

```sh
cargo run -p metaharness-cli -- conformance claude
cargo run -p metaharness-cli -- capabilities codex --render
cargo run -p metaharness-cli -- doctor claude
```

## Layout

| path | holds |
|---|---|
| `crates/metaharness` | the library: builder, run, hermetic floor, audit |
| `crates/metaharness-protocol` | the harness-neutral wire — events a run emits, commands that steer it, `RunSpec` |
| `crates/metaharness-claude` | the Claude Code adapter, and nothing else |
| `crates/metaharness-codex` | the Codex adapter, and nothing else |
| `crates/metaharness-b10x` | the b10x adapter: a loop we own, observed rather than driven |
| `crates/metaharness-tools` | the owned tool surface, served to a vendor harness over MCP |
| `crates/metaharness-cli` | the `metaharness` binary |
| `docs/design/` | the binding design documents and their amendments |
| `docs/research/` | vendor behaviour established by reading or probing, not by design |
| `evals/` | paid and recorded evaluations, by subject — not part of `task check` |
| `website/` | the public documentation site (Docusaurus) |

## Read more

- [`docs/design/metaharness-protocol-v0.1.md`](docs/design/metaharness-protocol-v0.1.md) — the
  event and command protocol.
- [`docs/design/adapter-contract-v0.1.md`](docs/design/adapter-contract-v0.1.md) — what an adapter
  must declare and prove.
- [`docs/design/model-adapter-v0.1.md`](docs/design/model-adapter-v0.1.md) and
  [`loopback-provider-v0.1.md`](docs/design/loopback-provider-v0.1.md) — model selection and the
  loopback credential proxy.
- [`docs/ROADMAP.md`](docs/ROADMAP.md) — operator-scheduled directions.
- [`evals/README.md`](evals/README.md) — what the evals cover and what they cost.
- [`AGENTS.md`](AGENTS.md) — working agreements for anyone, human or agent, changing this repo.
- Published docs: <https://beyond10x.github.io/metaharness/>
