# Golden sample — a frame minted by the other side of the seam

`metaharness-frame-canonical.json` is **not synthesised here**. It is the document
`AEP` mints and hands this repository across a process boundary as
`--frame <file>`, copied in byte for byte. It exists so that the two repositories, which share a
vocabulary and no code, are compared by something other than good intentions.

That gap is real and named on both sides. The public repository cannot depend on this private
workspace, so its own contract suite is a **transcription** of `frame.rs` — a second implementation
of the digest rule, written out by hand — and its stated open risk is that *"the transcription's
continued agreement with `frame.rs` is closed only by the metaharness-side replay of these bytes."*
`tests/frame_golden.rs` is that replay: these bytes, through the real
[`Frame::parse_document`](../../src/frame.rs), with the sealed digest re-derived here.

## Provenance

| fact | value |
|---|---|
| recorded | 2026-08-23 |
| minted by | `AEP`, `crates/protocol-cli` — its driver's own `frame_document` path, for one deterministic `llm` step. Not typed by hand |
| minter's copy | `crates/protocol-cli/fixtures/metaharness-frame-canonical.json` in that repository |
| file sha256 | `ef897a58a624848aad942d69d2745b431f2eaad5180cd0f5b2e1c8975adcb93b` — verified equal to the minter's copy at the time of the record |
| sealed digest | `43a6f845a21f3475569323950a9d276bfed3df11979adc3edf18878da6963a12`, stated inside the document and re-derived by `Frame::computed_digest` from its contents |
| reproducible | nothing in the minting path reads a clock, a random source or anything else off the machine that minted it |
| reviewed | before commit: no credential, no account identifier, no absolute path. The other repository is public, and this one takes nothing from it that could not be |

## What it is for

**To break when either side moves.** A frame is cited by digest by every event downstream of it, so
a producer and a consumer that quietly disagree about the canonical form do not produce a wrong
answer — they produce a driven run that dies at its first step, after the session is paid for. The
ordering rule is where that has actually happened: the first cross-repository document sorted its
operations by the enum's variant order, which no producer outside this workspace could have known,
and § 5.5 of `docs/design/metaharness-protocol-v0.1.md` is written the way it is because of it.

So the fixture is only doing its job when a change here makes it fail. **A failure is a question,
not a chore**: has this repository's canonical form moved (then the other side must be told, and the
document re-minted), or has the copy drifted from what the minter emits (then this file is stale)?
Re-sealing the document to make the suite green would delete the only evidence either way.

## Re-recording

1. Have `AEP` re-mint its fixture through `protocol-cli`'s own driver path — never
   by editing the JSON, which would make the file evidence of nothing.
2. Copy it here byte for byte and check `sha256sum` against that repository's copy.
3. Update the two hashes in the table above **and** the pinned digest in `tests/frame_golden.rs`,
   and say in the changelog which side moved.
