//! Source-backed terminal and tool observations never infer evidence from prose.
use metaharness_codex::{CodexSeam, RolloutReader, hook_request_line};
use metaharness_protocol::{HarnessSeam, HermeticAttestation, HermeticMode, Seam, TranscriptRef};
use serde_json::{Value, json};

fn read(records: Vec<(&str, Value)>) -> Vec<Value> {
    let mut reader = RolloutReader::new(
        TranscriptRef {
            path: None,
            digest: None,
            bytes: None,
        },
        HermeticAttestation::none(HermeticMode::Off),
    );
    let mut events = Vec::new();
    for (kind, payload) in records {
        events.extend(
            reader
                .push_line(&json!({"type":kind,"payload":payload}).to_string())
                .into_iter()
                .map(|e| serde_json::to_value(e.event).unwrap()),
        );
    }
    events.extend(
        reader
            .finish()
            .into_iter()
            .map(|e| serde_json::to_value(e.event).unwrap()),
    );
    assert!(
        reader.finish().is_empty(),
        "terminal aggregate is emitted once"
    );
    events
}
fn ended(events: &[Value]) -> &Value {
    events
        .iter()
        .find(|e| e["event"] == "session.ended")
        .unwrap()
}
fn result(events: &[Value]) -> &Value {
    events
        .iter()
        .rev()
        .find(|e| e["event"] == "tool.result")
        .expect("normalized result")
}
fn call(name: &str) -> (&'static str, Value) {
    (
        "response_item",
        json!({"type":"function_call","call_id":"call-1","name":name,"arguments":"{}"}),
    )
}
fn completion(error: Value) -> (&'static str, Value) {
    let mut payload =
        json!({"type":"task_complete","turn_id":"turn-1","last_agent_message":"final"});
    payload["error"] = error;
    ("event_msg", payload)
}

