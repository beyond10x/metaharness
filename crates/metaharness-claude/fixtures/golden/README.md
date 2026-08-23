# Golden samples — recorded real wire (adapter contract CT-2)

These files are **not synthesised**. They are the bytes Claude Code actually wrote during one
controlled hermetic run, kept so the adapter is tested against the vendor's real wire rather
than against this crate's own assumptions about it (design
`docs/design/adapter-contract-v0.1.md`, milestone CT-2).

## Provenance

| fact | value |
|---|---|
| captured | 2026-08-23 |
| binary | `claude` 2.1.240 (`claude --version`) — **the adapter's pin since 2026-08-23** (amendment a11). It was captured off-pin, against 2.1.239, and the pin moved to it rather than the other way round: these bytes are not re-labelled, ever. The pin pair is CT-3's business, and `golden-version-pair` now passes silently |
| command | `metaharness run claude --hermetic --max-turns 2 --retain-dir <dir> -p "Run exactly one tool call: ls via the Bash tool. …"` |
| run shape | scratch config home, scratch cwd, `--strict-mcp-config`, one `Bash(ls)` call, ~$0.24 on the operator's subscription |
| reviewed | before commit, for credentials and account identifiers: none present. Paths are the run's own scratch (`~/.cache/claude-tmp/.tmp*`); ids are the session's own UUIDs |

## Files

| file | face | read by |
|---|---|---|
| `transcript.jsonl` | the `stream-json` record | `golden-transcript` vector (`src/vectors.rs`) |
| `hook-input.json` | the raw `PreToolUse` stdin, exactly as published into the hook channel | `golden-hook-input` vector |
| `transcript.expected.jsonl` | **generated** — the event stream the reader owes for `transcript.jsonl` | the same vector, byte-exact |

## Capture version and pin are two different facts

The `captured` and `binary` rows above are a **fact about these bytes** and are never edited: a
golden that was re-labelled to match a moved pin would be a forged provenance, and the vector that
reads it would be agreeing with a lie. The pin lives in `PINNED_VERSIONS`, the capture version lives
here, and `golden-version-pair` (`src/vectors.rs`) is the one place the two are compared —
agreement passes silently, a disagreement is a **named warning** that stands until somebody pays
for a real re-capture. So a re-pin is free and honest; only a re-capture costs money.

## Re-capture (one-time cost per pin)

1. `metaharness run claude --hermetic --retain-dir <dir> -p "…"` — a prompt that makes exactly
   one tool call.
2. Review `<dir>/transcript.jsonl` and `<dir>/requests/*.json` line by line before they enter
   the tree; then copy them over `transcript.jsonl` and `hook-input.json`.
3. `cargo test -p metaharness-claude --lib regenerate -- --ignored` rewrites
   `transcript.expected.jsonl` **and the three synthesised `../c2/*.expected.jsonl`**, all from
   committed inputs and all offline; review that diff — it is the mapping's changelog. The same
   command is what a *protocol* change is regenerated with, since a field added to an event moves
   every expectation at once and hand-editing JSONL to match a serde field order is how a fixture
   stops describing what the reader does.
4. Update the recorded values pinned in `golden_hook_vector` and this table.
