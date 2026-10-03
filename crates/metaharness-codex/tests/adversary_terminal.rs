//! Independent synthetic checks of the terminal mapping, without a vendor process.

use metaharness_codex::RolloutReader;
use metaharness_protocol::{Event, HermeticAttestation, HermeticMode, TranscriptRef};
use serde_json::json;

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

#[test]
fn the_last_of_two_completions_alone_supplies_terminal_failure_evidence() {
    let absent = json!({"type": "task_complete"});
    let null = json!({"type": "task_complete", "error": null});
    let failure = json!({"type": "task_complete", "error": "synthetic failure"});
    for (earlier, last, expected_error) in [
        (failure.clone(), absent.clone(), None),
        (failure.clone(), null.clone(), None),
        (absent, failure.clone(), Some(true)),
        (null, failure, Some(true)),
    ] {
        let mut reader = reader();
        let mut events = Vec::new();
        for payload in [earlier, last] {
            events.extend(
                reader.push_line(r#"{"type":"event_msg","payload":{"type":"task_started"}}"#),
            );
            events.extend(
                reader.push_line(&json!({"type": "event_msg", "payload": payload}).to_string()),
            );
        }
        events.extend(reader.finish());
        assert_eq!(
            events
                .iter()
                .map(|line| line.event.name())
                .collect::<Vec<_>>(),
            [
                "turn.started",
                "turn.ended",
                "turn.started",
                "turn.ended",
                "session.ended",
            ]
        );
        match &events.last().expect("one terminal event").event {
            Event::SessionEnded {
                is_error,
                num_turns,
                total_cost_usd,
                usage,
                ..
            } => {
                assert_eq!(*is_error, expected_error);
                assert_eq!(*num_turns, Some(2));
                assert_eq!(*total_cost_usd, None);
                assert_eq!(*usage, None);
            }
            event => panic!("expected terminal event, got {event:?}"),
        }
        assert!(reader.finish().is_empty(), "finish is one-shot");
    }
}

#[test]
fn confidential_error_payload_is_not_copied_into_normalized_events() {
    // This marker is synthetic, not a credential or a retained vendor transcript.
    const PRIVATE_MARKER: &str = "synthetic-private-value-do-not-copy";
    let mut reader = reader();
    let mut events = reader.push_line(
        &json!({"type": "event_msg", "payload": {
            "type": "task_complete",
            "error": {"message": PRIVATE_MARKER, "authorization": PRIVATE_MARKER}
        }})
        .to_string(),
    );
    events.extend(reader.finish());
    assert_eq!(
        events
            .iter()
            .map(|line| line.event.name())
            .collect::<Vec<_>>(),
        ["turn.ended", "session.ended"]
    );
    assert!(matches!(
        &events[1].event,
        Event::SessionEnded {
            is_error: Some(true),
            ..
        }
    ));
    for emission in events {
        let normalized = serde_json::to_string(&emission.event).expect("serializable event");
        assert!(!normalized.contains(PRIVATE_MARKER));
        assert!(!normalized.contains("authorization"));
    }
}