#[test]
fn commentary_and_duplicate_representations_do_not_choose_the_final_answer() {
    let events = read(vec![
        (
            "response_item",
            json!({"type":"message","role":"assistant","content":[{"text":"plausible but intermediate"}]}),
        ),
        (
            "event_msg",
            json!({"type":"agent_message","message":"final","phase":"final_answer"}),
        ),
        (
            "response_item",
            json!({"type":"message","role":"assistant","content":[{"text":"final"}]}),
        ),
        completion(Value::Null),
    ]);
    assert_eq!(
        ended(&events)["final_answer"],
        json!({"text":"final","turn_id":"turn-1"})
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| e["event"] == "session.ended")
            .count(),
        1
    );
}
#[test]
fn missing_final_answer_and_terminal_error_cannot_promote_partial_text() {
    for terminal in [
        json!({"type":"task_complete","error":null}),
        json!({"type":"task_complete","last_agent_message":"partial","error":{"message":"failed"}}),
    ] {
        let events = read(vec![
            (
                "event_msg",
                json!({"type":"agent_message","message":"partial"}),
            ),
            ("event_msg", terminal),
        ]);
        assert!(ended(&events)["final_answer"].is_null());
    }
}
#[test]
fn model_selections_keep_turn_scope_changes_and_missing_fields_without_prices() {
    let events = read(vec![
        ("event_msg", json!({"type":"task_started","turn_id":"one"})),
        (
            "turn_context",
            json!({"turn_id":"one","model":"observed-a"}),
        ),
        (
            "turn_context",
            json!({"turn_id":"one","model":"observed-a"}),
        ),
        ("event_msg", json!({"type":"task_started","turn_id":"two"})),
        (
            "turn_context",
            json!({"turn_id":"two","model":"observed-b"}),
        ),
        (
            "event_msg",
            json!({"type":"task_started","turn_id":"three"}),
        ),
        ("turn_context", json!({"turn_id":"three"})),
        completion(Value::Null),
    ]);
    let terminal = ended(&events);
    let observed = terminal["observed_models"]
        .as_array()
        .expect("observed selections");
    assert_eq!(observed.len(), 3);
    for (index, model) in [json!("observed-a"), json!("observed-b"), Value::Null]
        .iter()
        .enumerate()
    {
        assert_eq!(&observed[index]["model"], model);
        assert_eq!(observed[index]["turn"], index + 1);
        assert_eq!(observed[index]["scope"], "turn_selection");
    }
    assert_eq!(observed[0]["turn_id"], "one");
    assert_eq!(observed[1]["turn_id"], "two");
    assert!(terminal["total_cost_usd"].is_null());
}
fn tool_item(status: &str, code: Value, id: &str) -> (&'static str, Value) {
    let mut payload = json!({"type":"item_completed","turn_id":"turn-1","item":{"type":"CommandExecution","id":id,"status":status,"aggregated_output":"not parsed for status"}});
    payload["item"]["exit_code"] = code;
    ("event_msg", payload)
}
#[test]
fn structured_command_success_failure_and_denial_are_correlated() {
    for (status, code, error) in [
        ("completed", json!(0), false),
        ("failed", json!(9), true),
        ("declined", Value::Null, true),
    ] {
        let events = read(vec![
            call("exec_command"),
            tool_item(status, code.clone(), "call-1"),
        ]);
        assert_eq!(result(&events)["is_error"], error);
        assert_eq!(result(&events)["exit_code"], code);
        assert_eq!(
            result(&events)["outcome_source"],
            "retained_tool_completion"
        );
        assert!(
            !events.iter().any(|e| e["event"] == "tool.decided"),
            "outcome never invents authorization"
        );
    }
}
#[test]
fn unmatched_contradictory_and_incomplete_commands_never_become_success() {
    let unmatched = read(vec![tool_item("completed", json!(0), "missing")]);
    assert!(result(&unmatched)["is_error"].is_null());
    assert!(result(&unmatched)["exit_code"].is_null());
    assert!(unmatched.iter().any(|e| e["event"] == "warning"
        && e["code"] == "TOOL_OUTCOME_UNVERIFIED"
        && e["message"] == "completion has no matching observed call"));
    let conflicting = read(vec![
        call("exec_command"),
        tool_item("completed", json!(0), "call-1"),
        tool_item("failed", json!(9), "call-1"),
        tool_item("completed", json!(0), "call-1"),
    ]);
    assert!(result(&conflicting)["is_error"].is_null());
    assert!(conflicting.iter().any(|e| e["event"] == "warning"));
    let incomplete = read(vec![
        call("exec_command"),
        (
            "response_item",
            json!({"type":"function_call_output","call_id":"call-1","output":"Process exited with code 0"}),
        ),
    ]);
    assert!(result(&incomplete)["is_error"].is_null());
    assert!(result(&incomplete)["exit_code"].is_null());
}
#[test]
fn legacy_patch_status_is_observed_without_fabricating_an_exit_code() {
    let events = read(vec![
        call("apply_patch"),
        (
            "event_msg",
            json!({"type":"patch_apply_end","call_id":"call-1","status":"completed","success":true,"stdout":"applied"}),
        ),
    ]);
    assert_eq!(result(&events)["is_error"], false);
    assert!(result(&events)["exit_code"].is_null());
}

#[test]
fn content_only_updates_keep_evidence_but_wrong_turn_or_family_invalidates_it() {
    let output = (
        "response_item",
        json!({"type":"function_call_output","call_id":"call-1","output":"new content with no outcome claim"}),
    );
    let events = read(vec![
        call("exec_command"),
        tool_item("completed", json!(0), "call-1"),
        output.clone(),
    ]);
    assert_eq!(result(&events)["exit_code"], 0);
    assert_eq!(result(&events)["is_error"], false);
    let events = read(vec![
        (
            "event_msg",
            json!({"type":"task_started","turn_id":"different-turn"}),
        ),
        call("exec_command"),
        tool_item("completed", json!(0), "call-1"),
        output.clone(),
    ]);
    assert!(result(&events)["is_error"].is_null());
    let events = read(vec![
        call("apply_patch"),
        tool_item("completed", json!(0), "call-1"),
        output,
    ]);
    assert!(result(&events)["is_error"].is_null());
}
#[test]
fn malformed_or_inconsistent_exit_metadata_never_claims_success() {
    for (status, code) in [
        ("completed", Value::Null),
        ("completed", json!(7)),
        ("failed", json!(0)),
        ("completed", json!(4_294_967_296_u64)),
        ("in_progress", json!(0)),
    ] {
        let events = read(vec![
            call("exec_command"),
            tool_item(status, code, "call-1"),
        ]);
        assert!(result(&events)["is_error"].is_null());
        assert!(result(&events)["exit_code"].is_null());
    }
}
#[test]
fn unfinished_new_turn_never_reuses_a_previous_final_answer() {
    let events = read(vec![
        completion(Value::Null),
        ("event_msg", json!({"type":"task_started","turn_id":"new"})),
    ]);
    assert!(!events.iter().any(|event| event["event"] == "session.ended"));
}

