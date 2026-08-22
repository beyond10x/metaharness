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

**M2 is built: `metaharness run claude --hermetic -p "…"` drives the real binary end to end.**
It spawns Claude Code 2.1.239 into a scratch config home, installs a blocking `PreToolUse` hook,
answers that hook's calls per call, streams the session out as protocol events on stdout, takes
steering on stdin, retains the raw transcript, and exits on the hermetic floor's verdict.
`capabilities`, `conformance` and `doctor` work with no model and no credential;
`metaharness conformance claude` runs **17** vectors that way.

**The seam is real, and it was verified against a paid run.** A frame that admitted no shell was
given a prompt that asked for one: metaharness denied the call at the hook, the call did not run,
and *the vendor's own terminal record* listed `Bash` in `permission_denials`. That is the claim
the whole design exists to be able to make, and it is the one thing no free test tier can reach.

Two findings from that first live run are worth naming, because both were defects in metaharness
and neither was reachable without a real session: the hermetic floor read the wrong field for
"was an API key in use", and it treated *"this run pinned no documents"* as *"nobody found out
whether the documents moved"* — which made `--hermetic strict` unpassable. Both are fixed, both
are regression-tested, and both are recorded as amendment a4 in the design.

**The frame crosses the process boundary (amendment a5).** `metaharness run claude --hermetic
--frame step.frame.json -p "…"` now takes the workflow frame as a sealed `metaharness.frame/1`
document: digest-verified on load, refused by name when unreadable, untagged, misshapen or
edited after sealing, and enforced per call from the first turn. This is the seam an external
driver integrates through — it writes the frame as a file and never links this workspace.

**The Codex adapter exists (CX-M1).** `metaharness-codex` reads the session rollout — the record
that carries timestamps, durations and per-turn usage where `codex exec --json` stdout does not —
version-gated on the 0.145.0 pin, with every unmapped shape preserved as `opaque`. `capabilities
codex`, `conformance codex` (4 replay vectors) and `doctor codex` all work with no model and no
credential; `run codex` is refused by name until a driven spawn (CX-M2), and `tool.decide` stays
refused until that run proves the vendor's documented hook contract from metaharness's own seam.
The evidence base is `docs/research/2026-08-21-codex-harness-research.md`.

**The eval machinery lives here now** (`evals/`), migrated from engineering-protocols under its
`epic:metaharness-migration`: the driven eval reads its denial census from `tool.decided` events
in the run's own streams, and nothing under `evals/` is part of `task check`.

**What is still not built:** `metaharness project` (gated on Q9 — `trace-ir/1` has no reader) and
`metaharness audit` over a transcript metaharness did not itself launch. Both refuse with exit 2
naming what they wait for. The live runs cost money and are behind
`METAHARNESS_LIVE=1`; they are never part of `task check`.
