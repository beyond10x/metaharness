# Caller-seam samples: the loop asks, metaharness answers (design amendment a61)

Two runs of the released `b10x-harness` 0.22.0 binary under `run --approve caller`, against
harness's deterministic local OpenAI Responses endpoint. This is `provider_emulated` evidence,
never `vendor_live`.

| file | what it is |
|---|---|
| `allow.jsonl`, `deny.jsonl` | the raw `--json` record of each run, 13 lines |
| `allow.answer.jsonl`, `deny.answer.jsonl` | the one line the loop read on its stdin for the call it asked about |
| `allow.expected.jsonl`, `deny.expected.jsonl` | generated: the record replayed through `B10xSeam` with the decision fed back, plus one `{"decision_line": …}` line holding what the seam wrote |

The `provider-emulated-caller-allow` and `-deny` vectors assert that the line the seam writes for the decision is
byte for byte the answer line the loop was fed, so the replayed answer is one the loop is known to
have accepted.

## Provenance

| fact | value |
|---|---|
| captured | 2026-10-28 |
| binary | `b10x-harness 0.22.0`, the released archive `b10x-harness-0.22.0-x86_64-unknown-linux-gnu.tar.gz` (sha256 `7db31b78b0c23a988d32697355a721c4f347d5d987d6ed7c2e48e285b178d3ce`, matching the release's `SHA256SUMS`), executable sha256 `8e94916f5ccc7cdb69cbcd33f21896a0559598b377dc1d23556d1b3e68eb49af`, commit `3eca074aea09574775ed340bb618e5322acaaf22` (annotated tag `0.22.0`) |
| endpoint | harness's `crates/harness-cli/tests/fixtures/fake_responses.py` at that commit, `flat-write` scenario (one `file_write` of `note.md`), bound to `127.0.0.1` inside a network namespace whose only interface is `lo` |
| run shape | as the golden's (`../golden/README.md`), plus `--approve caller --approve-up-to low --substrate-embedded` and the input `write a note`; stdin is the answer file |
| outcome | both runs exit 0 and the emulator recorded `has_authorization: false` on both requests. Under `allow` the loop wrote `note.md` and its `approval-resolved` says `approved: true`; under `deny` no `note.md` exists, `approved: false`, and the deny reason reached the model in the second request |
| reviewed | no credential, account identifier, machine path or endpoint address occurs in the committed bytes |

## Recording it again

Follow `../golden/README.md` § *Recording it again* with three changes: the scenario is
`flat-write`, the `run` adds `--approve caller --approve-up-to low --substrate-embedded` with
`--input "write a note"`, and its stdin is the answer file. Then regenerate the expected streams
after reviewing a deliberate mapping change:

```console
cargo test -p metaharness-b10x regenerate_the_caller_expectations -- --ignored
```
