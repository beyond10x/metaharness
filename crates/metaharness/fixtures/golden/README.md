# Golden records — the contract as a consumer reads it (adapter contract CT-1)

`contract-result-claude.json` and `contract-result-codex.json` are **not written by hand**. Each is
the exact line `metaharness conformance <kind> --contract` printed on stdout, redirected into the
tree, trailing newline included — the bytes an outside consumer reads as evidence.

They exist because the record's shape crosses a repository boundary. `engineering-protocols` is
public and this workspace is not, so no crate dependency may pass between them; what passes is the
`contract_result` **vocabulary** (`{checked, failed, breaking_changes, provider, consumer}`, design
`docs/design/adapter-contract-v0.1.md`). A vocabulary with no committed sample is a shape both sides
believe in separately. These two files are the sample, and `tests/contract_golden.rs` rebuilds the
record through the real `contract_result(kind, &conformance_vectors(kind))` and compares byte for
byte.

## Provenance

| fact | value |
|---|---|
| captured | 2026-08-23 |
| command | `./target/debug/metaharness conformance claude --contract` and `… codex --contract`, both exit `0` |
| tree state | CT-1..CT-3 built, protocol amendment a9, the loopback wave in the working tree |
| stdout only | the `golden-version-pair` warning both adapters carry today goes to **stderr** and is therefore not in these files — that is the CLI's contract, not an omission |
| reproducible | every vector in both runs is free: no model, no network, no credential, no clock |
| reviewed | before commit: no path, no account identifier, no credential. Six keys, two integers, three constants and a pinned version each |

| file | record |
|---|---|
| `contract-result-claude.json` | `checked: 20` — 4 launch, 3 synthesised replays, 3 golden (CT-2/CT-3), 7 control, 3 spawn |
| `contract-result-codex.json` | `checked: 10` — 4 synthesised replays, 3 golden, 3 spawn. No launch vector; the codex declaration names that gap (CT-4) |

## What it is for

**To break when the shape moves.** A consumer reading these bytes reads six keys in one order; key
order is part of what it reads, so the test pins the order as well as the values. `checked` moving
is the ordinary case — a vector added or removed — and it is still a **deliberate** regeneration,
because a count that drifts silently is a contract nobody agreed to. `provider` moving means the pin
moved. `failed` or `breaking_changes` moving means the contract is red and the record is not the
thing to fix.

## Re-recording

1. Make the change that moves the record, and know which field it moves and why.
2. `cargo test -p metaharness --test contract_golden regenerate -- --ignored` — it rebuilds both
   files through the library path, offline, from the same vectors the CLI runs.
3. Read the diff: it is one line per adapter, and every changed field is a claim.
4. Update the counts in the table above, the vector-count pins that moved
   (`crates/metaharness-claude/tests/adapter.rs`, `crates/metaharness/src/vectors.rs`), and the
   changelog — and tell the consumer, because it is building against these bytes.