#[test]
fn reused_call_id_with_another_tool_invalidates_the_last_public_outcome() {
    let events = read(vec![
        call("exec_command"),
        tool_item("completed", json!(0), "call-1"),
        call("apply_patch"),
    ]);
    assert!(
        result(&events)["is_error"].is_null(),
        "internal invalidation must also reach normalized consumers"
    );
    assert!(events.iter().any(|event| event["event"] == "warning"));
}

/// What the seam reads on a 0.153.4 run for commands run through the code-mode `exec` tool
/// (amendment a47): rollout lines and hook requests, merged. The shapes follow the pinned source
/// and the values are invented here; no line is copied from a retained session.
fn seamed(lines: Vec<String>) -> Vec<Value> {
    let mut seam = CodexSeam::new(
        TranscriptRef {
            path: None,
            digest: None,
            bytes: None,
        },
        HermeticAttestation::none(HermeticMode::Off),
        Seam::Hook,
    );
    let mut events = Vec::new();
    for line in lines {
        events.extend(
            seam.push_line(&line)
                .into_iter()
                .map(|e| serde_json::to_value(e.event).unwrap()),
        );
    }
    events.extend(
        seam.finish()
            .into_iter()
            .map(|e| serde_json::to_value(e.event).unwrap()),
    );
    events
}
fn rollout(kind: &str, payload: &Value) -> String {
    json!({"type":kind,"payload":payload}).to_string()
}
fn turn_started(turn: &str) -> String {
    rollout("event_msg", &json!({"type":"task_started","turn_id":turn}))
}
/// The outer call, as the rollout records it: the model's `call_*` id, the custom `exec` tool.
fn outer_exec() -> String {
    rollout(
        "response_item",
        &json!({"type":"custom_tool_call","status":"completed","call_id":"call_outer","name":"exec","input":"await tools.exec_command({\"cmd\":\"true\"});"}),
    )
}
/// A `PreToolUse` hook request for one command, under the rendezvous name `key`.
fn hook(key: &str, tool: &str, id: Option<&str>, turn: Option<&str>, command: &str) -> String {
    let mut raw = json!({"session_id":"s","cwd":"/scratch/work","hook_event_name":"PreToolUse","model":"a-model","permission_mode":"bypassPermissions","tool_name":tool,"tool_input":{"command":command}});
    if let Some(id) = id {
        raw["tool_use_id"] = json!(id);
    }
    if let Some(turn) = turn {
        raw["turn_id"] = json!(turn);
    }
    hook_request_line(key, &raw.to_string())
}
/// A command's `item_completed`, in the 0.153.4 shape.
fn command_completed(id: &str, turn: Option<&str>, status: &str, code: Value) -> String {
    let mut payload = json!({"type":"item_completed","thread_id":"s","item":{"type":"CommandExecution","id":id,"command":["/bin/bash","-lc","true"],"cwd":"file:///scratch/work","parsed_cmd":[],"source":"unified_exec_startup","status":status,"aggregated_output":""}});
    payload["item"]["exit_code"] = code;
    if let Some(turn) = turn {
        payload["turn_id"] = json!(turn);
    }
    rollout("event_msg", &payload)
}
fn result_of<'a>(events: &'a [Value], id: &str) -> &'a Value {
    events
        .iter()
        .rev()
        .find(|e| e["event"] == "tool.result" && e["call_id"] == id)
        .unwrap_or_else(|| panic!("no tool.result for {id}"))
}
fn unverified(events: &[Value]) -> Vec<&str> {
    events
        .iter()
        .filter(|e| e["event"] == "warning" && e["code"] == "TOOL_OUTCOME_UNVERIFIED")
        .map(|e| e["message"].as_str().unwrap_or_default())
        .collect()
}
/// The two lines of one mediated command: its hook request and its completion.
fn mediated(id: &str, command: &str, status: &str, code: Value) -> [String; 2] {
    [
        hook(id, "Bash", Some(id), Some("turn-1"), command),
        command_completed(id, Some("turn-1"), status, code),
    ]
}

