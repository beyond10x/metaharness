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
It spawns Claude Code 2.1.240 into a scratch config home, installs a blocking `PreToolUse` hook,
answers that hook's calls per call, streams the session out as protocol events on stdout, takes
steering on stdin, retains the raw transcript, and exits on the hermetic floor's verdict.
`capabilities`, `conformance` and `doctor` work with no model and no credential;
`metaharness conformance claude` runs **24** vectors that way, and `conformance codex` **17**.

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

**Codex is driven for real (CX-M2).** `metaharness run codex --hermetic -p "…"` starts a real
`codex exec` into a scratch `CODEX_HOME`, copies the operator's `auth.json` in per spawn, declares
a blocking `PreToolUse` hook, tails the **session rollout** for events — the record that carries
timestamps, durations and per-turn usage where `codex exec --json` stdout carries none — retains
those bytes for the auditor, and answers the hook per call. `capabilities codex`, `conformance
codex` (**10** vectors, including three that run a real process and the real hook program) and
`doctor codex` all still work with no model and no credential.

**The seam was verified against a paid run, on this vendor too.** A policy that admitted no shell
met a prompt that asked for one. The hook process received the call — `"tool_name":"Bash"`,
`"tool_use_id":"exec-96257928-…"` — metaharness answered `deny` with a reason, and *the vendor's
own session record* reads `Command blocked by PreToolUse hook: this step admits no shell, so the
command did not run` with an **empty** `Output:`. The model's closing message was *"The command was
blocked and did not run."* So `tool.decide` is `Honoured` and the call tier is `Delivered`. The
`allow` half of that wire was **driven live on 2026-08-23**: the hook held a real `Bash` call,
metaharness answered `permissionDecision: allow`, and the rollout's own `custom_tool_call_output`
carried the command's output — the grant is honoured, not discarded. One caveat travels with it:
the binary that honoured it was the child-`PATH` codex **0.144.0** (the pin is 0.145.0; the
two-install warning fired, as it must), so the observation names 0.144.0.

Three things about Codex cost more to learn than the code that uses them, and all three are silent
failures — which is why every claim above is read from the run's own record and not from the file
that configured it:

- **A hook is declared in `config.toml`, not `hooks.json`.** A `hooks.json` is a plugin manifest's
  file. An unrecognised key under `[hooks]` is dropped *without failing the config load*, so a
  misconfigured seam and a run where nothing was attempted are the same observation.
- **A hook in a fresh `CODEX_HOME` never fires without `--dangerously-bypass-hook-trust`.** A
  scratch home cannot hold persisted trust. The flag warns about running *somebody else's* hook
  unvetted; the only hook here is the one metaharness wrote a moment earlier.
- **The hook speaks Claude Code's tool vocabulary.** `tool_name` is `Bash`, where the rollout calls
  the same call `exec` and the binary's own tool list calls it `shell`. A rendering table built
  from the record would have denied every shell call and reported it as a frame decision.

One thing the live run found that nobody was looking for: `codex --version` says `0.145.0` and the
`session_meta.cli_version` written by the run that binary starts says `0.144.0`. The adapter keeps
one pin, the reader warns rather than widening it, and the split is **Q18**.

**The eval machinery lives here now** (`evals/`), migrated from engineering-protocols under its
`epic:metaharness-migration`: the driven eval reads its denial census from `tool.decided` events
in the run's own streams, and nothing under `evals/` is part of `task check`.

**What is still not built:** `metaharness project` (gated on Q9 — `trace-ir/1` has no reader) and
`metaharness audit` over a transcript metaharness did not itself launch. Both refuse with exit 2
naming what they wait for. The live runs cost money and are behind
`METAHARNESS_LIVE=1`; they are never part of `task check`.
