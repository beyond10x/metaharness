//! Adversary pass over `story:malformed-steering-line-answered-by-id` (amendment a44), through the
//! library face, [`Run::send_line`].
//!
//! Every run here is scripted: no process, no vendor binary, no model.

use metaharness::protocol::{
    CommandOutcome, DecidedBy, DecisionMode, EVENT_FORMAT, Event, EventLine, Kind, RefusalCode,
    parse_event_line, stream_completeness,
};
use metaharness::{
    Input, ManualClock, Metaharness, Run, ScriptStep, ScriptedLog, ScriptedRunner, ScriptedSeams,
};

const INIT: &str = r#"{"emit":"session.started","harness_version":"2.1.240","output_style":"default","plugins":[],"mcp_servers":[],"credential_source":"operator-login","inputs_digest":"tree"}"#;
const END: &str = r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#;
const CALL: &str =
    r#"{"emit":"tool.requested","call_id":"t1","name":"Bash","input":{"command":"ls"}}"#;

/// The flat decision the commands page once showed, under this id.
fn flat(id: &str) -> String {
    format!(
        r#"{{"format":"metaharness.command/1","id":{},"command":"tool.decide","call_id":"t1","decision":"allow"}}"#,
        serde_json::to_string(id).expect("a string")
    )
}

fn start(script: Vec<ScriptStep>, clock: &ManualClock) -> Run {
    Metaharness::new(Kind::Claude)
        .with_decisions(DecisionMode::Ask)
        .start_with_clock(
            Input::Prompt("do the thing".to_string()),
            &mut ScriptedRunner::new(script, ScriptedLog::new()),
            &mut ScriptedSeams,
            Box::new(clock.clone()),
        )
        .expect("the run starts")
}

fn answers(events: &[Event]) -> Vec<(String, Option<RefusalCode>)> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::CommandResult { id, outcome } => Some((
                id.clone(),
                match outcome {
                    CommandOutcome::Refused { refused } => Some(refused.code),
                    CommandOutcome::Ok { .. } => None,
                },
            )),
            _ => None,
        })
        .collect()
}

/// Every line of every fenced block on `page` whose info string is exactly `json`, numbered.
fn json_lines(page: &str) -> Vec<(usize, &str)> {
    let mut fence: Option<&str> = None;
    let mut lines = Vec::new();
    for (number, line) in page.lines().enumerate() {
        if let Some(info) = line.trim_start().strip_prefix("```") {
            fence = match fence {
                Some(_) => None,
                None => Some(info.trim()),
            };
            continue;
        }
        if fence == Some("json") && !line.trim().is_empty() {
            lines.push((number + 1, line));
        }
    }
    lines
}

/// Finding (amendment a17 against a44): a line sent after the stream closed is answered with a
/// `command.result` delivered **after** `stream.closed`, which a17 makes the last line. The stream
/// then reads inconsistent by the protocol's own audit.
///
/// The binary reaches this when a steering line arrives between the marker and the end of
/// `drive`: the loop drains the channel once more before it sees `Ended`.
#[test]
fn adversary_w7u3_a_line_sent_after_the_stream_closed_is_not_delivered_after_the_marker() {
    let mut run = start(
        vec![ScriptStep::line(INIT), ScriptStep::line(END)],
        &ManualClock::new(),
    );
    let lines = run.drain().expect("the run ends");
    assert_eq!(
        lines.last().map(|line| line.event.name()),
        Some("stream.closed")
    );

    let answered = run.send_line(flat("late")).expect("answered");
    assert!(matches!(
        answered,
        Some(CommandOutcome::Refused { refused }) if refused.code == RefusalCode::Malformed
    ));
    let after = run
        .next_event()
        .expect("polled")
        .map(|line| (line.event.name(), line.seq));
    assert_eq!(
        after, None,
        "an event was delivered after the stream.closed marker"
    );
    assert!(
        stream_completeness(run.events()).is_complete(),
        "{:?}",
        stream_completeness(run.events())
    );
}

/// The same after-the-marker delivery through `send_as`, which the unit did not change: the
/// mechanism predates the unit, and `send_line` inherits it.
#[test]
fn adversary_w7u3_send_as_after_the_stream_closed_is_not_delivered_after_the_marker_either() {
    let mut run = start(
        vec![ScriptStep::line(INIT), ScriptStep::line(END)],
        &ManualClock::new(),
    );
    run.drain().expect("the run ends");
    run.send_as(
        "late",
        metaharness::protocol::Command::Steer {
            text: "too late".to_string(),
        },
    )
    .expect("answered");
    let after = run
        .next_event()
        .expect("polled")
        .map(|line| (line.event.name(), line.seq));
    assert_eq!(
        after, None,
        "an event was delivered after the stream.closed marker"
    );
}

/// Confirms the implementor's suspicion about `website/docs/protocol/events.md:12`: the one
/// example line on the events page does not decode with the production decoder. The commands page
/// now has a test that would catch this; the events page has none.
#[test]
fn adversary_w7u3_every_json_line_on_the_events_page_parses_with_the_production_decoder() {
    let page = include_str!("../../../website/docs/protocol/events.md");
    let lines = json_lines(page);
    assert!(!lines.is_empty(), "the events page shows a JSON example");
    let refused: Vec<String> = lines
        .iter()
        .filter_map(|(number, line)| {
            parse_event_line(line)
                .err()
                .map(|error| format!("events.md:{number}: {error}"))
        })
        .collect();
    assert_eq!(refused, Vec::<String>::new());
}

