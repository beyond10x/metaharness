---
format: aep.planning-md/3
id: story:website-drops-npm-leftovers
kind: story
status: draft
title: The website directory carries no package-manager leftovers
revision: 1
---
# Story: the website directory carries no package-manager leftovers

## Scheduling

Draft, filed 2026-10-21 from the repository review of that date (finding 9). Small; fits any wave.

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
