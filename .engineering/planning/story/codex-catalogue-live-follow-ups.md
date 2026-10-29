---
format: aep.planning-md/3
id: story:codex-catalogue-live-follow-ups
kind: story
status: draft
title: Drive what the Codex catalogue route left unverified
summary: Agents and sleep switches on a real login, 0.153.4 on a login, request_user_input_async, and three uncovered checks
owner: metaharness
relations:
- decomposes: epic:loom-agent-work-through-metaharness
revision: 1
---
## Outcome

What the Codex catalogue route (amendment a66) left unverified is either driven on a real login or
labelled unverified with a reason.

## Items

- On a real login, `[agents] enabled = false` and `sleep_tool = false` remove the multi-agent and clock
  tools. They were checked against a fake model server only, after the one live probe of D193.
- Codex 0.153.4 on the catalogue route on a login. Only fake-server runs exist, and that version needs
  `[orchestrator.skills] enabled = false`.
- What a call to `request_user_input_async` does in a headless app-server session.
- From the fix unit: a refused non-catalogue `item/tool/call` name is not reflected in `offered_tools`;
  the vendor's input-schema check at `thread/start` is not repeated before the spawn; duplicate entry
  names are not refused on the Claude route.

## Acceptance

One live run on the subscription login per pinned version, recorded as the research note was. It shows
the model request's tool list with only the catalogue plus the three declared code-mode tools. Each
item above is fixed, or recorded as unverified with its reason.
