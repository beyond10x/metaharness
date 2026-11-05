---
format: aep.planning-md/3
id: review-result:wave-1105-u1-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on story:aep-version-reads-bounded
relations:
- reviews: story:aep-version-reads-bounded
- reviews: story:doctor-run-help-probe-bounded
revision: 1
---
unit: story:aep-version-reads-bounded, story:doctor-run-help-probe-bounded, impl/bounded-probes at dd7d2a1 (merged 71e61d4); cases 07b63f5
verdict: CONFIRMED (notes only; no red case)
cases: executed 281→290, red 0
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: wave scratch only

The probe held against an endless writer, the cap boundary, a closed stdout, a flooded stderr and a
grandchild holding the pipes; doctor b10x through the binary at the 60 s deadline; real doctor over the
installed help and a real aep drive with aep 0.79.0.

All three notes are closed: the cases are merged (8c8f5c1, the help-read case catches a dropped stdout),
a65 point 4 now names only the --version and run --help reads and the two unbounded aep-eval reads, and
the CLI reference doctor row states the 60 s wait and the 64 KiB cap.

```findings
[
{"file":"docs/design/metaharness-protocol-v0.1.md","line":5567,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"a65 point 4 claims every answer-only child goes through the bounded probe, while aep-eval git rev-parse HEAD (lib.rs:1371) and id -u (lib.rs:412) still read through the unbounded output() helper"},
{"file":"crates/metaharness/src/doctor.rs","line":791,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the unit help-read case expects every flag Unknown from a help declaring none, which an empty read also produces, so it cannot catch a dropped probe stdout"},
{"file":"website/docs/reference/cli.md","line":23,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the 64 KiB run --help cap and the 60 s probe deadline that doctor b10x now refuses on are not on the operator site, only in the CHANGELOG and the refusal text"}
]
```
