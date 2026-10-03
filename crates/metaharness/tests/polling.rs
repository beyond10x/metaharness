//! Actual native reader silence, with a Rust subprocess and no vendor/network.
use metaharness::{CodexSpawnRunner, LaunchPlanView, ProcessPoll, ProcessRunner, SpawnRunner};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

#[test]
#[ignore = "owned subprocess fixture, invoked by quiet_native_readers_yield_without_claiming_eof"]
fn quiet_process_fixture() {
    std::thread::sleep(Duration::from_secs(2));
}

#[test]
fn quiet_native_readers_yield_without_claiming_eof() {
    for mut runner in [
        Box::new(SpawnRunner::new()) as Box<dyn ProcessRunner>,
        Box::new(CodexSpawnRunner::new()) as Box<dyn ProcessRunner>,
    ] {
        let root = tempfile::tempdir().unwrap();
        let program = std::env::current_exe().unwrap();
        let args = vec![
            "--exact".into(),
            "quiet_process_fixture".into(),
            "--ignored".into(),
            "--nocapture".into(),
        ];
        let env = BTreeMap::from([(
            "CODEX_HOME".into(),
            root.path().join("codex-home").display().to_string(),
        )]);
        let channel = root.path().join("channel");
        let transcript = root.path().join("transcript.jsonl");
        let mut process = runner
            .start(&LaunchPlanView {
                program: program.to_str().unwrap(),
                args: &args,
                env: &env,
                cwd: root.path(),
                credential_copies: &[],
                decision_channel: &channel,
                transcript: &transcript,
            })
            .unwrap();
        let start = Instant::now();
        let observed = loop {
            match process.poll_line().unwrap() {
                ProcessPoll::Line(_) => (),
                other => break other,
            }
        };
        process.kill().unwrap();
        process.wait().unwrap();
        assert_eq!(
            observed,
            ProcessPoll::Idle,
            "silence must yield without claiming EOF"
        );
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "the reader waited for the quiet child to exit"
        );
    }
}

#[test]
fn polling_keeps_decisions_pending_until_the_clock_expires_them() {
    use metaharness::protocol::{CredentialSource, DecidedBy, DecisionMode, Event, Kind};
    use metaharness::{
        EventPoll, Input, ManualClock, Metaharness, ScriptStep, ScriptedLog, ScriptedRunner,
        ScriptedSeams,
    };
    let clock = ManualClock::new();
    let mut runner = ScriptedRunner::new(
        vec![
            ScriptStep::line(
                r#"{"emit":"tool.requested","call_id":"t1","name":"Bash","input":{"command":"ls"}}"#,
            ),
            ScriptStep::awaiting("t1"),
            ScriptStep::Idle,
        ],
        ScriptedLog::new(),
    );
    let mut run = Metaharness::new(Kind::Claude)
        .with_credentials(CredentialSource::None)
        .with_decisions(DecisionMode::Ask)
        .start_with_clock(
            Input::Prompt("fixture".into()),
            &mut runner,
            &mut ScriptedSeams,
            Box::new(clock.clone()),
        )
        .unwrap();
    assert!(matches!(run.poll_event().unwrap(), EventPoll::Event(_)));
    assert!(matches!(
        run.poll_event().unwrap(),
        EventPoll::DecisionPending
    ));
    assert_eq!(clock.reading_ms(), 0);
    let deadline = run.pending_calls()[0].deadline_ms();
    clock.advance(deadline);
    let EventPoll::Event(line) = run.poll_event().unwrap() else {
        panic!("deadline must emit its decision")
    };
    assert!(matches!(
        line.event,
        Event::ToolDecided {
            decided_by: DecidedBy::Deadline,
            ..
        }
    ));
    assert_eq!(run.pending_calls(), []);
    assert!(matches!(run.poll_event().unwrap(), EventPoll::Idle));
    let closure = run.next_event().unwrap().unwrap();
    assert!(matches!(closure.event, Event::StreamClosed { .. }));
    assert!(run.next_event().unwrap().is_none());
    assert!(matches!(run.poll_event().unwrap(), EventPoll::Ended));
}

#[test]
fn polling_rejects_an_unmatched_pending_decision_instead_of_spinning() {
    use metaharness::protocol::{CredentialSource, DecisionMode, Kind};
    use metaharness::{Input, Metaharness, ScriptStep, ScriptedLog, ScriptedRunner, ScriptedSeams};
    let mut runner = ScriptedRunner::new(vec![ScriptStep::awaiting("unseen")], ScriptedLog::new());
    let mut run = Metaharness::new(Kind::Claude)
        .with_credentials(CredentialSource::None)
        .with_decisions(DecisionMode::Ask)
        .start_with(
            Input::Prompt("fixture".into()),
            &mut runner,
            &mut ScriptedSeams,
        )
        .unwrap();
    assert!(
        run.poll_event()
            .unwrap_err()
            .to_string()
            .contains("none pending")
    );
}

