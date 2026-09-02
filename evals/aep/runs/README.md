# The two recorded runs, as this driver's own wire — and what they are not

The fixture `epic:runs-side-by-side` names: two runs, in one format, read beside each other.

| file | what it is |
|---|---|
| `decomposer-clean.events.jsonl` | `checks/transcripts/decomposer-clean.jsonl`, through this driver's Claude transcript reader |
| `plan-reviewer-clean.events.jsonl` | `checks/transcripts/plan-reviewer-clean.jsonl`, the same |
| `*.trace-ir.json` | each of those, through `metaharness project` |
| `side-by-side.html` | both, through `metaharness project --html` |

## What these are not

**Nothing here came from a model.** The two source transcripts are hand-written — their own
`checks/transcripts/README.md` says so out loud: *"nothing here came from a model, and no claim in
this repository rests on one of these files describing a real run"* — and that label travels with
everything derived from them. The numbers in them are plausible rather than measured. No bound in
this repository is calibrated against one, and no claim about a harness, a cost or a latency rests
on one.

What they *are* is a **reading fixture**: two runs whose event streams differ in a way a reader has
to be able to see, cheaply, with no credential and no paid run.

## They are derived, and the derivation is checked

Every file here is regenerated from its source and compared byte for byte by the ordinary gate:

| check | where |
|---|---|
| the event streams are what the transcript reader produces | `crates/metaharness-claude/tests/recorded_runs.rs` |
| the documents and the page are what `project` produces | `crates/metaharness-cli/tests/project.rs` |

Both carry an `#[ignore]`d regeneration test beside the check. A change here is deliberate, and the
diff is the review.

**This does not put an eval in the gate** (`AGENTS.md` invariant 5). That invariant is about a
**paid run** never gating; these tests read committed files, convert them in memory and compare
bytes. Nothing under `evals/` is executed.

## What the consumer says about them

```console
$ aep trace check --spec ../expectations.projection.trace.yaml \
    --transcript decomposer-clean.events.jsonl
metaharness/projection against transcript sha256:90a4bcfdf5cb… — 23 ok, 0 gap, 0 unk
```

Both streams, `aep` 0.42.0, exit 0. That is the acceptance line of `story:trace-ir-reader`, in the
form the consumer supports — `aep trace check` reads a `metaharness.event/1` stream and has no
`trace-ir/1` reader (`docs/design/runs-side-by-side-v0.1.md` P4).

**One row is deliberately absent from that specification and the reason is a finding.** A
`tool.failed` bound reads `is_error`; the real Claude Code wire carries it explicitly
(`crates/metaharness-claude/fixtures/golden/transcript.jsonl`, captured from 2.1.240, has
`"is_error": false`) and these hand-written inputs omit it. The two readers then answer differently
about the same run — `claude-code/stream-json` says `ok`, `metaharness/event-stream` says `unk` —
and the second is right: an absent field is `null` and never `false`. The row is left out rather
than the reader changed.
