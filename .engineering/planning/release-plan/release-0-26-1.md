---
format: aep.planning-md/3
id: release-plan:release-0-26-1
kind: release-plan
status: draft
title: 'Publish Metaharness 0.26.1: 0.26.0 with the macOS build fixed'
summary: the 0.26.0 tag failed to compile on macOS and was never published
relations:
- supersedes: release-plan:release-0-26-0
revision: 1
---
## Why this release

The `0.26.0` tag's Release run (41127186453) failed to compile `metaharness` on both macOS
targets. `foreign_entry` read std's raw `st_mode`, a `u32`, into
`rustix::fs::FileType::from_raw_mode`, which takes a `u16` on macOS. `0.26.0` was never published,
and its tag stays where it is: tags are not moved.

`0.26.1` is `0.26.0` with the entry kind read from std's file type, so `0.26.1` carries everything
`0.26.0` announced.

## Procedure

As in `release-plan:release-0-26-0`: one PR through the bot; after CI, the bot fast-forwards
`main` and pushes the annotated tag `0.26.1`; the Release run's artifacts are checked and scanned,
then the bot publishes the release.