#[test]
fn an_answer_after_the_armed_deadline_cannot_win_between_polls() {
    use metaharness::protocol::{
        Command, CommandOutcome, CredentialSource, DecidedBy, Decision, DecisionMode, Event, Kind,
        RefusalCode,
    };
    use metaharness::{
        Input, ManualClock, Metaharness, ScriptStep, ScriptedLog, ScriptedRunner, ScriptedSeams,
    };
    let clock = ManualClock::new();
    let mut runner = ScriptedRunner::new(
        vec![
            ScriptStep::line(
                r#"{"emit":"tool.requested","call_id":"t1","name":"Bash","input":{"command":"ls"}}"#,
            ),
            ScriptStep::awaiting("t1"),
        ],
        ScriptedLog::new(),
    );
    let mut run = Metaharness::new(Kind::Claude)
        .with_credentials(CredentialSource::None)
        .with_decisions(DecisionMode::Ask)
        .start_with_clock(
            Input::Prompt("fixture".into()),
            &mut runner,
            &mut ScriptedSeams,
            Box::new(clock.clone()),
        )
        .unwrap();
    run.next_event().unwrap().unwrap();
    clock.advance(run.pending_calls()[0].deadline_ms());
    let result = run
        .send(Command::ToolDecide {
            call_id: "t1".into(),
            decision: Decision::Allow,
        })
        .unwrap();
    assert!(
        matches!(&result, CommandOutcome::Refused { refused } if refused.code == RefusalCode::TooLate),
        "elapsed deadline must win over a queued answer: {result:?}"
    );
    run.drain().unwrap();
    assert_eq!(
        run.events()
            .iter()
            .filter(|event| matches!(
                event,
                Event::ToolDecided {
                    decided_by: DecidedBy::Deadline,
                    ..
                }
            ))
            .count(),
        1
    );
}

#[test]
fn an_honoured_interrupt_without_a_control_wire_stops_the_owned_process() {
    use metaharness::protocol::{
        CloseReason, Command, CommandOutcome, CredentialSource, DecisionMode, Event, Kind,
        ProcessTermination,
    };
    use metaharness::{EventPoll, Input, Metaharness, ScriptStep, ScriptedLog, ScriptedRunner};
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::new(vec![ScriptStep::Idle, ScriptStep::Idle], log.clone());
    let mut run = Metaharness::new(Kind::B10x)
        .with_credentials(CredentialSource::None)
        .with_decisions(DecisionMode::Observe)
        .with_model_endpoint("http://127.0.0.1:1")
        .with_model("synthetic-never-called")
        .start_with(
            Input::Prompt("fixture".into()),
            &mut runner,
            &mut metaharness_b10x::B10xSeams::new(None, None, None),
        )
        .unwrap();
    assert!(matches!(run.poll_event().unwrap(), EventPoll::Idle));
    assert!(matches!(
        run.send_as(
            "wireless-interrupt",
            Command::Interrupt {
                reason: "fixture".into()
            }
        )
        .unwrap(),
        CommandOutcome::Ok { .. }
    ));
    assert!(
        log.killed(),
        "an acknowledged interrupt without a wire must stop its owned process"
    );
    assert_eq!(
        log.written(),
        Vec::<String>::new(),
        "no fictitious control wire is written"
    );
    run.drain().unwrap();
    assert!(matches!(
        run.events().last(),
        Some(Event::StreamClosed {
            reason: CloseReason::Error,
            process: ProcessTermination::Unknown,
            ..
        })
    ));
    assert_eq!(
        run.events()
            .iter()
            .filter(|event| matches!(event, Event::StreamClosed { .. }))
            .count(),
        1
    );
    assert_eq!(run.events().iter().filter(|event| matches!(event, Event::CommandResult { id, .. } if id == "wireless-interrupt")).count(), 1);
}

#[test]
fn consuming_a_control_record_is_progress_rather_than_provider_silence() {
    use metaharness::protocol::{CredentialSource, DecisionMode, Kind};
    use metaharness::{
        ClaudeSeams, EventPoll, Input, Metaharness, ScriptStep, ScriptedLog, ScriptedRunner,
    };
    let mut runner = ScriptedRunner::new(
        vec![
            ScriptStep::line(
                r#"{"type":"tool_progress","tool_use_id":"fixture","tool_name":"Bash","elapsed_time_seconds":1}"#,
            ),
            ScriptStep::Idle,
        ],
        ScriptedLog::new(),
    );
    let mut run = Metaharness::new(Kind::Claude)
        .with_credentials(CredentialSource::None)
        .with_decisions(DecisionMode::Observe)
        .start_with(
            Input::Prompt("fixture".into()),
            &mut runner,
            &mut ClaudeSeams,
        )
        .unwrap();
    let observed = run.poll_event().unwrap();
    assert!(
        !matches!(observed, EventPoll::Idle),
        "a consumed control-plane record must not trigger the CLI idle wait"
    );
    assert!(matches!(observed, EventPoll::Progress));
    assert!(
        matches!(run.poll_event().unwrap(), EventPoll::Idle),
        "the following actual silence remains idle"
    );
}
