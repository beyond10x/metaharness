//! `stream.closed` — the last line of every stream this driver writes (amendment a17).
//!
//! **What the marker buys, in one sentence:** without it, a stream containing no `Bash` call and a
//! stream that was cut off before the first one are the same bytes, so every *negative* expectation
//! about a run — `nothing-was-moved`, `no-store-command-was-run`, `nothing-was-written-to-tmp` — is
//! undecidable whatever the run actually did. Eight `aep observe trace check` reports ended `undecided` for
//! exactly that reason on 2026-09-03.
//!
//! Every run below is driven through [`metaharness::ScriptedRunner`]: no process, no model, no
//! network, no credential. The C3 vectors in `metaharness::control_vectors` assert the same
//! property as a *complete* observable expectation, whole trace and whole written channel; this
//! file asserts the four exit paths one at a time, so a failure names which one moved.

use metaharness::protocol::{
    CloseReason, Command, CredentialSource, DecisionMode, Event, Kind, StreamCompleteness,
    stream_completeness,
};
use metaharness::{
    FakeAuditor, Input, ManualClock, Metaharness, Run, ScriptStep, ScriptedLog, ScriptedRunner,
    ScriptedSeams,
};

const INIT: &str = r#"{"emit":"session.started","harness_version":"2.1.240","output_style":"default","plugins":[],"mcp_servers":[],"credential_source":"operator-login"}"#;
const CALL: &str =
    r#"{"emit":"tool.requested","call_id":"t1","name":"Bash","input":{"command":"ls"}}"#;
const RESULT: &str = r#"{"emit":"tool.result","call_id":"t1","is_error":false}"#;
const END: &str = r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#;
const END_ON_BUDGET: &str = r#"{"emit":"session.ended","is_error":false,"subtype":"stopped","terminal_reason":"budget-exhausted"}"#;

fn started(script: Vec<ScriptStep>, decisions: DecisionMode) -> Run {
    started_as(Kind::Claude, script, decisions)
}

fn started_as(kind: Kind, script: Vec<ScriptStep>, decisions: DecisionMode) -> Run {
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::new(script, log);
    let mut seams = ScriptedSeams;
    let mut builder = Metaharness::new(kind).with_decisions(decisions);
    if kind == Kind::B10x {
        // The b10x loop is pointed at an endpoint by its caller and refuses to default to one.
        // Nothing is reached: the script answers, and the scripted runner starts no process.
        builder = builder
            .with_model_endpoint("https://gw.invalid/v1")
            .with_model("a-model-nobody-calls")
            .with_credentials(CredentialSource::None);
    }
    builder
        .start_with_clock(
            Input::Prompt("do the thing".to_string()),
            &mut runner,
            &mut seams,
            Box::new(ManualClock::new()),
        )
        .expect("the run starts")
}

/// The marker, read off the very last event, or a description of what was there instead.
fn closing(run: &Run) -> Result<(u64, CloseReason), Vec<&'static str>> {
    match run.events().last() {
        Some(Event::StreamClosed { events, reason, .. }) => Ok((*events, *reason)),
        _ => Err(run.events().iter().map(Event::name).collect()),
    }
}

// --- the four exit paths ------------------------------------------------------------------------

/// **Normal end.** The child ends its own stream, its terminal record reports a finished run, and
/// the marker counts every line before it.
#[test]
fn a_run_that_ends_normally_closes_completed_and_counts_the_lines_before_it() {
    let mut run = started(
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(CALL),
            ScriptStep::line(RESULT),
            ScriptStep::line(END),
        ],
        DecisionMode::Frame,
    );
    let lines = run.drain().expect("the run drains");

    // Six before it: the opening record, the frame-mode warning this run earns by carrying no
    // frame, the call, its decision, its result, and the terminal record.
    assert_eq!(
        run.events().iter().map(Event::name).collect::<Vec<_>>(),
        [
            "session.started",
            "warning",
            "tool.requested",
            "tool.decided",
            "tool.result",
            "session.ended",
            "stream.closed",
        ]
    );
    assert_eq!(closing(&run), Ok((6, CloseReason::Completed)));
    let last = lines.last().expect("a last line");
    assert_eq!(last.event.name(), "stream.closed");
    assert_eq!(
        last.seq,
        lines.len() as u64,
        "the marker takes the last sequence number, so `events` is `seq - 1`"
    );
    assert!(stream_completeness(run.events()).is_complete());
}

/// **Budget stop.** The reason is read from the terminal record's own word and never defaulted: a
/// loop that closed every stream `completed` would report a stopped run as a finished one.
#[test]
fn a_budget_stop_closes_with_the_terminal_records_own_word() {
    let mut run = started(
        vec![ScriptStep::line(INIT), ScriptStep::line(END_ON_BUDGET)],
        DecisionMode::Frame,
    );
    run.drain().expect("the run drains");
    assert_eq!(closing(&run), Ok((2, CloseReason::Budget)));
}