/// Probe, green when written: the answer the commands page shows for its first line written flat
/// is the answer `send_line` gives, in id, code and the reason's exact words. Nothing in the unit
/// compared the page's quoted reason with the decoder.
#[test]
fn adversary_w7u3_the_commands_page_shows_the_answer_send_line_gives() {
    let page = include_str!("../../../website/docs/protocol/commands.md");
    let lines = json_lines(page);
    let first = lines.first().expect("the page shows a command").1;
    let flattened = first.replace(
        r#""decision":{"decision":"allow"}"#,
        r#""decision":"allow""#,
    );
    assert_ne!(flattened, first, "the first example is the nested allow");
    let shown = lines
        .iter()
        .find_map(|(_, line)| {
            parse_event_line(line)
                .ok()
                .filter(|event| event.event.name() == "command.result")
        })
        .expect("the page shows a command.result");

    let mut run = start(
        vec![ScriptStep::line(INIT), ScriptStep::line(END)],
        &ManualClock::new(),
    );
    run.send_line(&flattened).expect("answered");
    let given = run
        .next_event()
        .expect("polled")
        .expect("the answer is the next event");
    assert_eq!(given.event, shown.event);
}

/// Probe, green when written: ids of every shape are answered exactly once, in order, and every
/// answer is one line of JSON that decodes back to itself, so nothing a caller put in an id
/// breaks the stream's framing.
#[test]
fn adversary_w7u3_every_id_shape_is_answered_once_on_one_line() {
    let long = "x".repeat(100_000);
    let control = "c\n\r\u{0}\u{1b}\u{2028}\"\\x";
    let cases: Vec<(String, &str)> = vec![
        (
            r#"{"format":"metaharness.command/1","id":7,"command":"halt","reason":"x"}"#.into(),
            "-",
        ),
        (
            r#"{"format":"metaharness.command/1","id":null,"command":"halt","reason":"x"}"#.into(),
            "-",
        ),
        (
            r#"{"format":"metaharness.command/1","id":{"id":"c9"},"command":"halt","reason":"x"}"#
                .into(),
            "-",
        ),
        (r#"["format","metaharness.command/1","id","c1"]"#.into(), "-"),
        (flat(""), ""),
        (flat(&long), long.as_str()),
        (flat(control), control),
        (
            r#"{"id":"no-format","command":"halt","reason":"x"}"#.into(),
            "no-format",
        ),
        (
            r#"{"format":"metaharness.event/1","id":"echoed","event":"command.result"}"#.into(),
            "echoed",
        ),
        (
            r#"{"format":"metaharness.command/1","id":"dup","command":"tool.decide","call_id":"ghost","decision":{"decision":"allow"}}"#.into(),
            "dup",
        ),
        (
            r#"{"format":"metaharness.command/1","id":"dup","command":"tool.decide","call_id":"ghost","decision":{"decision":"allow"}}"#.into(),
            "dup",
        ),
    ];
    let mut run = start(
        vec![ScriptStep::line(INIT), ScriptStep::line(END)],
        &ManualClock::new(),
    );
    for (line, _) in &cases {
        assert!(run.send_line(line).expect("answered").is_some(), "{line}");
    }
    let delivered: Vec<EventLine> = run.drain().expect("the run ends");
    for line in &delivered {
        let wire = serde_json::to_string(line).expect("renders");
        assert!(!wire.contains('\n'), "one line: {wire:.200}");
        assert_eq!(line.format, EVENT_FORMAT);
        assert_eq!(
            parse_event_line(&wire).expect("decodes").event,
            line.event,
            "{wire:.200}"
        );
    }
    let ids: Vec<String> = answers(run.events())
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    let expected: Vec<String> = cases.iter().map(|(_, id)| (*id).to_string()).collect();
    assert_eq!(ids, expected);
}

/// Probe, green when written: a line that does not parse leaves a pending call pending. The
/// deadline still decides it, its answer comes after the `MALFORMED` one, and a nested decision
/// for the same call sent after the deadline is answered `TOO_LATE`, once.
#[test]
fn adversary_w7u3_a_malformed_line_decides_nothing_and_the_deadline_still_does() {
    let clock = ManualClock::new();
    let mut run = start(
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(CALL),
            ScriptStep::awaiting("t1"),
            ScriptStep::line(END),
        ],
        &clock,
    );
    run.next_event().expect("polled");
    let requested = run.next_event().expect("polled").expect("an event");
    assert_eq!(requested.event.name(), "tool.requested");

    run.send_line(flat("c1")).expect("answered");
    assert_eq!(run.pending_calls().len(), 1, "the call is still pending");
    clock.advance(run.deadline_ms() + 1);
    run.send_line(
        r#"{"format":"metaharness.command/1","id":"c2","command":"tool.decide","call_id":"t1","decision":{"decision":"allow"}}"#,
    )
    .expect("answered");
    run.drain().expect("the run ends");

    let decided: Vec<DecidedBy> = run
        .events()
        .iter()
        .filter_map(|event| match event {
            Event::ToolDecided { decided_by, .. } => Some(*decided_by),
            _ => None,
        })
        .collect();
    assert_eq!(decided, vec![DecidedBy::Deadline]);
    assert_eq!(
        answers(run.events()),
        vec![
            ("c1".to_string(), Some(RefusalCode::Malformed)),
            ("c2".to_string(), Some(RefusalCode::TooLate)),
        ]
    );
    let order: Vec<&str> = run
        .events()
        .iter()
        .map(Event::name)
        .filter(|name| matches!(*name, "command.result" | "tool.decided"))
        .collect();
    assert_eq!(
        order,
        vec!["command.result", "tool.decided", "command.result"]
    );
}
