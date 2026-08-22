# Changelog

What changed. The design document carries *why*; where code and design disagreed, the design
was amended and the amendment is named here.

## [Unreleased]

### Added
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

### Not yet
- No real vendor spawn: `metaharness run` exits 2 naming what is missing. The path is exercised
  through a scripted process.
- No Codex adapter.
