//! Source-backed terminal and tool observations never infer evidence from prose.
use metaharness_codex::RolloutReader;
use metaharness_protocol::{HermeticAttestation, HermeticMode, TranscriptRef};
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
