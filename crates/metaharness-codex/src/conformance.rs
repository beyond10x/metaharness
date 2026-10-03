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
