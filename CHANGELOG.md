# Changelog

What changed. The design document carries *why*; where code and design disagreed, the design
was amended and the amendment is named here.

## [Unreleased]

### Added
- **The on-disk frame document — `metaharness.frame/1` (amendment a5).** The format § 9.3
  correction 3 left owed now exists: one JSON object, a `format` tag on the D2 rule, every § 5.1
  field, and a digest that is **required to describe the contents** — SHA-256 over the compact,
  key-sorted serialization without `digest`/`format`, reproducible without linking this
  workspace. `--frame <file>` and `.with_frame_file(path)` resolve it in the library at start
  (D11 intact: the binary carries only a path), and every failure is a free pre-spawn refusal by
  name: `FrameUnreadable`, `FrameInvalid` (untagged, misshapen or digest-broken, parser text
  verbatim), and `FrameConflict` when an in-memory frame and a document compete. A launch-time
  frame now requires `tool.decide` rather than the undriven mid-session `frame.set`, and the
  Claude adapter's `FrameFormatUnowned` refusal is gone rather than left as a variant nothing
  produces.

- `metaharness-protocol`: the wire — 19 events, 7 commands, versioned JSONL framing with the tag
  on every line, sequence numbers assigned in one place, the workflow `Frame` and the one
  function that renders it for the model, the 12 hermetic rows, adapter capabilities, and the
  structural projection into `trace-ir/1`.
- `metaharness-claude`: hermetic launch construction against Claude Code 2.1.239 — scratch config
  home, environment scrub, `--strict-mcp-config`, the `--bare`/`--safe-mode` denylist, the memory
  ancestor walk, the non-`async` `PreToolUse` hook definition, and a `SHADOWED` refusal for a seam
  another layer would override — plus stream-json transcript reading in which nothing is dropped.
- `metaharness`: the run loop with per-call decisions, several pending at once and answerable out
  of order, deadlines armed at delivery, and `--audit`'s built-in hermetic floor with exit codes
  0/1/2/3.
- `metaharness-cli`: `run`, `capabilities [--render]`, `conformance`, and honest refusals for
  `project`, `audit` and `doctor`. 14 conformance vectors run with no model and no credential.
- Design amendments a1–a3 and questions Q13–Q16.

- **The real spawn (M2).** `metaharness run claude --hermetic -p "…"` starts Claude Code 2.1.239
  for real: a constructed environment, a scratch config home, the credential re-copied
  immediately before every spawn, stdout streamed through the transcript reader into protocol
  events, stderr retained whole, and the raw bytes kept on disk for the auditor (O8).
- **A control seam a separate process can answer over.** The adapter now renders the
  `PreToolUse` program its hook definition always named, and metaharness answers it over a
  request/response channel. The program parses no JSON and needs no interpreter — it publishes
  the vendor's input under a name only it holds and waits for a file — and it fails closed with a
  reason on a missing channel, an unwritable channel or an unanswered call.
- **The correlation key, read off the vendor rather than guessed (V22).** The hook input carries
  `tool_use_id`, which is the same string the transcript's `tool_use` block calls `id`. This
  **closes Q16**, and M1's provisional envelope turns out to have been right, so it did not move.
- Two further driven rows: the `tool_use` record reaches stdout **before** the hook runs (V23),
  and a `--settings` file outside the config home still loads its hooks under
  `--setting-sources ""` (V24, which **answers Q14**).
- `metaharness doctor <kind>` — the installed vendor version against the adapter's pin, for free.
- Three C3 **spawn vectors**: a real process and the real hook program against a fake vendor, so
  the seam's round trip, the per-spawn credential copy and the retained transcript are all
  checked with no model, no network and no credential. `conformance` now runs 17 vectors.
- A C4 tier that exists: two live runs in `tests/live.rs`, `#[ignore]`d and behind
  `METAHARNESS_LIVE=1`.

### Fixed
- **The hermetic floor failed its own first live run, twice, and both were its fault**
  (design amendment a4). `H4` looked for the word the spec used in `apiKeySource`, a field that
  says `"none"` under an operator login — so every hermetic operator-login run reported a gap on
  the row it most clearly satisfied. `H10` read *"this run pinned no input tree"* as `unk`, which
  made `--hermetic strict` unpassable for every run that pins nothing, including the design's own
  example. Neither was reachable below C4: there is no real opening record before it.
- `metaharness run` no longer exits 2 on a well-formed spec, and `Refusal::NoSpawner` is gone
  rather than left as a variant nothing produces.

### Guarded
- **A paid run can no longer be reached from `task check`.** When `run` learned to spawn, two CLI
  tests that asserted it exited `2` kept passing their argv through and billed two real sessions.
  Those tests now use only pre-spawn refusals, and an interlock over the test file's own source
  refuses to let a prompt-carrying `run` argv back in.

### Not yet
- `metaharness project` is gated on Q9 (`trace-ir/1` is `Serialize`-only, so a document written
  there has no reader) and `metaharness audit` on the launch facts a foreign transcript cannot
  carry. Both refuse with exit 2, each naming what it waits for.
- `session.started` carries the transcript's path and not its digest: the opening record is line
  one of a file whose last line does not exist yet (**Q17**).
- No Codex adapter.
