//! Synthetic vendor inputs for the ESS production target. Never reaches a provider.
use metaharness_protocol::{Event, HermeticAttestation, HermeticMode, TranscriptRef};
use serde_json::{Value, json};

/// Construct a representative raw completion and read it through the production adapter.
///
/// # Errors
/// Refuses an unknown evidence class or a missing production terminal record.
pub fn completion(error_property: &str, last_message: &str) -> Result<Event, &'static str> {
    let mut payload = json!({"type":"task_complete"});
    match error_property {
        "missing" => (),
        "explicit-null" => payload["error"] = Value::Null,
        "nonnull" => payload["error"] = json!({"message":"synthetic provider failure"}),
        _ => return Err("unsupported error-property class"),
    }
    match last_message {
        "missing-or-nonstring" => (),
        "blank" => payload["last_agent_message"] = json!(" \t"),
        "nonblank" => payload["last_agent_message"] = json!("done"),
        _ => return Err("unsupported last-message class"),
    }
    let mut reader = crate::RolloutReader::new(
        TranscriptRef {
            path: None,
            digest: None,
            bytes: None,
        },
        HermeticAttestation::none(HermeticMode::Off),
    );
    reader.push_line(&json!({"type":"event_msg","payload":payload}).to_string());
    reader
        .finish()
        .pop()
        .map(|record| record.event)
        .ok_or("no terminal observation")
}

/// A synthetic hook call for a bounded governor scenario, in this adapter's vocabulary.
///
/// # Errors
/// Refuses a request shape that the fixture does not implement.
pub fn governed_request(shape: &str) -> Result<(&'static str, Value), &'static str> {
    match shape {
        "supported-shell" => Ok(("Bash", json!({"command":"cat README.md"}))),
        "malformed-shell" => Ok(("Bash", json!({"command":["cat","README.md"]}))),
        "patch" => Ok(("apply_patch", json!({"patch":"*** Delete File: README.md"}))),
        "unsupported-tool" => Ok(("Skill", json!({"skill":"aep:planning"}))),
        _ => Err("unsupported governed-call shape"),
    }
}

/// Feed a synthetic observation class through the actual rollout decoder.
///
/// # Errors
/// Refuses any evidence class outside this bounded corpus.
pub fn observation(evidence: &str) -> Result<Vec<Event>, &'static str> {
    let mut records: Vec<(&str, Value)> = Vec::new();
    let mut complete = json!({"type":"task_complete","error":null});
    match evidence {
        "final-answer" => complete["last_agent_message"] = json!("answer"),
        "commentary-only" => records.push((
            "event_msg",
            json!({"type":"agent_message","message":"not a terminal answer"}),
        )),
        "failed-after-text" => {
            complete["last_agent_message"] = json!("partial");
            complete["error"] = json!({"message":"failed"});
        }
        "model-known" | "model-missing" => {
            let mut context = json!({"turn_id":"fixture-turn"});
            if evidence == "model-known" {
                context["model"] = json!("observed-model");
            }
            records.push(("turn_context", context));
        }
        "tool-success" | "tool-failure" | "tool-denied" | "tool-patch-success"
        | "tool-unmatched" | "tool-contradictory" | "tool-incomplete" => {
            if evidence != "tool-unmatched" {
                records.push(("response_item",json!({"type":"function_call","call_id":"fixture-call","name":if evidence == "tool-patch-success" {"apply_patch"} else {"exec_command"},"arguments":"{}"})));
            }
            let command = |status: &str, code: Value| json!({"type":"item_completed","item":{"type":"CommandExecution","id":"fixture-call","status":status,"exit_code":code}});
            match evidence {
                "tool-incomplete" => records.push(("response_item",json!({"type":"function_call_output","call_id":"fixture-call","output":"Process exited with code 0"}))),
                "tool-patch-success" => records.push(("event_msg",json!({"type":"patch_apply_end","call_id":"fixture-call","status":"completed","success":true}))),
                "tool-failure" => records.push(("event_msg",command("failed",json!(7)))),
                "tool-denied" => records.push(("event_msg",command("declined",Value::Null))),
                _ => {
                    records.push(("event_msg",command("completed",json!(0))));
                    if evidence == "tool-contradictory" { records.push(("event_msg",command("failed",json!(7)))); }
                }
            }
        }
        _ => return Err("unsupported observation evidence"),
    }
    records.push(("event_msg", complete));
    let mut reader = crate::RolloutReader::new(
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
                .map(|emission| emission.event),
        );
    }
    events.extend(reader.finish().into_iter().map(|emission| emission.event));
    Ok(events)
}
