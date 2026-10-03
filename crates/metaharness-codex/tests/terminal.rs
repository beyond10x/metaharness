//! Synthetic terminal records; no vendor process or real session is used.

use metaharness_codex::RolloutReader;
use metaharness_protocol::{Event, HermeticAttestation, HermeticMode, TranscriptRef};
use serde_json::{Value, json};

fn reader() -> RolloutReader {
    RolloutReader::new(
        TranscriptRef {
            path: None,
            digest: None,
            bytes: None,
        },
        HermeticAttestation::none(HermeticMode::Off),
    )
}

fn complete(reader: &mut RolloutReader, payload: &Value) -> Event {
    let turns = reader.push_line(&json!({"type": "event_msg", "payload": payload}).to_string());
    assert!(matches!(turns.as_slice(), [line] if matches!(line.event, Event::TurnEnded { .. })));
    assert!(reader.saw_terminal_record());
    let mut ended = reader.finish();
    assert_eq!(ended.len(), 1);
    assert!(
        reader.finish().is_empty(),
        "the terminal record is emitted once"
    );
    ended.remove(0).event
}

fn outcome(event: &Event, expected_error: Option<bool>) {
    let Event::SessionEnded {
        is_error,
        total_cost_usd,
        usage,
        terminal_reason,
        api_error_status,
        ..
    } = event
    else {
        panic!("expected session.ended, got {event:?}");
    };
    assert_eq!(*is_error, expected_error);
    assert_eq!(*total_cost_usd, None, "missing cost is not zero");
    assert_eq!(*usage, None, "missing usage is not invented");
    assert_eq!(*terminal_reason, None);
    assert_eq!(*api_error_status, None);
}

#[test]
fn explicit_terminal_error_survives_normalization() {
    for error in [
        json!("synthetic provider failure"),
        json!({"message": "synthetic failure"}),
    ] {
        let event = complete(
            &mut reader(),
            &json!({"type": "task_complete", "error": error}),
        );
        outcome(&event, Some(true));
    }
}

#[test]
fn partial_text_does_not_turn_terminal_failure_into_success() {
    let mut reader = reader();
    let text = reader.push_line(
        &json!({"type": "response_item", "payload": {"type": "message", "role": "assistant",
            "content": [{"type": "output_text", "text": "partial answer"}]}})
        .to_string(),
    );
    assert!(
        matches!(text.as_slice(), [line] if matches!(&line.event, Event::Text { text, .. } if text == "partial answer"))
    );
    let event = complete(
        &mut reader,
        &json!({"type": "task_complete", "error": "synthetic failure"}),
    );
    outcome(&event, Some(true));
}

#[test]
fn completion_without_an_error_preserves_unknown_outcome_and_cost() {
    for payload in [
        json!({"type": "task_complete"}),
        json!({"type": "task_complete", "error": null}),
    ] {
        outcome(&complete(&mut reader(), &payload), None);
    }
}

#[test]
fn an_incomplete_or_unknown_record_does_not_invent_a_terminal_outcome() {
    let mut reader = reader();
    assert!(!reader.saw_terminal_record());
    assert!(reader.finish().is_empty());
    let unknown = reader.push_line(r#"{"type":"event_msg","payload":{"type":"synthetic_unknown","error":"not a terminal record"}}"#);
    assert!(matches!(unknown.as_slice(), [line] if matches!(line.event, Event::Opaque { .. })));
    assert!(!reader.saw_terminal_record());
    assert!(reader.finish().is_empty());
}

#[test]
fn explicit_failure_does_not_suppress_the_version_warning() {
    let mut reader = reader();
    let opening =
        reader.push_line(r#"{"type":"session_meta","payload":{"cli_version":"0.153.4"}}"#);
    assert!(matches!(opening.as_slice(), [started, warning]
        if matches!(started.event, Event::SessionStarted { .. })
        && matches!(&warning.event, Event::Warning { code, .. } if code == "version_outside_pin")));
    outcome(
        &complete(
            &mut reader,
            &json!({"type": "task_complete", "error": "synthetic failure"}),
        ),
        Some(true),
    );
}
