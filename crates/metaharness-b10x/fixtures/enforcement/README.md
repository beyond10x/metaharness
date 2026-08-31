# b10x enforcement excerpts

These are selected, unedited JSON lines captured from `b10x-harness 0.9.1` against harness's
deterministic local Responses endpoint on 2026-08-31. They are `provider_emulated` evidence, not a
claim about a live provider.

`unpublished.jsonl` used the `unpublished-tool` scenario over a read-only workspace. The model
asked for `shell.exec`, a name the run never published; the loop refused it and recovered.

`approval-denied.jsonl` used the `flat-write` scenario over an adoptable `ws_` workspace with
embedded substrate and `--approve deny`. The write was published, reached the approval gate, was
denied, and returned a failed outcome to the model. `note.md` was absent after the run.

Only the enforcement-bearing lines are retained. This avoids pinning timestamps and context
digests which do not participate in the mapping under test; every retained line is byte-for-byte
from the released binary's output.