/// Amendment a47: a command run through the code-mode `exec` tool completes under the `exec-` id
/// the hook request carried, in the hook's turn, and that id is what correlates it. Exit 0 and
/// exit 7 reach `tool.result` with no `TOOL_OUTCOME_UNVERIFIED`.
#[test]
fn a_mediated_command_exit_reaches_tool_result_through_the_hook_id() {
    let mut lines = vec![turn_started("turn-1"), outer_exec()];
    lines.extend(mediated("exec-zero", "true", "completed", json!(0)));
    lines.extend(mediated("exec-seven", "exit 7", "failed", json!(7)));
    let events = seamed(lines);
    let zero = result_of(&events, "exec-zero");
    assert_eq!(zero["exit_code"], 0);
    assert_eq!(zero["is_error"], false);
    assert_eq!(zero["outcome_source"], "retained_tool_completion");
    let seven = result_of(&events, "exec-seven");
    assert_eq!(seven["exit_code"], 7);
    assert_eq!(seven["is_error"], true);
    assert_eq!(seven["outcome_source"], "retained_tool_completion");
    assert_eq!(unverified(&events), Vec::<&str>::new());
}

/// The same completions read without the hook requests that named them, as a rollout read on its
/// own is: nothing in the rollout carries their id, so both stay unknown and say so.
#[test]
fn mediated_completions_without_their_hook_requests_stay_unknown() {
    let events = seamed(vec![
        turn_started("turn-1"),
        outer_exec(),
        command_completed("exec-zero", Some("turn-1"), "completed", json!(0)),
        command_completed("exec-seven", Some("turn-1"), "failed", json!(7)),
    ]);
    for id in ["exec-zero", "exec-seven"] {
        assert!(result_of(&events, id)["exit_code"].is_null());
        assert!(result_of(&events, id)["is_error"].is_null());
        assert!(result_of(&events, id)["outcome_source"].is_null());
    }
    assert_eq!(
        unverified(&events),
        [
            "completion has no matching observed call",
            "completion has no matching observed call"
        ]
    );
}

/// The join is the vendor's id and the vendor's turn id, both present and equal, on a `Bash` hook
/// that carried its own `tool_use_id`. Every other shape stays unknown and warns.
#[test]
fn a_mediated_completion_joins_only_on_the_vendors_id_and_turn() {
    let other_turn = seamed(vec![
        hook("k", "Bash", Some("exec-a"), Some("turn-1"), "true"),
        command_completed("exec-a", Some("turn-2"), "completed", json!(0)),
    ]);
    let no_completion_turn = seamed(vec![
        hook("k", "Bash", Some("exec-a"), Some("turn-1"), "true"),
        command_completed("exec-a", None, "completed", json!(0)),
    ]);
    let no_hook_turn = seamed(vec![
        hook("k", "Bash", Some("exec-a"), None, "true"),
        command_completed("exec-a", Some("turn-1"), "completed", json!(0)),
    ]);
    for events in [&other_turn, &no_completion_turn, &no_hook_turn] {
        assert!(result_of(events, "exec-a")["is_error"].is_null());
        assert!(result_of(events, "exec-a")["exit_code"].is_null());
        assert_eq!(
            unverified(events),
            ["completion does not match the observed call family or turn"]
        );
    }
    // A payload without `tool_use_id` is presented under the rendezvous name, which the vendor
    // never wrote; a completion that happens to carry that name correlates with nothing.
    let rendezvous = seamed(vec![
        hook("exec-a", "Bash", None, Some("turn-1"), "true"),
        command_completed("exec-a", Some("turn-1"), "completed", json!(0)),
    ]);
    // A hook for another tool is not a command.
    let other_tool = seamed(vec![
        hook("k", "apply_patch", Some("exec-a"), Some("turn-1"), "true"),
        command_completed("exec-a", Some("turn-1"), "completed", json!(0)),
    ]);
    for events in [&rendezvous, &other_tool] {
        assert!(result_of(events, "exec-a")["is_error"].is_null());
        assert_eq!(
            unverified(events),
            ["completion has no matching observed call"]
        );
    }
    // One id held by two hook requests that disagree is not one call.
    let reused = seamed(vec![
        hook("k1", "Bash", Some("exec-a"), Some("turn-1"), "true"),
        hook("k2", "Bash", Some("exec-a"), Some("turn-1"), "exit 7"),
        command_completed("exec-a", Some("turn-1"), "completed", json!(0)),
    ]);
    assert!(result_of(&reused, "exec-a")["is_error"].is_null());
    assert!(result_of(&reused, "exec-a")["exit_code"].is_null());
    assert_eq!(
        unverified(&reused),
        ["a call identifier was reused with another turn or request; its outcome is now unknown"]
    );
    // A supported shape that contradicts a22 stays unknown on this path too.
    let inconsistent = seamed(vec![
        hook("k", "Bash", Some("exec-a"), Some("turn-1"), "exit 7"),
        command_completed("exec-a", Some("turn-1"), "completed", json!(7)),
    ]);
    assert!(result_of(&inconsistent, "exec-a")["is_error"].is_null());
    assert_eq!(
        unverified(&inconsistent),
        ["completion lacks consistent supported outcome metadata"]
    );
}