/// **Kill.** `halt` writes the control, kills the child and winds up, so there is no terminal
/// record to read a reason out of — and the marker still closes the stream, saying **who** ended
/// the run rather than pretending it finished.
#[test]
fn a_halted_run_closes_steer_halt_although_no_terminal_record_was_ever_written() {
    let mut run = started(
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(CALL),
            ScriptStep::awaiting("t1"),
            ScriptStep::line(RESULT),
            ScriptStep::line(END),
        ],
        DecisionMode::Ask,
    );
    while let Some(line) = run.next_event().expect("the run drives") {
        if let Event::ToolRequested {
            decision_required: true,
            ..
        } = &line.event
        {
            run.send(Command::Halt {
                reason: "the embedder is done".to_string(),
            })
            .expect("halt is honoured");
        }
    }

    assert!(
        !run.saw_terminal_record(),
        "the child was killed mid-stream"
    );
    let (events, reason) = closing(&run).expect("a halted run still closes its stream");
    assert_eq!(reason, CloseReason::SteerHalt);
    assert_eq!(events, run.events().len() as u64 - 1);
}

/// **Error.** A child that dies without a terminal record still closes its stream, and the marker
/// says `error` rather than `completed`: *the stream is complete and the run is not* are two facts,
/// and they live in two fields.
#[test]
fn a_run_with_no_terminal_record_closes_error_rather_than_completed() {
    let mut run = started(vec![ScriptStep::line(INIT)], DecisionMode::Frame);
    run.drain().expect("the run drains");

    assert!(!run.saw_terminal_record());
    assert_eq!(closing(&run), Ok((1, CloseReason::Error)));
}

/// **For every harness kind**, because the marker is the *loop's* and not an adapter's: one `Run`
/// drives every kind, and a stream that closed on one vendor and not on another would mean a
/// checker could decide a negative row about a Claude run and not about a Codex one.
#[test]
fn every_harness_kind_closes_its_stream() {
    // b10x takes `observe` and nothing else — it decides nothing and asserts that it decides
    // nothing (invariant 9) — so the mode is the kind's, and the marker is not.
    for (kind, decisions) in [
        (Kind::Claude, DecisionMode::Frame),
        (Kind::Codex, DecisionMode::Frame),
        (Kind::B10x, DecisionMode::Observe),
    ] {
        let mut run = started_as(
            kind,
            vec![ScriptStep::line(INIT), ScriptStep::line(END)],
            decisions,
        );
        run.drain().expect("the run drains");
        assert_eq!(
            closing(&run),
            Ok((2, CloseReason::Completed)),
            "{} did not close its stream",
            kind.as_str()
        );
    }
}

// --- what a reader gets out of it -----------------------------------------------------------------

/// The marker is readable **on its own**, by something that seeks to the end of a file: it names
/// its run in the payload as well as on the line, and the two are rendered from one stream so they
/// cannot disagree.
#[test]
fn the_last_line_names_its_own_run_and_is_readable_alone() {
    let mut run = started(
        vec![ScriptStep::line(INIT), ScriptStep::line(END)],
        DecisionMode::Frame,
    );
    let lines = run.drain().expect("the run drains");
    let last = lines.last().expect("a last line");

    let written = serde_json::to_string(last).expect("the line renders");
    let read: serde_json::Value = serde_json::from_str(&written).expect("the line is JSON");
    assert_eq!(read["event"], "stream.closed");
    assert_eq!(read["run"], read["run_id"]);
    assert_eq!(read["events"], 2);
    assert_eq!(read["reason"], "completed");
}

/// The audit names a truncated stream, and says nothing softer about it. A report that stayed
/// silent here would leave *this run did X zero times* and *this file stopped before X could
/// happen* the same bytes — which is the reading invariant 3 refuses everywhere else.
#[test]
fn the_audit_names_a_stream_with_no_marker_as_truncated() {
    let mut run = started(
        vec![ScriptStep::line(INIT), ScriptStep::line(END)],
        DecisionMode::Frame,
    );
    run.drain().expect("the run drains");
    let report = run
        .audit(&mut FakeAuditor::default())
        .expect("the floor always runs");

    assert!(
        report.stream.is_complete(),
        "a driven run closes its own stream"
    );
    assert!(
        report.render().contains("stream: complete"),
        "{}",
        report.render()
    );

    // The same report about a stream that was cut off: named, and named as a truncation.
    let truncated = metaharness::AuditReport {
        stream: StreamCompleteness::Truncated,
        ..report
    };
    let rendered = truncated.render();
    assert!(rendered.contains("stream: TRUNCATED"), "{rendered}");
    assert!(
        rendered.contains("absence"),
        "the line says what the truncation costs a reader: {rendered}"
    );
}

