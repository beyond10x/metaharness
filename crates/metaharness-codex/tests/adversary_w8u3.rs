//! Adversary cases for amendment a47: a code-mode command's completion joined to its `PreToolUse`
//! request by the vendor's `tool_use_id` and `turn_id`. Each case names the mutant or the boundary
//! it stands against. Every line is invented in the 0.153.4 shape; none comes from a session.
use metaharness_codex::{CodexSeam, RolloutReader, hook_request_line};
use metaharness_protocol::{HarnessSeam, HermeticAttestation, HermeticMode, Seam, TranscriptRef};
use serde_json::{Value, json};

fn nowhere() -> TranscriptRef {
    TranscriptRef {
        path: None,
        digest: None,
        bytes: None,
    }
}

fn seamed(lines: &[String]) -> Vec<Value> {
    let mut seam = CodexSeam::new(
        nowhere(),
        HermeticAttestation::none(HermeticMode::Off),
        Seam::Hook,
    );
    let mut events = Vec::new();
    for line in lines {
        events.extend(
            seam.push_line(line)
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

fn rollout_only(lines: &[String]) -> Vec<Value> {
    let mut reader = RolloutReader::new(nowhere(), HermeticAttestation::none(HermeticMode::Off));
    let mut events = Vec::new();
    for line in lines {
        events.extend(
            reader
                .push_line(line)
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
    events
}

fn rollout(kind: &str, payload: &Value) -> String {
    json!({"type":kind,"payload":payload}).to_string()
}

fn turn_started(turn: &str) -> String {
    rollout("event_msg", &json!({"type":"task_started","turn_id":turn}))
}

fn outer_exec() -> String {
    rollout(
        "response_item",
        &json!({"type":"custom_tool_call","status":"completed","call_id":"call_outer","name":"exec","input":"await tools.exec_command({\"cmd\":\"true\"});"}),
    )
}

fn outer_output() -> String {
    rollout(
        "response_item",
        &json!({"type":"custom_tool_call_output","call_id":"call_outer","output":"Script completed\n"}),
    )
}

fn hook(key: &str, tool: &str, id: &str, turn: Option<&str>, command: &str) -> String {
    let mut raw = json!({"session_id":"s","cwd":"/scratch/work","hook_event_name":"PreToolUse","model":"a-model","permission_mode":"bypassPermissions","tool_name":tool,"tool_input":{"command":command},"tool_use_id":id});
    if let Some(turn) = turn {
        raw["turn_id"] = json!(turn);
    }
    hook_request_line(key, &raw.to_string())
}

fn completed(kind: &str, id: &str, turn: Option<&str>, status: &str, code: Value) -> String {
    let mut payload = json!({"type":"item_completed","thread_id":"s","item":{"type":kind,"id":id,"command":["/bin/bash","-lc","true"],"cwd":"file:///scratch/work","parsed_cmd":[],"source":"unified_exec_startup","status":status,"aggregated_output":""}});
    payload["item"]["exit_code"] = code;
    if let Some(turn) = turn {
        payload["turn_id"] = json!(turn);
    }
    rollout("event_msg", &payload)
}

fn command(id: &str, turn: Option<&str>, status: &str, code: Value) -> String {
    completed("CommandExecution", id, turn, status, code)
}

fn results<'a>(events: &'a [Value], id: &str) -> Vec<&'a Value> {
    events
        .iter()
        .filter(|e| e["event"] == "tool.result" && e["call_id"] == id)
        .collect()
}

fn last_result<'a>(events: &'a [Value], id: &str) -> &'a Value {
    results(events, id)
        .last()
        .copied()
        .unwrap_or_else(|| panic!("no tool.result for {id}: {events:#?}"))
}

fn unverified(events: &[Value]) -> Vec<&str> {
    events
        .iter()
        .filter(|e| e["event"] == "warning" && e["code"] == "TOOL_OUTCOME_UNVERIFIED")
        .map(|e| e["message"].as_str().unwrap_or_default())
        .collect()
}

fn assert_unknown(result: &Value) {
    assert!(result["is_error"].is_null(), "is_error stated: {result}");
    assert!(result["exit_code"].is_null(), "exit_code stated: {result}");
    assert!(
        result["outcome_source"].is_null(),
        "outcome_source stated: {result}"
    );
}

/// a47 § 3: "both sides carry a `turn_id`". Neither side carrying one is not equality. Stands
/// against the mutant that drops `call.turn_id.is_none()` from the hooked mismatch, which no
/// existing case reaches: every existing case has at least one side carrying a turn.
#[test]
fn a_join_with_no_turn_id_on_either_side_stays_unknown() {
    let events = seamed(&[
        hook("k", "Bash", "exec-a", None, "true"),
        command("exec-a", None, "completed", json!(0)),
    ]);
    assert_unknown(last_result(&events, "exec-a"));
    assert_eq!(
        unverified(&events),
        ["completion does not match the observed call family or turn"]
    );
}

/// a47 § 3: correlated only when the completion "is a `CommandExecution`". A `FileChange` under a
/// `Bash` hook's id is the wrong family. Stands against the mutant that drops the family
/// comparison from the hooked path; a patch completion would otherwise publish
/// `is_error: false` for a shell command.
#[test]
fn a_file_change_completion_never_joins_a_bash_hook() {
    let events = seamed(&[
        hook("k", "Bash", "exec-a", Some("turn-1"), "true"),
        completed(
            "FileChange",
            "exec-a",
            Some("turn-1"),
            "completed",
            Value::Null,
        ),
    ]);
    assert_unknown(last_result(&events, "exec-a"));
    assert_eq!(
        unverified(&events),
        ["completion does not match the observed call family or turn"]
    );
}

/// a47 § 3: "a second hook request under the same id with another turn id or input" voids the id.
/// The existing case varies only the input. Here the input is equal and the turn differs, and the
/// completion names the *first* turn, so a mutant that compares only the input joins it.
#[test]
fn a_second_hook_in_another_turn_voids_the_id_even_with_the_same_input() {
    let events = seamed(&[
        hook("k1", "Bash", "exec-a", Some("turn-1"), "true"),
        hook("k2", "Bash", "exec-a", Some("turn-2"), "true"),
        command("exec-a", Some("turn-1"), "completed", json!(0)),
    ]);
    assert_unknown(last_result(&events, "exec-a"));
    assert_eq!(
        unverified(&events),
        ["a call identifier was reused with another turn or request; its outcome is now unknown"]
    );
}

/// a22 keeps priority (a47 § 3, "A completion whose id a rollout call record carries keeps a22's
/// path unchanged"). A direct call whose rollout id was reused is unknown on a22's path; the hook
/// record under the same id must not resurrect it. Stands against looking up the hook record
/// before, or instead of, a conflicted rollout call.
#[test]
fn a_conflicted_rollout_call_is_not_rescued_by_its_hook() {
    let call = |args: &str| {
        rollout(
            "response_item",
            &json!({"type":"function_call","call_id":"call-1","name":"exec_command","arguments":args}),
        )
    };
    let events = seamed(&[
        turn_started("turn-1"),
        hook("k", "Bash", "call-1", Some("turn-1"), "ls"),
        call("{\"cmd\":\"ls\"}"),
        call("{\"cmd\":\"rm -rf x\"}"),
        command("call-1", Some("turn-1"), "completed", json!(0)),
    ]);
    assert_unknown(last_result(&events, "call-1"));
}

/// a47 § 4: "The outer call's `tool.result`, from `custom_tool_call_output`, stays content only",
/// even when every command it ran was joined in the same turn.
#[test]
fn the_outer_exec_call_stays_content_only_beside_joined_commands() {
    let events = seamed(&[
        turn_started("turn-1"),
        outer_exec(),
        hook("k0", "Bash", "exec-0", Some("turn-1"), "true"),
        command("exec-0", Some("turn-1"), "completed", json!(0)),
        hook("k7", "Bash", "exec-7", Some("turn-1"), "exit 7"),
        command("exec-7", Some("turn-1"), "failed", json!(7)),
        outer_output(),
    ]);
    assert_eq!(last_result(&events, "exec-0")["exit_code"], 0);
    assert_eq!(last_result(&events, "exec-7")["exit_code"], 7);
    let outer = last_result(&events, "call_outer");
    assert_unknown(outer);
    assert_eq!(outer["content"], "Script completed\n");
}

/// Order is not a key (a47 § 3). A completion read before its hook request matches nothing then,
/// and the hook arriving afterwards does not reach back and state an outcome.
#[test]
fn a_completion_read_before_its_hook_stays_unknown() {
    let events = seamed(&[
        command("exec-a", Some("turn-1"), "completed", json!(0)),
        hook("k", "Bash", "exec-a", Some("turn-1"), "true"),
    ]);
    let all = results(&events, "exec-a");
    assert_eq!(all.len(), 1, "{events:#?}");
    assert_unknown(all[0]);
    assert_eq!(
        unverified(&events),
        ["completion has no matching observed call"]
    );
}

/// The same completion twice keeps its exit; a contradicting third voids it (a22's rule, now on
/// the hooked path).
#[test]
fn a_repeated_completion_keeps_its_exit_and_a_contradiction_voids_it() {
    let events = seamed(&[
        hook("k", "Bash", "exec-a", Some("turn-1"), "exit 7"),
        command("exec-a", Some("turn-1"), "failed", json!(7)),
        command("exec-a", Some("turn-1"), "failed", json!(7)),
        command("exec-a", Some("turn-1"), "failed", json!(3)),
    ]);
    let all = results(&events, "exec-a");
    assert_eq!(all.len(), 3);
    assert_eq!(all[0]["exit_code"], 7);
    assert_eq!(all[1]["exit_code"], 7);
    assert_unknown(all[2]);
    assert_eq!(
        unverified(&events),
        ["contradictory structured tool outcomes"]
    );
}

/// Status and exit boundaries on the hooked path. `completed` needs exit 0 and `failed` a nonzero
/// one; a non-integer or out-of-range exit is never published as an exit.
#[test]
fn status_and_exit_boundaries_on_the_hooked_path() {
    let one = |status: &str, code: Value| {
        seamed(&[
            hook("k", "Bash", "exec-a", Some("turn-1"), "true"),
            command("exec-a", Some("turn-1"), status, code),
        ])
    };
    for (status, code) in [
        ("failed", json!(0)),
        ("completed", json!(1)),
        ("completed", json!("0")),
        ("completed", json!(0.0)),
        ("completed", json!(4_294_967_296_i64)),
        ("in_progress", json!(0)),
        ("declined", json!(0)),
    ] {
        let events = one(status, code.clone());
        assert_unknown(last_result(&events, "exec-a"));
        assert_eq!(
            unverified(&events),
            ["completion lacks consistent supported outcome metadata"],
            "{status} {code}"
        );
    }
    for code in [json!(2_147_483_648_i64), json!("7"), Value::Null] {
        let events = one("failed", code.clone());
        let result = last_result(&events, "exec-a");
        assert_eq!(result["is_error"], true, "{code}");
        assert!(result["exit_code"].is_null(), "{code}: {result}");
    }
}

/// a47 § 3: only a hook that names the tool `Bash` is recorded; the vendor's other names for a
/// shell call are not the hook's word on 0.153.4.
#[test]
fn only_the_bash_hook_name_is_recorded() {
    for tool in ["shell", "exec_command", "exec", "bash", ""] {
        let events = seamed(&[
            hook("k", tool, "exec-a", Some("turn-1"), "true"),
            command("exec-a", Some("turn-1"), "completed", json!(0)),
        ]);
        assert_unknown(last_result(&events, "exec-a"));
        assert_eq!(
            unverified(&events),
            ["completion has no matching observed call"],
            "{tool}"
        );
    }
}

/// a47 § 3, last bullet: a rollout read without the run's hook requests, as the viewer reads one,
/// cannot join. A hook envelope that somehow sat in a rollout file is not a hook request to the
/// bare reader.
#[test]
fn the_rollout_reader_alone_never_joins_even_with_a_hook_envelope_in_the_file() {
    let events = rollout_only(&[
        turn_started("turn-1"),
        outer_exec(),
        hook("k", "Bash", "exec-a", Some("turn-1"), "true"),
        command("exec-a", Some("turn-1"), "completed", json!(0)),
        outer_output(),
    ]);
    assert_unknown(last_result(&events, "exec-a"));
    assert_unknown(last_result(&events, "call_outer"));
    assert_eq!(
        unverified(&events),
        ["completion has no matching observed call"]
    );
}
