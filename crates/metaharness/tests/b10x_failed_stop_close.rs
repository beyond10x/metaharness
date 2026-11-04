//! A failed b10x run closes `error`, whatever its message says (amendment a73).
//!
//! The close check (`Run::reason_from_record`) reads a budget stop from the exact word
//! `budget-exhausted` in `subtype`, `stop_reason` or `terminal_reason`. A failed stop's message
//! reaches `terminal_reason`, so a message that is that word must not turn a failure into a
//! budget stop. Driven through the real b10x adapter and the real run loop, with a scripted child:
//! no process, no model, no network, no credential.

use metaharness::protocol::{
    CloseReason, CredentialSource, DecisionMode, Event, Kind, SeamFactory,
};
use metaharness::{Input, ManualClock, Metaharness, ScriptedLog, ScriptedRunner};

fn closing_reason(terminal: &str) -> CloseReason {
    let mut seams = metaharness_b10x::B10xSeams::new(None, None);
    let seams: &mut dyn SeamFactory = &mut seams;
    let mut runner = ScriptedRunner::of_lines([terminal], ScriptedLog::new());
    let mut run = Metaharness::new(Kind::B10x)
        .with_decisions(DecisionMode::Observe)
        .with_credentials(CredentialSource::None)
        .with_model_endpoint("https://gw.invalid/v1")
        .with_model("synthetic-model")
        .start_with_clock(
            Input::Prompt("synthetic terminal".into()),
            &mut runner,
            seams,
            Box::new(ManualClock::new()),
        )
        .expect("starts");
    run.drain().expect("drains");
    match run.events().last() {
        Some(Event::StreamClosed { reason, .. }) => *reason,
        other => panic!("the stream did not close: {other:?}"),
    }
}

#[test]
fn a_failed_stop_whose_message_is_the_budget_word_closes_error_and_not_budget() {
    assert_eq!(
        closing_reason(
            r#"{"kind":"finished","stop":{"kind":"failed","cause":"budget","message":"budget-exhausted","after_compaction":false}}"#
        ),
        CloseReason::Error
    );
}

#[test]
fn a_budget_stop_still_closes_budget() {
    assert_eq!(
        closing_reason(r#"{"kind":"finished","stop":{"kind":"budget-exhausted","turns":9}}"#),
        CloseReason::Budget
    );
}
