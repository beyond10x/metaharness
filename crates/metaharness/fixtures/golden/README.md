# Golden records — the contract as a consumer reads it (adapter contract CT-1)

The three `contract-result-<kind>.json` files are **not written by hand**. Each is
the exact line `metaharness conformance <kind> --contract` printed on stdout, redirected into the
tree, trailing newline included — the bytes an outside consumer reads as evidence.

They exist because the record's shape crosses a repository boundary. `AEP` is
public and this workspace is not, so no crate dependency may pass between them; what passes is the
`contract_result` **vocabulary** (`{checked, failed, breaking_changes, provider, consumer}`, design
`docs/design/adapter-contract-v0.1.md`). A vocabulary with no committed sample is a shape both sides
believe in separately. These files are the samples, and `tests/contract_golden.rs` rebuilds the
record through the real `contract_result(kind, &conformance_vectors(kind))` and compares byte for
byte.

## Provenance

| fact | value |
|---|---|
| captured | 2026-08-23; the codex record **re-recorded the same day** when its launch face was filled (CT-4's gap closed, LP-4's free half, the allow vector) |
| command | `./target/debug/metaharness conformance <kind> --contract`, exit `0` for claude, codex and b10x |
| tree state | CT-1..CT-4 built, protocol amendments a9 through a11, the loopback wave; the codex C1 vectors, the codex loopback door, observe mode and plugin injection in the working tree |
| re-recorded | 2026-08-23, both records, one cause each: the claude re-pin (amendment a11) moved `provider` `claude 2.1.239` → `claude 2.1.240`, the observe/plugin launch vectors (amendment a10) moved claude's `checked` 20 → 24, and the codex launch face filling CT-4's gap plus the loopback door moved codex's `checked` 10 → 17. `failed: 0` and `breaking_changes: 0` throughout |
| stdout only | the `golden-version-pair` warning goes to **stderr** and is therefore not in these files — that is the CLI's contract, not an omission. **Codex still carries it** (0.144.0 vs 0.145.0); claude's stopped when the pin met the recorded capture at 2.1.240 |
| reproducible | every vector in all three runs is free: no model, no network, no credential, no clock |
| reviewed | before commit: no path, no account identifier, no credential. Six keys, two integers, three constants and a pinned version each |

| file | record |
|---|---|
| `contract-result-claude.json` | `checked: 24` — 7 launch, 3 synthesised replays, 3 golden (CT-2/CT-3), 8 control, 3 spawn |
| `contract-result-codex.json` | `checked: 17` — **6 launch**, 4 synthesised replays, 3 golden, **4 spawn**. Was `10` until 2026-08-23, when the launch face CT-4 recorded as a named gap was filled (+6: two of them the loopback door's, LP-4) and the spawn tier gained the allow round trip (+1) |
| `contract-result-b10x.json` | `checked: 7` — one recorded launch, one byte-exact recorded loop replay, one capture-banner/version-pin pair and four provider-emulated enforcement outcomes (unpublished tool, approval denial, budget stop and cancellation). The hook-input row is N/A because the observe-only adapter has no metaharness hook seam |

The adapters' counts differ and always will: they are counts of *that adapter's* vectors, not a
score. Codex carries no `control` tier of its own — those seven vectors are metaharness's own
machinery and run once, under claude — and its launch face needs two more vectors than claude's
because one vendor's credential door has two classes and only one of them is routed.

## What it is for

**To break when the shape moves.** A consumer reading these bytes reads six keys in one order; key
order is part of what it reads, so the test pins the order as well as the values. `checked` moving
is the ordinary case — a vector added or removed — and it is still a **deliberate** regeneration,
because a count that drifts silently is a contract nobody agreed to. `provider` moving means the pin
moved. `failed` or `breaking_changes` moving means the contract is red and the record is not the
thing to fix.

## Re-recording

1. Make the change that moves the record, and know which field it moves and why.
2. `cargo test -p metaharness --test contract_golden regenerate -- --ignored` — it rebuilds all
   files through the library path, offline, from the same vectors the CLI runs.
3. Read the diff: it is one line per adapter, and every changed field is a claim.
4. Update the counts in the table above, the vector-count pins that moved
   (`crates/metaharness-claude/tests/adapter.rs`, `crates/metaharness/src/vectors.rs`,
   `crates/metaharness/src/spawn_codex_vectors.rs`), the site's own counts
   (`website/docs/status.mdx`, `website/docs/harnesses/codex.mdx`), and the changelog — and tell the
   consumer, because it is building against these bytes.
