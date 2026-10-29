---
format: aep.planning-md/3
id: story:codex-catalogue-live-follow-ups
kind: story
status: implemented
title: Drive what the Codex catalogue route left unverified
summary: Agents and sleep switches on a real login, 0.153.4 on a login, request_user_input_async, and three uncovered checks
owner: metaharness
relations:
- decomposes: epic:loom-agent-work-through-metaharness
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-29T13:34:27Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-29T13:34:27Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-29T13:34:27Z", actor: "human:timo", revision: 5, decided_on: {"asserted":{"test_result":1,"verification":6}}}
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

## Delivered (D209, 2026-10-29)

`approval-record:d209-codex-live-follow-ups`. Evidence: `docs/research/2026-10-29-codex-catalogue-live-follow-ups.md`,
design amendment a67. Six live runs of the released 0.24.0 binary on the ChatGPT-subscription login, three at
0.153.4 (`gpt-6-astra`) and three at 0.160.0 (`gpt-6.1-sol`), all exit 0.

- Item 1: with the two switches, no request offered `clock` or `collaboration`, at either pin. The model request
  held `functions` (`exec`, `wait`, `request_user_input_async`) and nothing else beside the catalogue, which
  `exec` reaches as `fs_read` and `fs_stat`.
- Item 2: 0.153.4 behaves as 0.160.0 on a login; no `skills` tool; the catalogue call served and decided by the
  frame.
- Item 3: `request_user_input_async` sends no server request, posts an async `agentMessage` and answers
  `{"accepted":true}`; the turn goes on and the run ends normally.
- `exec` probe (D209): every file, process and network route the model tried from inside `exec` failed, at both
  pins; `strace -f` showed no extra program, no connection beyond the vendor's endpoints and no write outside the
  scratch home. No security finding; the route stays admitted.
- Item 4: the out-of-catalogue name and the schema parse are recorded as unchanged with their reasons (a67 § 5).
  Duplicate names were already refused on Claude and Codex, as `catalogue-malformed`; a test pins it and the
  pages that named `catalogue-name-duplicate` are corrected.
