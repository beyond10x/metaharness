---
format: aep.planning-md/1
id: story:pinned-plugin-loads-in-the-session
kind: story
status: implemented
title: A pinned plugin loads in the session, the 2.1.259 wire is read, and the cap acts during the run
summary: 'Also pass --plugin-dir for each --plugin pin (Q19 closed: the registry alone loads nothing under H2); recognise 2.1.259''s task_* and tool_progress records; pass --max-budget-usd through.'
owner: metaharness
tags:
- eval
- hermetic
relations:
- decomposes: epic:runs-side-by-side
revision: 4
---
# Story: a pinned plugin loads in the session, the 2.1.259 wire is read, and the cap acts during the run

## Outcome

A run that declares `--plugin <repo>@<name>@<pin>` opens with that plugin in `session.started.plugins`; a stream from Claude Code 2.1.259 carries no `opaque` line for the vendor's own bookkeeping; and a `--max-budget-usd` the caller states is the session's own stop.

## Context

One paid run — the agentplugins golden-path case, 2026-09-03, 1,515 events, $10.96 — showed all three at once. Two pinned plugins were placed in the scratch registry and not loaded (probe Q19, answered *no*: enablement lives in the user settings source that `--setting-sources ""` removes). 183 lines were `system/task_*` and `tool_progress` records the 2.1.241 reader had never seen, and every `tool.absent` row read over them came back `unk`. The runner's `--budget-usd 5` stopped nothing.

## Acceptance

- Each pinned marketplace plugin is also named to the vendor with `--plugin-dir <config home>/plugins/cache/<mkt>/<name>/<pin>`; `InstalledPlugin::loaded_by`, the H1a row and the C1 vector `c1-marketplace-plugin` say so.
- `system/task_started`, `system/task_progress`, `system/task_notification`, `system/task_updated` and `tool_progress` emit nothing and are not `opaque`; an unnamed subtype still is. Pin `2.1.241 → 2.1.259`.
- `metaharness run claude --max-budget-usd <USD>` is passed through as written; codex refuses it by name.
- Verified by `task check` on the release commit and, once recorded, by the golden-path stream's `session.started.plugins` listing three plugins with `opaque` = 0.