/// A marker that does not add up is `inconsistent`, never `complete`. `events` is checked against
/// the lines actually read, so a field a reader had to take on trust is not one.
#[test]
fn a_marker_whose_count_disagrees_with_the_stream_decides_nothing() {
    let miscounted = [
        Event::Text {
            text: "hello".to_string(),
            request_id: None,
        },
        Event::StreamClosed {
            events: 99,
            reason: CloseReason::Completed,
            run_id: "r".to_string(),
        },
    ];
    let seen = stream_completeness(&miscounted);
    assert!(!seen.is_complete());
    assert_eq!(seen.events(), None, "a miscount decides no count");
    assert_eq!(
        seen.reason(),
        Some(CloseReason::Completed),
        "the reason is still the producer's own claim, and is reported as such"
    );
    assert!(seen.render().contains("INCONSISTENT"), "{}", seen.render());
}

/// Real adapter → core stream closure → the exit value used by the CLI with no
/// audit. The scripted child starts no process and reads no credentials.
mod codex_terminal_regression {
    use super::*;

    #[test]
    fn other_real_adapters_retain_success_and_reject_failure() {
        use metaharness::protocol::SeamFactory;
        for (kind, terminal, expected) in [
            (
                Kind::Claude,
                r#"{"type":"result","subtype":"success","is_error":false}"#,
                CloseReason::Completed,
            ),
            (
                Kind::Claude,
                r#"{"type":"result","subtype":"success"}"#,
                CloseReason::Completed,
            ),
            (
                Kind::Claude,
                r#"{"type":"result","is_error":true}"#,
                CloseReason::Error,
            ),
            (
                Kind::B10x,
                r#"{"kind":"finished","stop":{"kind":"completed"}}"#,
                CloseReason::Completed,
            ),
            (
                Kind::B10x,
                r#"{"kind":"finished","stop":{"kind":"error"}}"#,
                CloseReason::Error,
            ),
            (
                Kind::B10x,
                r#"{"kind":"finished","stop":{"kind":"budget-exhausted"}}"#,
                CloseReason::Budget,
            ),
        ] {
            let mut seam: Box<dyn SeamFactory> = match kind {
                Kind::Claude => Box::new(metaharness::ClaudeSeams),
                Kind::B10x => Box::new(metaharness_b10x::B10xSeams::new(None, None, None)),
                Kind::Codex => unreachable!("covered by the Codex-specific cases"),
            };
            let mut runner = ScriptedRunner::of_lines([terminal], ScriptedLog::new());
            let mut run = Metaharness::new(kind)
                .with_decisions(DecisionMode::Observe)
                .with_credentials(CredentialSource::None)
                .with_model_endpoint("https://gw.invalid/v1")
                .with_model("synthetic-model")
                .start_with_clock(
                    Input::Prompt("synthetic terminal".into()),
                    &mut runner,
                    seam.as_mut(),
                    Box::new(ManualClock::new()),
                )
                .expect("starts");
            run.drain().expect("drains");
            assert_eq!(
                closing(&run).map(|(_, reason)| reason),
                Ok(expected),
                "{kind:?}: {terminal}"
            );
            assert_eq!(
                run.exit(None).code() == 0,
                expected == CloseReason::Completed
            );
            let report = metaharness::AuditReport {
                rows: Vec::new(),
                census: metaharness::protocol::DecisionCensus::default(),
                withheld: None,
                installed_plugins: Vec::new(),
                auditor: None,
                saw_terminal_record: run.saw_terminal_record(),
                stream: stream_completeness(run.events()),
            };
            assert_eq!(
                run.exit(Some(&report)).code() == 0,
                expected == CloseReason::Completed
            );
        }
    }

    const FAILURE: &str = r#"{"type":"event_msg","payload":{"type":"task_complete","error":{"message":"synthetic unsupported model"}}}"#;
    const SUCCESS: &str = r#"{"type":"event_msg","payload":{"type":"task_complete","error":null,"last_agent_message":"done"}}"#;
    const PARTIAL: &str =
        r#"{"type":"event_msg","payload":{"type":"agent_message","message":"starting the work"}}"#;

    fn replay(lines: &[&str]) -> Run {
        let mut runner = ScriptedRunner::of_lines(lines.iter().copied(), ScriptedLog::new());
        let mut seams = metaharness::CodexSeams;
        let mut run = Metaharness::new(Kind::Codex)
            .with_decisions(DecisionMode::Observe)
            .with_credentials(CredentialSource::None)
            .start_with_clock(
                Input::Prompt("synthetic terminal regression".to_string()),
                &mut runner,
                &mut seams,
                Box::new(ManualClock::new()),
            )
            .expect("the synthetic run starts");
        run.drain().expect("the synthetic run drains");
        run
    }

