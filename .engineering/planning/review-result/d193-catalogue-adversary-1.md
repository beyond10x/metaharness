---
format: aep.planning-md/3
id: review-result:d193-catalogue-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on story:codex-catalogue-on-app-server
relations:
- reviews: story:codex-catalogue-on-app-server
revision: 1
---
unit: story:codex-catalogue-on-app-server, commit 8ae85af (merged f21d117 into integration/d193); cases da9a9bf
verdict: NEEDS-CHANGE (six red cases)
cases: executed 284→290, red 6
origin: introduced 5 / pre-existing 1
wrote-outside-worktree: wave scratch only

| id | severity | finding | red case |
|---|---|---|---|
| F1 | major | `offered_tools` comes from `dynamic_tools` while the same record shows `exec` called (invariant 3) | `every_tool_the_record_shows_called_is_among_the_offered_tools` |
| F2 | major, pre-existing | a code-mode `exec` cell is recorded with `input: ""`; `rollout.rs` reads only `arguments` | `a_code_mode_cell_keeps_its_script_in_the_tool_requested_input` |
| F3 | minor | stdin closes on `turn/completed` from any thread | `a_turn_completed_on_another_thread_does_not_close_the_input` |
| F4 | minor | an answer is retained as sent before the write to the child | `an_answer_to_a_child_that_died_mid_call_is_not_retained_as_sent` |
| F5 | minor | a reused request id can match one call's allow to another call's arguments | `a_reused_request_id_never_performs_a_denied_calls_arguments` |
| F6 | minor | catalogue names `mcp` / `mcp__*` pass the check that 0.160.0 refuses at `thread/start` | `a_catalogue_codex_refuses_at_thread_start_is_refused_before_the_spawn` |

All six go to the fix unit `unit/d193-catalogue-fixes` in the same wave.
