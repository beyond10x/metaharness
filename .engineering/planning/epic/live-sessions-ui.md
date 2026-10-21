---
format: aep.planning-md/3
id: epic:live-sessions-ui
kind: epic
status: draft
title: Every agent session on this machine, live, in one browser page
summary: A loopback web page that lists every running and recorded agent session and renders a selected one's metaharness events as HTML as they arrive
tags:
- live
- ui
relations:
- informed_by: epic:runs-side-by-side
revision: 6
---
# Epic: Every agent session on this machine, live, in one browser page

## Outcome

A person opens one local page and sees every agent session on the machine as it runs: which are
live, waiting, failed or ended, with harness, model, repository, step, turns, tokens, cost and
denials. Selecting one shows its events as they arrive, rendered as HTML (text, thinking, tool
calls with their decision and result, usage, warnings), and its session metadata (tools offered and
withheld, permission mode, hermetic attestation, context files). Every session renders the same way
whatever ran it, because the page reads metaharness `Event`s only.

## Why Now

The operator asked for it (2026-10-21): "a web-based UI around meta-harness - show all sessions
live - allow to select them and see details (events rendered in html)". Today the only reading
surface is `story:event-stream-viewer`, a static page over one or two recorded `trace-ir/1`
documents, which lists "Live rendering of a running session" under Out of Scope. A running session
is visible only as the JSON lines `metaharness run` writes to its own stdout (`metaharness --help`,
0.9.3), so nobody but the embedder can watch it.

## Scope

- A local server, `metaharness serve`, bound to `127.0.0.1`, in Rust, that lists sessions and
  pushes their events to the browser as they arrive.
- Two session sources, each normalised to `Event` (crates/metaharness-protocol/src/event.rs):
  - sessions metaharness itself drives (`metaharness run`, the library behind takt and
    `metaharness aep drive`), which write their event stream to a per-session spool beside a lease
    that says whether the process is still alive. No spool exists today (`git grep -ic spool` over
    `crates/` at 1634016: no match);
  - recorded vendor sessions it does not drive, read through the existing readers:
    `crates/metaharness-claude/src/transcript.rs` (Claude Code transcripts) and
    `crates/metaharness-codex/src/rollout.rs` (`$CODEX_HOME/sessions/…/rollout-*.jsonl`).
- A session list and a session detail page, laid out as the design the operator picks (§ Design).
- The page's contract with the server is read-only Server-Sent Events (operator, 2026-10-21: "it
  would connect to "some" or "more" streams SSE for read-only"). The page opens one or more SSE
  streams, for example one for the session list and one per session it shows, and sends nothing
  back beyond the HTTP requests that open them. Each stream's event names and payloads are part of
  the contract, declared in `spec/` and generated for the page.

## Out of Scope

- Steering from the page (`Command`: tool_decide, message_inject, steer, interrupt, halt). The
  page is read-only; steering is a later milestone with its own authority story, and it may move
  the page to a WebSocket for two-way traffic (operator, 2026-10-21: "later we might make it
  websocket for 2 way communication"). Nothing in this epic depends on that move.
- Other hosts, remote access, and any authentication beyond binding to the loopback interface.
- Storing anything beyond the spool: no database, no retention policy beyond the vendor's own.
- Comparing two runs: that stays with `epic:runs-side-by-side`.

## Design

Mockups on mock data with a simulated live feed: `~/.cache/b10x-layers/metaharness-ui-designs/`
(`index.html` compares them; local only, not committed). The operator found A, B and C each
right for a different question ("we just have different views", 2026-10-21) and asked for them
combined; D does that.

| part of D | from |
|---|---|
| overview of all sessions, switchable between a wall of tiles and a sortable table | B, C |
| focus on one session: a rail of all sessions grouped by status | A |
| detail tab Transcript: text, thinking, tool cards with decision and result | A |
| detail tab Timeline: swimlanes over time (model, tools, decisions, meta) above the event log | B |
| detail tab Trace: waterfall of step, turn and tool call with durations | C |
| inspector: the session's counters and metadata, or the selected event rendered or as JSON | A, C |
| one event selection shared by the three tabs; keys 1/2/3, j/k, Esc; state in the URL | new |
| dark and light themes | new |

Pick: D. The operator confirmed it on 2026-10-21: "design looks good".

The browser side is Vue (operator, 2026-10-21: "browser side of things should just be vue"),
built the way `uilab/widget/` is: Vue 3, TypeScript, Vite, `vue-tsc` and pnpm. Only the page
build is Node; `metaharness serve` and everything it reads stay Rust. The event types the page
uses are generated from `metaharness-protocol` by a Rust crate and drift-checked by `task check`,
never written by hand.

## Not yet

Parked until the sessions on this machine run through metaharness (operator, 2026-10-21: "we do
not yet implement that, we first have to create basically some agent/agentsdk/takt/etc or even
the conductor rewrite"). Until then the epic stays a draft: no ESS domain, no stories.

Direction the operator sketched, as brainstorming and not decided ("just brainstorming here"):

- everything goes through metaharness, so every session on the machine is a metaharness session;
- a metaharness process started with `--serve` exposes its own session over SSE, or later a
  WebSocket;
- the page then finds sessions through some form of discovery, and possibly federation across
  hosts.

If that direction holds, the spool and the vendor-transcript readers in § Scope may not be needed,
and § Scope is rewritten when the epic is taken up.

## Before decomposition

- Guardrail 7: the epic introduces nouns no `spec/` domain declares — a live session (the spool
  entry and its lease) and its source. Draft `spec/domains/observatory.yaml` and validate it with
  `ess specify validate --path spec` before any story is written. Cardinalities not read from code
  are `UNMAPPED:`.

## Open Questions

- Whether a recorded vendor session that metaharness did not drive counts as "live" while its file
  still grows, or only as recorded. Default: live while the file grew in the last 60 s.
- Transcripts carry everything a session saw. Loopback-only binding is the only control in scope;
  whether that is enough is the owner's call.
