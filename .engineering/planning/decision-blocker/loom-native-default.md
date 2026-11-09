---
format: aep.planning-md/3
id: decision-blocker:loom-native-default
kind: decision-blocker
status: open
title: Nobody has decided what making Loom the native default changes
relations:
- blocks: story:loom-native-default
revision: 1
---
## The question

Issue 28 asks that the Loom adapter become "the native default" so that a consumer selecting the
`b10x` kind runs on Loom without changing its own code. Nobody has decided what that changes.
Decides: the operator.

## Facts, read 2026-11-09 at `origin/main` `bff6a66`

- There is no default kind: `metaharness run <kind>` requires it
  (`crates/metaharness-protocol/src/spec.rs:19`).
- A driven run's native arm is the kind its AEP step map names, `b10x`
  (`crates/metaharness-aep/src/drive.rs:1618`, `:2026`).
- `b10x-loom run <intent>` takes an intent and runs Loom's own slice
  (`b10x-loom run --help`, loom `crates/loom-cli/src/main.rs:907`); it takes none of the b10x loop's
  launch arguments (`--write-scope`, `--allow-program`, the hook file; `drive.rs:359`, `:1420`,
  `:1728`).

## Options

| | option | what it changes |
|---|---|---|
| A | the name `b10x` resolves to Loom | every existing `b10x` step map and every caller of `metaharness run b10x` gets another loop; the b10x loop needs a new kind name to stay selectable; AEP step maps and the frame are bytes another repository verifies |
| B | a new kind `loom`, and the consumers' step maps move to it | no caller changes silently; the consumer changes its own configuration, which the issue asks to avoid |
| C | `loom` stays a selectable kind and nothing is a default | the issue's second ask is declined |

## Clears when

The operator records which option holds, here or in an approval record. A and B need an ADR in
Atlas under the cross-repository rule before code.
