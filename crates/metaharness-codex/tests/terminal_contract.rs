//! Boundary attacks on the terminal evidence contract (synthetic wire only).
use metaharness_codex::RolloutReader;
use metaharness_protocol::{Event, HermeticAttestation, HermeticMode, TranscriptRef};
use serde_json::{Value, json};

fn terminal(payloads: &[Value]) -> Option<bool> {
    let mut reader = RolloutReader::new(
        TranscriptRef {
            path: None,
            digest: None,
            bytes: None,
        },
        HermeticAttestation::none(HermeticMode::Off),
    );
    for payload in payloads {
        reader.push_line(&json!({"type": "event_msg", "payload": payload}).to_string());
    }
    let events = reader.finish();
    assert_eq!(events.len(), 1);
    match &events[0].event {
        Event::SessionEnded {
            is_error,
            total_cost_usd,
            ..
        } => {
            assert_eq!(*total_cost_usd, None, "a terminal shape supplies no price");
            *is_error
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn nonnull_error_values_are_never_truthiness_tested() {
    for error in [json!(false), json!(0), json!(""), json!([]), json!({})] {
        assert_eq!(
            terminal(&[
                json!({"type": "task_complete", "error": error}),
                json!({"type": "task_started"}),
                json!({"type": "task_complete", "error": null, "last_agent_message": "ok"}),
            ]),
            Some(true)
        );
    }
}

#[test]
fn explicit_null_and_absent_error_have_distinct_evidence() {
    assert_eq!(
        terminal(&[json!({"type": "task_complete", "error": null})]),
        Some(false)
    );
    for message in [
        Value::Null,
        json!(""),
        json!(" \t\n"),
        json!(false),
        json!({}),
    ] {
        assert_eq!(
            terminal(&[json!({"type": "task_complete", "last_agent_message": message})]),
            None
        );
    }
    assert_eq!(
        terminal(&[json!({"type": "task_complete", "last_agent_message": " done "})]),
        Some(false)
    );
}
