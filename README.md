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
- **As a library:** `Metaharness::new(Kind::Claude).hermetic().allow_tools([…]).run(prompt)` —
  the same run, embedded.

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

Pre-v1. The protocol is being designed in `docs/design/`; nothing here is stable.
