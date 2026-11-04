//! Adversary pass 1 on story:b10x-failed-run-names-its-cause (design amendment a73).
//!
//! The unit's own cases feed hand-written `finished` lines. These drive the run loop with the
//! records a released loop actually wrote, and probe the one branch of the mapping no unit case
//! reaches.
//!
//! The recorded stream below was written by the installed `b10x-harness 0.26.0` on 2026-11-04,
//! `run --json` against a closed loopback port: no model, no credential, no network. Only the
//! `started` line is reduced (no profile path, no workspace path); every other line is verbatim.

use metaharness::protocol::{
    CloseReason, CredentialSource, DecisionMode, Event, Kind, SeamFactory,
};
use metaharness::{Input, ManualClock, Metaharness, ScriptedLog, ScriptedRunner};

const RECORDED_FAILED_RUN: [&str; 6] = [
    r#"{"kind":"started","model":"b10x-emulated","published_tools":["file_read","dir_list","search","find"],"operations":["file.read","dir.list","search","find"],"withheld":[],"skills":[],"agents":[],"memories":[],"profiles":[],"context":[],"toolchains":[],"mcp":[],"credential_source":"api-key:environment","cwd":"/scratch/ws"}"#,
    r#"{"kind":"turn-started","turn":1}"#,
    r#"{"kind":"turn-retried","turn":1,"attempt":1,"reason":"HTTP request failed"}"#,
    r#"{"kind":"turn-retried","turn":1,"attempt":2,"reason":"HTTP request failed"}"#,
    r#"{"kind":"turn-retried","turn":1,"attempt":3,"reason":"HTTP request failed"}"#,
    r#"{"kind":"finished","stop":{"kind":"failed","cause":"provider-transport","message":"model wire refused: Transport: HTTP request failed","after_compaction":false},"turns":1}"#,
];

fn drive(lines: &[&str]) -> Vec<Event> {
    let mut seams = metaharness_b10x::B10xSeams::new(None, None);
    let seams: &mut dyn SeamFactory = &mut seams;
    let mut runner = ScriptedRunner::of_lines(lines.iter().copied(), ScriptedLog::new());
    let mut run = Metaharness::new(Kind::B10x)
        .with_decisions(DecisionMode::Observe)
        .with_credentials(CredentialSource::None)
        .with_model_endpoint("https://gw.invalid/v1")
        .with_model("synthetic-model")
        .start_with_clock(
            Input::Prompt("adversary u1".into()),
            &mut runner,
            seams,
            Box::new(ManualClock::new()),
        )
        .expect("starts");
    run.drain().expect("drains");
    run.events().to_vec()
}

fn closed(events: &[Event]) -> CloseReason {
    match events.last() {
        Some(Event::StreamClosed { reason, .. }) => *reason,
        other => panic!("the stream did not close: {other:?}"),
    }
}

/// Acceptance 1 against the released loop's own record, through the real run loop: exactly one
/// `session.ended`, `is_error: true`, the loop's turn count, the cause in `subtype` and the
/// message (which carries colons of its own) in `terminal_reason`, and the stream closes `error`.
#[test]
fn a_recorded_failed_run_ends_in_one_session_ended_that_names_its_cause() {
    let events = drive(&RECORDED_FAILED_RUN);
    let ended: Vec<&Event> = events
        .iter()
        .filter(|event| matches!(event, Event::SessionEnded { .. }))
        .collect();
    assert_eq!(ended.len(), 1, "exactly one terminal record: {events:?}");
    let Event::SessionEnded {
        is_error,
        subtype,
        terminal_reason,
        stop_reason,
        num_turns,
        ..
    } = ended[0]
    else {
        unreachable!()
    };
    assert_eq!(*is_error, Some(true));
    assert_eq!(*num_turns, Some(1));
    assert_eq!(subtype.as_deref(), Some("failed:provider-transport"));
    assert_eq!(
        terminal_reason.as_deref(),
        Some("failed: model wire refused: Transport: HTTP request failed")
    );
    assert_eq!(*stop_reason, None);
    assert_eq!(closed(&events), CloseReason::Error);
}

/// Amendment a73 point 3: only a `failed` stop is refined, *whatever else its stop carries*. The
/// unit's case for that carries `cause`, `message` and `turns` on a budget stop but never
/// `after_compaction`, so dropping `failed &&` from the compaction flag leaves every unit case
/// green. This one fails on that mutant: the budget stop's `subtype` would read
/// `budget-exhausted:after-compaction`.
#[test]
fn a_stop_that_is_not_failed_ignores_an_after_compaction_flag() {
    let events = drive(&[
        r#"{"kind":"finished","stop":{"kind":"budget-exhausted","after_compaction":true,"turns":9}}"#,
    ]);
    let Some(Event::SessionEnded {
        subtype,
        terminal_reason,
        ..
    }) = events
        .iter()
        .find(|event| matches!(event, Event::SessionEnded { .. }))
    else {
        panic!("a terminal record: {events:?}")
    };
    assert_eq!(subtype.as_deref(), Some("budget-exhausted"));
    assert_eq!(terminal_reason.as_deref(), Some("budget-exhausted"));
    assert_eq!(closed(&events), CloseReason::Budget);
}
