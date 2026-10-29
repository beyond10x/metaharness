---
format: aep.planning-md/3
id: review-result:d193-catalogue-security-1
kind: review-result
status: active
title: Security review of story:codex-catalogue-on-app-server
relations:
- reviews: story:codex-catalogue-on-app-server
revision: 1
---
unit: story:codex-catalogue-on-app-server, commit 8ae85af (merged f21d117 into integration/d193); cases 6151c90
verdict: NEEDS-CHANGE (attestation), no effect outside the catalogue
cases: executed 284→287, red 2

Central question: can the model act outside the catalogue on the app-server route? No, from Codex source at
`rust-v0.160.0`. `exec` runs in a separate V8 process that has no file, network or process access, and its
nested calls reach only the offered tools as `item/tool/call`. `request_user_input_async` posts a message and
returns. `wait` has no effect. Without an environment, no shell, patch or image tool is registered. Not
observed on a real login: the `[agents]` and `sleep_tool` switches. Per-call governance is intact.

| id | severity | finding | case |
|---|---|---|---|
| F1 | warning | H8 says every offered tool is a client tool beside `exec` and `wait` in `ambient_inputs` | red, `the_h8_row_does_not_claim_every_offered_tool_is_a_client_tool_beside_exec_and_wait` |
| F2 | warning | `offered_tools` omits `exec`, which the same record shows called | red, `every_tool_the_record_shows_called_is_among_the_offered_tools_when_they_are_stated` |
| F3 | cleared | the copied `auth.json` is reachable by no catalogue call, event or retained file; mode 0600 | green |
| N1 | note | a `--cwd` containing `$TMPDIR` would put the copied `auth.json` inside the file root; same as Claude's owned route; nothing found reaches it | judgement |
| N2 | note | AGENTS.md invariant 2 names two spawn runners; the unit adds a third Codex one | judgement |

Coordinator decision: code mode is admitted, because no effect leaves the catalogue. `offered_tools` and H8 must
state the vendor's code-mode tools. Invariant 2 names the app-server runner. All of this goes to the fix unit.
