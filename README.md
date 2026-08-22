# metaharness

One interface to many agent harnesses. A harness — Claude Code, Codex, the next one — keeps its
own loop, its own tools and its own credentials; metaharness drives it from outside and makes the
run **observable, steerable and hermetic**, the same way regardless of which harness is inside.

## The shape

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
2. **Hermetic**: a run shares credentials with the operator and nothing else — no ambient
   plugins, no account-level MCP servers, no inherited environment. Hermeticity is asserted from
   the transcript, not assumed from a directory.
3. **In control at every step**: which tools the harness may call is decided per call, by the
   embedder, through the protocol — not once at launch.

## Where it came from

Two working systems, each of which built half of this and proved it:

- [`engineering-protocols`](https://github.com/former organization/engineering-protocols) — hermetic
  headless evals, deterministic hook enforcement with 1:1 denial audits, and a transcript IR
  (`trace-ir/1`) that turns "the agent behaved" into a checked claim.
- former organization's agent runtime — harness adapter classes (a vendor keeps its loop; we drive its
  documented surface), approvals as blocking calls, steering, and the port seam that makes
  in-process and over-the-wire tool binding indistinguishable to the loop.

## Status

Pre-v1, and the design in `docs/design/` is what is binding — where this code and that document
disagree, the document is amended rather than the disagreement left in the code.

**M1 is built:** the event and command vocabulary, the workflow frame, the Claude Code adapter's
launch construction and transcript reading, the run loop with per-call decisions, and `--audit`'s
built-in hermetic floor. `metaharness capabilities claude`, `metaharness capabilities claude
--render` and `metaharness conformance claude` work today — the last one runs 14 conformance
vectors with no model, no network and no credential.

**M1 does not drive the real `claude` binary.** The spawn is behind a trait, the whole path is
exercised through a scripted process, and `metaharness run` refuses with exit 2 naming what is
missing rather than pretending. `project`, `audit` and `doctor` refuse the same way, each naming
what it is waiting for.
