---
format: aep.planning-md/3
id: release-plan:release-0-26-2
kind: release-plan
status: draft
title: 'Publish Metaharness 0.26.2: release executables pass the artifact scan'
summary: 0.25.0, 0.26.0 and 0.26.1 were tagged but never published
relations:
- supersedes: release-plan:release-0-26-1
revision: 1
---
## Why this release

`0.26.1`'s Release run was green, but its Linux executables failed `b10x-gates scan-artifact`, so
it was not published. `0.25.0` was not published either: it was the first release to compile C
sources (tree-sitter, under `metaharness-semantic`). Two causes:

- **personal-paths:** the C sources' file names kept the runner's home path, because
  `--remap-path-prefix` applies to Rust only.
- **private-identifiers:** a forbidden-literal rule matched a mangled symbol name in the
  unstripped symbol table.

## Change

`.github/workflows/release.yml` adds `CFLAGS="-ffile-prefix-map=$HOME=/build
-ffile-prefix-map=$GITHUB_WORKSPACE=/src"` and `-C strip=symbols` to `RUSTFLAGS`. Before the tag
is written, a local release build made with the same flags passes `scan-artifact`.

## Constraint (b10x-chef-orchestrator, 2026-10-30)

The literal the private-identifiers rule matched must not appear in any commit message, PR body,
CHANGELOG or release note. Write "a forbidden-literal rule matched a mangled symbol".

## Procedure

As in `release-plan:release-0-26-0`: one PR through the bot; after CI, the bot fast-forwards
`main` and pushes the annotated tag `0.26.2`; the artifacts are checked and scanned, then the bot
publishes the release. Tags `0.26.0` and `0.26.1` stay unpublished where they are.
