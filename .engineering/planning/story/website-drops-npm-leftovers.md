---
format: aep.planning-md/3
id: story:website-drops-npm-leftovers
kind: story
status: active
title: The website directory carries no package-manager leftovers
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T12:00:26Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-21T12:00:26Z", actor: "human:timo", revision: 4}
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