/// A direct call's hook carries the model's own `call_*` id, which the rollout's call record
/// carries too; recording the hook leaves a22's path as it was.
#[test]
fn a_direct_call_with_its_hook_keeps_the_rollout_correlation() {
    let events = seamed(vec![
        turn_started("turn-1"),
        hook("k", "Bash", Some("call-1"), Some("turn-1"), "ls"),
        rollout(
            "response_item",
            &json!({"type":"function_call","call_id":"call-1","name":"exec_command","arguments":"{\"cmd\":\"ls\"}"}),
        ),
        command_completed("call-1", Some("turn-1"), "failed", json!(2)),
    ]);
    assert_eq!(result_of(&events, "call-1")["exit_code"], 2);
    assert_eq!(result_of(&events, "call-1")["is_error"], true);
    assert_eq!(unverified(&events), Vec::<&str>::new());
}

/// Amendment a47 § 5: Codex writes `exit_code: -1` for a command it could not start, one it
/// rejected, or one whose exit status it lost. That -1 is not an exit; the failure is still known.
fn assert_failed_without_an_exit(outcome: &Value, what: &str) {
    assert!(
        outcome["exit_code"].is_null(),
        "{what}: the vendor's -1 is published as an exit: {outcome}"
    );
    assert_eq!(outcome["is_error"], true, "{what}: {outcome}");
    assert_eq!(
        outcome["outcome_source"], "retained_tool_completion",
        "{what}: {outcome}"
    );
}

#[test]
fn a_direct_calls_minus_one_is_a_failure_without_an_exit_code() {
    for status in ["failed", "declined"] {
        let events = read(vec![
            call("exec_command"),
            tool_item(status, json!(-1), "call-1"),
        ]);
        assert_failed_without_an_exit(result(&events), &format!("direct {status}"));
        assert_eq!(unverified(&events), Vec::<&str>::new(), "direct {status}");
    }
    // `completed` with any exit other than 0 stays unsupported metadata, -1 included.
    let completed = read(vec![
        call("exec_command"),
        tool_item("completed", json!(-1), "call-1"),
    ]);
    assert!(result(&completed)["is_error"].is_null());
    assert!(result(&completed)["exit_code"].is_null());
}

#[test]
fn a_mediated_commands_minus_one_is_a_failure_without_an_exit_code() {
    for status in ["failed", "declined"] {
        let mut lines = vec![turn_started("turn-1"), outer_exec()];
        lines.extend(mediated("exec-unstarted", "true", status, json!(-1)));
        let events = seamed(lines);
        assert_failed_without_an_exit(
            result_of(&events, "exec-unstarted"),
            &format!("mediated {status}"),
        );
        assert_eq!(unverified(&events), Vec::<&str>::new(), "mediated {status}");
    }
    // A real exit beside it still reaches the result as a number.
    let mut lines = vec![turn_started("turn-1"), outer_exec()];
    lines.extend(mediated("exec-unstarted", "true", "declined", json!(-1)));
    lines.extend(mediated("exec-one", "false", "failed", json!(1)));
    let events = seamed(lines);
    assert_failed_without_an_exit(result_of(&events, "exec-unstarted"), "mediated declined");
    assert_eq!(result_of(&events, "exec-one")["exit_code"], 1);
    assert_eq!(result_of(&events, "exec-one")["is_error"], true);
}