    #[test]
    fn explicit_failure_closes_error() {
        let run = replay(&[FAILURE]);
        assert_eq!(
            closing(&run).map(|(_, reason)| reason),
            Ok(CloseReason::Error)
        );
    }

    #[test]
    fn explicit_failure_yields_nonzero_cli_exit_value() {
        let run = replay(&[FAILURE]);
        assert_ne!(run.exit(None).code(), 0);
    }

    #[test]
    fn actual_success_closes_completed_and_exits_zero() {
        let run = replay(&[SUCCESS]);
        assert_eq!(
            closing(&run).map(|(_, reason)| reason),
            Ok(CloseReason::Completed)
        );
        assert_eq!(run.exit(None).code(), 0);
    }

    #[test]
    fn preliminary_text_does_not_hide_failure_from_stream_or_exit() {
        let run = replay(&[PARTIAL, FAILURE]);
        assert!(run.events().iter().any(|event| matches!(event,
            Event::Text { text, .. } if text == "starting the work")));
        assert_eq!(
            closing(&run).map(|(_, reason)| reason),
            Ok(CloseReason::Error)
        );
        assert_ne!(run.exit(None).code(), 0);
    }

    #[test]
    fn missing_terminal_record_cannot_claim_success() {
        let run = replay(&[PARTIAL]);
        assert_eq!(
            closing(&run).map(|(_, reason)| reason),
            Ok(CloseReason::Error)
        );
        assert_ne!(run.exit(None).code(), 0);
    }

    #[test]
    fn incomplete_terminal_evidence_cannot_claim_completed() {
        let run = replay(&[r#"{"type":"event_msg","payload":{"type":"task_complete"}}"#]);
        assert_ne!(
            closing(&run).map(|(_, reason)| reason),
            Ok(CloseReason::Completed)
        );
    }

    #[test]
    fn incomplete_terminal_evidence_cannot_exit_zero() {
        let run = replay(&[r#"{"type":"event_msg","payload":{"type":"task_complete"}}"#]);
        assert_ne!(run.exit(None).code(), 0);
    }

    #[test]
    fn successful_audit_cannot_mask_failed_or_unknown_termination() {
        for terminal in [
            FAILURE,
            r#"{"type":"event_msg","payload":{"type":"task_complete"}}"#,
        ] {
            let run = replay(&[terminal]);
            let report = metaharness::AuditReport {
                rows: Vec::new(),
                census: metaharness::protocol::DecisionCensus::default(),
                withheld: None,
                installed_plugins: Vec::new(),
                auditor: None,
                saw_terminal_record: run.saw_terminal_record(),
                stream: stream_completeness(run.events()),
            };
            assert_ne!(run.exit(Some(&report)).code(), 0);
            assert_ne!(report.exit().code(), 0);
        }
    }

    #[test]
    fn native_process_failure_overrides_a_successful_terminal_record() {
        use metaharness::{HarnessProcess, LaunchPlanView, ProcessRunner, ScriptedProcess};

        struct ExitingRunner(Option<i32>);
        struct ExitingProcess(ScriptedProcess, Option<i32>);
        impl HarnessProcess for ExitingProcess {
            fn next_line(&mut self) -> std::io::Result<Option<String>> {
                self.0.next_line()
            }
            fn write_line(&mut self, line: &str) -> std::io::Result<()> {
                self.0.write_line(line)
            }
            fn kill(&mut self) -> std::io::Result<()> {
                self.0.kill()
            }
            fn wait(&mut self) -> std::io::Result<Option<i32>> {
                Ok(self.1)
            }
        }
        impl ProcessRunner for ExitingRunner {
            fn start(
                &mut self,
                _: &LaunchPlanView<'_>,
            ) -> std::io::Result<Box<dyn HarnessProcess>> {
                Ok(Box::new(ExitingProcess(
                    ScriptedProcess::new(vec![ScriptStep::line(SUCCESS)], ScriptedLog::new()),
                    self.0,
                )))
            }
        }
        for code in [Some(17), None] {
            let mut run = Metaharness::new(Kind::Codex)
                .with_decisions(DecisionMode::Observe)
                .with_credentials(CredentialSource::None)
                .start_with_clock(
                    Input::Prompt("synthetic process exit".into()),
                    &mut ExitingRunner(code),
                    &mut metaharness::CodexSeams,
                    Box::new(ManualClock::new()),
                )
                .expect("starts");
            run.drain().expect("drains");
            assert_eq!(
                closing(&run).map(|(_, reason)| reason),
                Ok(CloseReason::Error)
            );
            assert_ne!(run.exit(None).code(), 0);
        }
    }
}
