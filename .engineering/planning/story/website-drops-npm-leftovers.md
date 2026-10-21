---
format: aep.planning-md/3
id: story:website-drops-npm-leftovers
kind: story
status: implemented
title: The website directory carries no package-manager leftovers
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T12:00:26Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-21T12:00:26Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-21T18:43:46Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"ess_conformance_coverage_v1":1}}}
---
# Story: the website directory carries no package-manager leftovers

## Scheduling

Wave 5 unit 6 (`approval-record:adr-0096-wave-5`), filed 2026-10-21 from the repository review of that date
(finding 9).

## Outcome

`website/` holds only what its README describes: static HTML, local CSS and docs, with no npm or
Docusaurus configuration.

## Context

- `website/README.md:5`: "There is no JavaScript runtime, package manager or network font."
- `website/.npmrc` (`allow-git=root`) and `website/.gitignore` (Docusaurus entries: `/node_modules`,
  `.docusaurus`, `npm-debug.log*`) are still tracked.

## Acceptance

- `git ls-files website` lists no `.npmrc` and no `.gitignore` with Docusaurus or npm entries.
- The Pages build still publishes the same files.

## Delivered

Commit `7b96b12`, merged `83d15d0` into `integration/adr-0096-wave-5`; no adversary pass, by the
coordinator's decision (two deleted configuration files, build output identical); coordinator gate 82 test
binaries, 1014 passed, 0 failed, 49 ignored, conformance 103/103. `website/.npmrc` (`allow-git=root`) is
removed; `website/.gitignore` keeps only `/build`, the directory the Rust docs builder writes. The docs build
at the base and after the change emits the same 19 files with the same sha256. A test in
`crates/metaharness-docs/tests/website_tree.rs` fails if `.npmrc` returns or `.gitignore` gains any entry
other than `/build`.

Whole gate at the wave-6 head: task check, one step at a time, on `integration/adr-0096-wave-6` at
`5ddebbd`: 6 steps exit 0, 96 test binaries, 1128 passed, 0 failed, 49 ignored, conformance 103/103 on ESS
0.67.0.

Deviations: the acceptance asked for no `.gitignore` with Docusaurus or npm entries; `.gitignore` is kept
with `/build` only rather than deleted.
