//! What the run loop promises, asserted through the scripted fake.
//!
//! Every test here is named as the property it holds, because a test called `test_run_2` tells
//! a later reader nothing about which guarantee they just broke.

use metaharness::protocol::{
    Command, CommandOutcome, CredentialSource, DecidedBy, Decision, DecisionMode, Digest, Event,
    EvidenceLine, Frame, Handoff, HermeticMode, Kind, NodeRef, Operation, OperationSet,
    RefusalCode, RunSpec, Seam, StepRef, ToolSurface, WorkflowRef,
};
use metaharness::{
    ClaudeSeams, Input, ManualClock, Metaharness, Refusal, Run, ScriptStep, ScriptedLog,
    ScriptedRunner, ScriptedSeams, capabilities, metaharness_deadline_ms, request_digest,
    start_refusals, vendor_hook_timeout_ms, warning,
};

const INIT: &str = r#"{"emit":"session.started","harness_version":"2.1.239","output_style":"default","plugins":[],"mcp_servers":[],"credential_source":"operator-login","inputs_digest":"tree"}"#;
const END: &str = r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#;

fn call(id: &str, tool: &str) -> String {
    format!(
        r#"{{"emit":"tool.requested","call_id":"{id}","name":"{tool}","input":{{"command":"ls"}}}}"#
    )
}

fn frame_admitting(operations: OperationSet) -> Frame {
    Frame {
        workflow: WorkflowRef {
            id: "development/default".into(),
            version: "1".into(),
        },
        node: NodeRef {
            id: "implement".into(),
        },
        step: StepRef {
            workflow: "development/default".into(),
            state: "implement".into(),
            index: 1,
            attempt: 1,
        },
        prior: vec![EvidenceLine {
            text: "the specification is approved".into(),
            source: Some("docs/specs/one.md".into()),
        }],
        obligations: Vec::new(),
        reaching: Vec::new(),
        next: Vec::new(),
        handoff: Handoff::None,
        operations,
        entities: None,
        digest: Digest::of(b""),
    }
}

struct Started {
    run: Run,
    log: ScriptedLog,
}

fn start(builder: Metaharness, script: Vec<ScriptStep>) -> Started {
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::new(script, log.clone());
    let mut seams = ScriptedSeams;
    let run = builder
        .start_with_clock(
            Input::Prompt("do the thing".to_string()),
            &mut runner,
            &mut seams,
            Box::new(ManualClock::new()),
        )
        .expect("the run starts");
    Started { run, log }
}

fn names(run: &Run) -> Vec<&'static str> {
    run.events().iter().map(Event::name).collect()
}

// ---------------------------------------------------------------- refusals, all exit 2

#[test]
fn a_codex_run_is_refused_by_name_because_there_is_no_codex_adapter() {
    let refusal = Metaharness::new(Kind::Codex)
        .start(Input::FromSpec)
        .expect_err("codex has no adapter in this build");
    assert_eq!(
        refusal,
        Refusal::NoAdapter {
            kind: "codex".to_string()
        }
    );
    assert!(refusal.to_string().contains("codex"));
}

#[test]
fn a_frame_document_is_refused_because_the_on_disk_format_is_owed() {
    let refusal = Metaharness::new(Kind::Claude)
        .with_frame_file("frames/implement.yaml")
        .start(Input::FromSpec)
        .expect_err("--frame is refused in v0.1");
    assert!(matches!(refusal, Refusal::FrameFile { .. }));
    assert!(refusal.to_string().contains("on-disk frame format"));
}

#[test]
fn an_owned_tool_surface_is_refused_because_metaharness_does_not_implement_the_tools() {
    let refusal = Metaharness::new(Kind::Claude)
        .with_tool_surface(ToolSurface::Owned)
        .start(Input::FromSpec)
        .expect_err("strategy C is not built");
    assert_eq!(refusal, Refusal::ToolSurfaceOwned);
}

/// A caller with two problems should learn about the one they can fix — and should learn it
/// **before** anything is spawned, because a refusal that costs a process start is a refusal that
/// costs money on the run after this one.
#[test]
fn a_bad_spec_is_refused_before_anything_is_spawned() {
    let refusal = Metaharness::new(Kind::Codex)
        .with_frame_file("x.yaml")
        .start(Input::FromSpec)
        .expect_err("refused");
    assert!(matches!(refusal, Refusal::NoAdapter { .. }));
}

#[test]
fn a_command_the_adapter_refuses_is_named_at_run_start_and_not_at_the_call() {
    let mut adapter = capabilities(Kind::Claude).expect("the claude adapter exists");
    adapter.commands.insert(
        "tool.decide".to_string(),
        metaharness::protocol::CommandSupport::Refused(RefusalCode::UnsupportedControl),
    );
    let spec = RunSpec {
        decisions: DecisionMode::Ask,
        ..RunSpec::new(Kind::Claude)
    };
    let refusals = start_refusals(&adapter, &spec);
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0].0, "tool.decide");

    let refusal = Refusal::Control { refusals };
    let emissions = refusal.emissions();
    assert_eq!(emissions.len(), 1);
    assert!(matches!(
        &emissions[0].event,
        Event::CommandResult {
            id,
            outcome: CommandOutcome::Refused { .. }
        } if id == "start/tool.decide"
    ));
}

// ---------------------------------------------------------------- the builder is one value

#[test]
fn every_builder_method_sets_one_field_of_the_one_options_type() {
    let built = Metaharness::new(Kind::Claude)
        .with_hermetic(HermeticMode::Strict)
        .with_prompt("p")
        .with_frame_file("f.yaml")
        .with_decisions(DecisionMode::Ask)
        .with_tool_surface(ToolSurface::Owned)
        .with_credentials(CredentialSource::ApiKey)
        .with_model("sonnet")
        .with_max_turns(30)
        .with_plugin_dir("plugins/one")
        .with_strict_version(true)
        .with_audit(true)
        .with_spec_file("expectations.yaml")
        .with_auditor("protocol trace check")
        .with_auditor_arg("--advisory")
        .with_auditor_arg("billed-to-the-session");

    let expected = RunSpec {
        kind: Kind::Claude,
        hermetic: HermeticMode::Strict,
        prompt: Some("p".to_string()),
        frame: Some("f.yaml".into()),
        decisions: DecisionMode::Ask,
        tool_surface: ToolSurface::Owned,
        credentials: CredentialSource::ApiKey,
        model: Some("sonnet".to_string()),
        max_turns: Some(30),
        plugin_dir: vec!["plugins/one".into()],
        strict_version: true,
        audit: true,
        spec: Some("expectations.yaml".into()),
        auditor: Some("protocol trace check".to_string()),
        auditor_args: vec![
            "--advisory".to_string(),
            "billed-to-the-session".to_string(),
        ],
    };
    assert_eq!(built.spec(), &expected);
}

#[test]
fn a_frame_given_to_the_builder_is_sealed_so_its_digest_describes_it() {
    let built = Metaharness::new(Kind::Claude)
        .with_frame(frame_admitting(OperationSet::of([Operation::FileRead])));
    assert!(built.frame().expect("a frame").digest_intact());
}

#[test]
fn from_spec_skips_the_builder_and_keeps_the_spec_unchanged() {
    let spec = RunSpec {
        model: Some("opus".to_string()),
        ..RunSpec::new(Kind::Claude)
    };
    assert_eq!(Metaharness::from_spec(spec.clone()).spec(), &spec);
}

// ---------------------------------------------------------------- frame mode (D5)

#[test]
fn a_frame_mode_run_decides_from_the_frame_and_is_still_fully_audited() {
    let started = start(
        Metaharness::new(Kind::Claude)
            .with_decisions(DecisionMode::Frame)
            .with_frame(frame_admitting(OperationSet::of([Operation::FileRead]))),
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(call("t1", "Read")),
            ScriptStep::line(call("t2", "Bash")),
            ScriptStep::line(END),
        ],
    );
    let mut run = started.run;
    run.drain().expect("the run drains");

    let decisions: Vec<(&Decision, DecidedBy)> = run
        .events()
        .iter()
        .filter_map(|event| match event {
            Event::ToolDecided {
                decision,
                decided_by,
                ..
            } => Some((decision, *decided_by)),
            _ => None,
        })
        .collect();
    assert_eq!(decisions.len(), 2);
    assert_eq!(decisions[0].1, DecidedBy::Frame);
    assert!(matches!(decisions[0].0, Decision::Allow));
    assert!(matches!(decisions[1].0, Decision::Deny { .. }));

    // The census counts both modes, so a frame-mode run is not a run nobody audited.
    assert_eq!(run.census().allowed, 1);
    assert_eq!(run.census().denied, 1);
    assert_eq!(run.census().by_decider.get("frame"), Some(&2));
}

#[test]
fn a_frame_mode_deny_tells_the_model_which_operations_the_step_does_admit() {
    let started = start(
        Metaharness::new(Kind::Claude)
            .with_decisions(DecisionMode::Frame)
            .with_frame(frame_admitting(OperationSet::of([Operation::FileRead]))),
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(call("t1", "Bash")),
            ScriptStep::line(END),
        ],
    );
    let mut run = started.run;
    run.drain().expect("drains");
    let reason = run
        .events()
        .iter()
        .find_map(|event| match event {
            Event::ToolDecided {
                decision: Decision::Deny { reason },
                ..
            } => Some(reason.clone()),
            _ => None,
        })
        .expect("a deny");
    assert!(reason.contains("file.read"), "{reason}");
    assert!(reason.contains("shell"), "{reason}");
}

#[test]
fn frame_mode_with_no_frame_in_force_says_so_instead_of_claiming_a_frame_decided() {
    let started = start(
        Metaharness::new(Kind::Claude).with_decisions(DecisionMode::Frame),
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(call("t1", "Bash")),
            ScriptStep::line(END),
        ],
    );
    let mut run = started.run;
    run.drain().expect("drains");
    assert!(run.events().iter().any(|event| matches!(
        event,
        Event::Warning { code, .. } if code == warning::NO_FRAME_IN_FORCE
    )));
    let (decision, by, seam) = run
        .events()
        .iter()
        .find_map(|event| match event {
            Event::ToolDecided {
                decision,
                decided_by,
                seam,
                ..
            } => Some((decision.clone(), *decided_by, *seam)),
            _ => None,
        })
        .expect("a decision");
    assert_eq!(by, DecidedBy::Adapter);
    assert_eq!(seam, Seam::None);
    // Abstain and not allow. `allow` grants on this wire — it bypasses the rest of the vendor's
    // permission pipeline and overrides a stricter rule in the vendor's own settings (§ 6) — so
    // a run that had nothing to narrow with must not answer it, or the default invocation would
    // switch the vendor's permission system off by accident (amendment a3).
    assert_eq!(decision, Decision::Abstain);
    let census = run
        .events()
        .iter()
        .find_map(|event| match event {
            Event::SessionEnded { census, .. } => Some(census.clone()),
            _ => None,
        })
        .expect("a terminal record");
    assert_eq!(
        census.abstained, 1,
        "the census counts it as adjudicated by nobody"
    );
    assert_eq!(census.allowed, 0, "and not as a grant");
}

#[test]
fn a_tool_no_operation_renders_to_is_denied_because_the_frame_cannot_admit_it() {
    let started = start(
        Metaharness::new(Kind::Claude)
            .with_decisions(DecisionMode::Frame)
            .with_frame(frame_admitting(OperationSet::of([Operation::Shell]))),
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(call("t1", "SomeFutureTool")),
            ScriptStep::line(END),
        ],
    );
    let mut run = started.run;
    run.drain().expect("drains");
    assert!(run.events().iter().any(|event| matches!(
        event,
        Event::Warning { code, .. } if code == warning::UNCOVERED_TOOL
    )));
    assert_eq!(run.census().denied, 1);
}

// ---------------------------------------------------------------- § 7.7 rule 5

#[test]
fn every_pending_call_is_delivered_before_an_answer_to_any_of_them_is_due() {
    let started = start(
        Metaharness::new(Kind::Claude).with_decisions(DecisionMode::Ask),
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(call("t1", "Bash")),
            ScriptStep::line(call("t2", "Read")),
            ScriptStep::awaiting("t2"),
            ScriptStep::line(END),
        ],
    );
    let mut run = started.run;

    assert_eq!(
        run.next_event().unwrap().unwrap().event.name(),
        "session.started"
    );
    assert_eq!(
        run.next_event().unwrap().unwrap().event.name(),
        "tool.requested"
    );
    assert_eq!(
        run.next_event().unwrap().unwrap().event.name(),
        "tool.requested"
    );
    // Both are open, and neither has been answered.
    assert_eq!(run.pending_calls().len(), 2);
    assert!(
        run.pending_calls()
            .iter()
            .all(|call| call.armed_at_ms().is_some())
    );
}

#[test]
fn a_deadline_is_armed_at_delivery_so_an_embedder_is_not_timed_out_by_its_own_queue() {
    let clock = ManualClock::new();
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::new(
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(call("t1", "Bash")),
            ScriptStep::line(call("t2", "Read")),
            ScriptStep::awaiting("t1"),
            ScriptStep::line(END),
        ],
        log,
    );
    let mut seams = ScriptedSeams;
    let mut run = Metaharness::new(Kind::Claude)
        .with_decisions(DecisionMode::Ask)
        .start_with_clock(
            Input::Prompt("do the thing".to_string()),
            &mut runner,
            &mut seams,
            Box::new(clock.clone()),
        )
        .expect("starts");

    run.next_event().unwrap();
    clock.advance(1_000);
    run.next_event().unwrap(); // t1 delivered at 1000
    clock.advance(1_000);
    run.next_event().unwrap(); // t2 delivered at 2000

    let armed: Vec<Option<u64>> = run
        .pending_calls()
        .iter()
        .map(metaharness::PendingCall::armed_at_ms)
        .collect();
    assert_eq!(armed, vec![Some(1_000), Some(2_000)]);
}

#[test]
fn two_pending_calls_may_be_answered_in_the_reverse_order() {
    let started = start(
        Metaharness::new(Kind::Claude).with_decisions(DecisionMode::Ask),
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(call("t1", "Bash")),
            ScriptStep::line(call("t2", "Read")),
            ScriptStep::awaiting("t1"),
            ScriptStep::line(END),
        ],
    );
    let mut run = started.run;
    run.next_event().unwrap();
    run.next_event().unwrap();
    run.next_event().unwrap();

    let second = run
        .send(Command::ToolDecide {
            call_id: "t2".to_string(),
            decision: Decision::Allow,
        })
        .unwrap();
    let first = run
        .send(Command::ToolDecide {
            call_id: "t1".to_string(),
            decision: Decision::Deny {
                reason: "no shell here".to_string(),
            },
        })
        .unwrap();
    assert!(matches!(second, CommandOutcome::Ok { .. }));
    assert!(matches!(first, CommandOutcome::Ok { .. }));
    assert!(run.pending_calls().is_empty());
    let written = started.log.written();
    assert!(written[0].contains("\"t2\""));
    assert!(written[1].contains("\"t1\""));
}

// ---------------------------------------------------------------- § 7.7 rules 1–4

#[test]
fn a_deny_followed_by_an_interrupt_reaches_the_child_in_that_order() {
    let started = start(
        Metaharness::new(Kind::Claude).with_decisions(DecisionMode::Ask),
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(call("t1", "Bash")),
            ScriptStep::awaiting("t1"),
            ScriptStep::line(END),
        ],
    );
    let mut run = started.run;
    run.next_event().unwrap();
    run.next_event().unwrap();
    run.send(Command::ToolDecide {
        call_id: "t1".to_string(),
        decision: Decision::Deny {
            reason: "not in this step".to_string(),
        },
    })
    .unwrap();
    run.send(Command::Interrupt {
        reason: "enough".to_string(),
    })
    .unwrap();

    let written = started.log.written();
    assert_eq!(written.len(), 2);
    assert!(written[0].contains("deny"));
    assert!(written[1].contains("interrupt"));
}

#[test]
fn a_decision_for_a_call_that_was_never_presented_is_unknown_call() {
    let started = start(
        Metaharness::new(Kind::Claude).with_decisions(DecisionMode::Ask),
        vec![ScriptStep::line(INIT), ScriptStep::line(END)],
    );
    let mut run = started.run;
    let outcome = run
        .send(Command::ToolDecide {
            call_id: "ghost".to_string(),
            decision: Decision::Allow,
        })
        .unwrap();
    assert!(matches!(
        outcome,
        CommandOutcome::Refused { refused } if refused.code == RefusalCode::UnknownCall
    ));
}

#[test]
fn a_deny_with_an_empty_reason_is_malformed_because_the_reason_is_the_instruction() {
    let started = start(
        Metaharness::new(Kind::Claude).with_decisions(DecisionMode::Ask),
        vec![ScriptStep::line(INIT), ScriptStep::line(END)],
    );
    let mut run = started.run;
    let outcome = run
        .send(Command::ToolDecide {
            call_id: "anything".to_string(),
            decision: Decision::Deny {
                reason: "  ".to_string(),
            },
        })
        .unwrap();
    assert!(matches!(
        outcome,
        CommandOutcome::Refused { refused } if refused.code == RefusalCode::Malformed
    ));
}

#[test]
fn the_same_call_id_presented_with_a_different_input_is_refused_and_not_carried_over() {
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::new(
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(
                r#"{"emit":"tool.requested","call_id":"t1","name":"Bash","input":{"command":"ls"}}"#,
            ),
            ScriptStep::line(
                r#"{"emit":"tool.requested","call_id":"t1","name":"Bash","input":{"command":"rm -rf /"}}"#,
            ),
            ScriptStep::line(END),
        ],
        log.clone(),
    );
    let mut seams = ScriptedSeams;
    let mut run = Metaharness::new(Kind::Claude)
        .with_decisions(DecisionMode::Frame)
        .with_frame(frame_admitting(OperationSet::of([Operation::Shell])))
        .start_with_clock(
            Input::Prompt("do the thing".to_string()),
            &mut runner,
            &mut seams,
            Box::new(ManualClock::new()),
        )
        .expect("starts");
    run.drain().expect("drains");

    assert!(run.events().iter().any(|event| matches!(
        event,
        Event::Warning { code, .. } if code == warning::REQUEST_MUTATED
    )));
    // The first presentation was admitted; the mutated one was refused by name.
    assert_eq!(run.census().allowed, 1);
    assert_eq!(run.census().denied, 1);
}

#[test]
fn the_correlation_key_changes_when_the_input_changes() {
    let one = request_digest("t1", "Bash", &serde_json::json!({"command": "ls"}));
    let two = request_digest("t1", "Bash", &serde_json::json!({"command": "rm"}));
    assert_ne!(one, two);
    assert_eq!(
        one,
        request_digest("t1", "Bash", &serde_json::json!({"command": "ls"}))
    );
}

#[test]
fn halting_writes_the_pending_decision_first_then_stops_the_child() {
    let started = start(
        Metaharness::new(Kind::Claude).with_decisions(DecisionMode::Ask),
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(call("t1", "Bash")),
            ScriptStep::awaiting("t1"),
            ScriptStep::line(END),
        ],
    );
    let mut run = started.run;
    run.next_event().unwrap();
    run.next_event().unwrap();
    run.send(Command::Halt {
        reason: "stop".to_string(),
    })
    .unwrap();
    run.drain().unwrap();

    let written = started.log.written();
    assert!(written[0].contains("deny"));
    assert!(written[1].contains("halt"));
    assert!(started.log.killed());
    // The harness never produced a terminal record, and that is exit 3, not exit 0.
    assert!(!run.saw_terminal_record());
}

// ---------------------------------------------------------------- deadlines (§ 7.7 rule 2)

#[test]
fn metaharnesss_own_deadline_is_strictly_less_than_the_vendors_timeout() {
    for vendor in [1_u64, 100, 5_000, 5_001, 30_000, 600_000] {
        assert!(
            metaharness_deadline_ms(vendor) < vendor,
            "vendor {vendor} gave {}",
            metaharness_deadline_ms(vendor)
        );
    }
}

#[test]
fn the_vendor_timeout_is_read_from_the_hook_definition_in_seconds() {
    let hook = serde_json::json!({
        "PreToolUse": [{ "matcher": "", "hooks": [{ "type": "command", "timeout": 12 }] }]
    });
    assert_eq!(vendor_hook_timeout_ms(&hook), 12_000);
    assert_eq!(
        vendor_hook_timeout_ms(&serde_json::json!({})),
        metaharness::DEFAULT_VENDOR_HOOK_TIMEOUT_MS
    );
}

#[test]
fn an_unanswered_call_is_denied_by_metaharness_and_the_record_says_who_decided() {
    let started = start(
        Metaharness::new(Kind::Claude).with_decisions(DecisionMode::Ask),
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(call("t1", "Bash")),
            ScriptStep::awaiting("t1"),
            ScriptStep::line(END),
        ],
    );
    let mut run = started.run;
    run.drain().expect("the deadline unblocks the child");
    assert_eq!(run.census().denied, 1);
    assert_eq!(run.census().by_decider.get("deadline"), Some(&1));
}

// ---------------------------------------------------------------- the record

#[test]
fn a_line_the_seam_cannot_read_becomes_opaque_and_is_never_dropped() {
    let started = start(
        Metaharness::new(Kind::Claude),
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line("this is not JSON at all"),
            ScriptStep::line(r#"{"emit":"something.new","weird":true}"#),
            ScriptStep::line(END),
        ],
    );
    let mut run = started.run;
    run.drain().expect("drains");
    assert_eq!(
        names(&run),
        vec!["session.started", "opaque", "opaque", "session.ended"]
    );
}

#[test]
fn an_expired_credential_is_its_own_event_and_does_not_end_the_run() {
    let started = start(
        Metaharness::new(Kind::Claude),
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(
                r#"{"emit":"auth.expired","credential_source":"operator-login","detail":"OAuth session expired"}"#,
            ),
            ScriptStep::line(END),
        ],
    );
    let mut run = started.run;
    run.drain().expect("drains");
    assert_eq!(
        names(&run),
        vec!["session.started", "auth.expired", "session.ended"]
    );
    assert!(run.saw_terminal_record());
}

#[test]
fn sequence_numbers_come_from_the_protocols_own_stream_and_are_monotone_from_one() {
    let started = start(
        Metaharness::new(Kind::Claude),
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(r#"{"emit":"text","text":"hello"}"#),
            ScriptStep::line(END),
        ],
    );
    let mut run = started.run;
    let lines = run.drain().expect("drains");
    let sequence: Vec<u64> = lines.iter().map(|line| line.seq).collect();
    assert_eq!(sequence, vec![1, 2, 3]);
    assert!(
        lines
            .iter()
            .all(|line| line.format == "metaharness.event/1")
    );
}

#[test]
fn a_command_always_produces_exactly_one_result_even_when_it_is_refused() {
    let started = start(
        Metaharness::new(Kind::Claude),
        vec![ScriptStep::line(INIT), ScriptStep::line(END)],
    );
    let mut run = started.run;
    run.send(Command::Steer {
        text: "turn left".to_string(),
    })
    .unwrap();
    run.drain().unwrap();
    let results: Vec<&Event> = run
        .events()
        .iter()
        .filter(|event| matches!(event, Event::CommandResult { .. }))
        .collect();
    assert_eq!(results.len(), 1);
    assert!(matches!(
        results[0],
        Event::CommandResult {
            outcome: CommandOutcome::Refused { .. },
            ..
        }
    ));
}

#[test]
fn steer_is_refused_by_name_on_claude_because_it_does_not_exist_headless() {
    let started = start(
        Metaharness::new(Kind::Claude),
        vec![ScriptStep::line(INIT), ScriptStep::line(END)],
    );
    let mut run = started.run;
    let outcome = run
        .send(Command::Steer {
            text: "turn left".to_string(),
        })
        .unwrap();
    assert!(matches!(
        outcome,
        CommandOutcome::Refused { refused } if refused.code == RefusalCode::UnsupportedControl
    ));
}

#[test]
fn a_frame_mutated_after_it_was_sealed_is_refused_as_malformed() {
    let started = start(
        Metaharness::new(Kind::Claude),
        vec![ScriptStep::line(INIT), ScriptStep::line(END)],
    );
    let mut run = started.run;
    let mut frame = frame_admitting(OperationSet::of([Operation::FileRead])).seal();
    frame.operations = OperationSet::of([Operation::Shell]);
    let outcome = run
        .send(Command::FrameSet {
            frame: Box::new(frame),
        })
        .unwrap();
    assert!(matches!(
        outcome,
        CommandOutcome::Refused { refused } if refused.code == RefusalCode::Malformed
    ));
}

/// `frame.set` is not partially deliverable: an adapter that can inject the text but cannot
/// enforce it would tell the model "strictly only these operations" and make it false, so it is
/// refused by name rather than half-delivered (design § 6, finding F9).
#[test]
fn setting_a_frame_is_refused_by_name_while_the_adapter_delivers_no_turn_tier() {
    let started = start(
        Metaharness::new(Kind::Claude),
        vec![ScriptStep::line(INIT), ScriptStep::line(END)],
    );
    let mut run = started.run;
    let outcome = run
        .send(Command::FrameSet {
            frame: Box::new(frame_admitting(OperationSet::of([Operation::Shell])).seal()),
        })
        .unwrap();
    assert!(matches!(
        outcome,
        CommandOutcome::Refused { refused } if refused.code == RefusalCode::UnsupportedControl
    ));
}

// ---------------------------------------------------------------- Q13, the credential copy

#[test]
fn the_credential_is_copied_at_the_spawn_and_once_per_spawn() {
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::new(vec![ScriptStep::line(END)], log.clone());
    let mut seams = ScriptedSeams;

    for _ in 0..2 {
        let _run = Metaharness::new(Kind::Claude)
            .with_credentials(CredentialSource::OperatorLogin)
            .start_with_clock(
                Input::Prompt("do the thing".to_string()),
                &mut runner,
                &mut seams,
                Box::new(ManualClock::new()),
            )
            .expect("starts");
    }
    assert_eq!(log.spawns(), 2);
    assert_eq!(
        log.credential_copies().len(),
        2,
        "a copied operator-login token ages out, so it is re-copied at every spawn (Q13)"
    );
}

#[test]
fn a_run_that_declared_no_credential_copies_nothing() {
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::new(vec![ScriptStep::line(END)], log.clone());
    let mut seams = ScriptedSeams;
    let _run = Metaharness::new(Kind::Claude)
        .with_credentials(CredentialSource::None)
        .start_with_clock(
            Input::Prompt("do the thing".to_string()),
            &mut runner,
            &mut seams,
            Box::new(ManualClock::new()),
        )
        .expect("starts");
    assert!(log.credential_copies().is_empty());
}

#[test]
fn the_run_is_launched_with_the_argv_the_adapter_planned() {
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::new(vec![ScriptStep::line(END)], log.clone());
    let mut seams = ScriptedSeams;
    let _run = Metaharness::new(Kind::Claude)
        .with_prompt("tidy up")
        .start_with_clock(
            Input::FromSpec,
            &mut runner,
            &mut seams,
            Box::new(ManualClock::new()),
        )
        .expect("starts");
    let argv = &log.launched()[0];
    assert_eq!(argv[0], "claude");
    assert!(argv.contains(&"tidy up".to_string()));
}

// ---------------------------------------------------------------- the real adapter seam

#[test]
fn the_claude_seam_reads_the_vendors_own_stream_json_into_protocol_events() {
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::new(
        vec![
            ScriptStep::line(
                r#"{"type":"system","subtype":"init","version":"2.1.239","cwd":"/w","tools":["Bash"]}"#,
            ),
            ScriptStep::line(
                r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_1","name":"Bash","input":{"command":"ls"}}]}}"#,
            ),
            ScriptStep::line(
                r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_1","is_error":false}]}}"#,
            ),
            ScriptStep::line(r#"{"type":"result","subtype":"success","is_error":false}"#),
        ],
        log,
    );
    let mut seams = ClaudeSeams;
    let mut run = Metaharness::new(Kind::Claude)
        .with_decisions(DecisionMode::Frame)
        .with_frame(frame_admitting(OperationSet::of([Operation::Shell])))
        .start_with_clock(
            Input::Prompt("do the thing".to_string()),
            &mut runner,
            &mut seams,
            Box::new(ManualClock::new()),
        )
        .expect("starts");
    run.drain().expect("drains");

    assert_eq!(
        names(&run),
        vec![
            "session.started",
            "tool.requested",
            "tool.decided",
            "tool.result",
            "session.ended",
        ],
        "if this fails the adapter's transcript shape moved and the fixture, not the loop, is stale"
    );
    assert_eq!(run.census().allowed, 1);
}

#[test]
fn the_same_call_presented_twice_byte_for_byte_is_recorded_twice_and_decided_once() {
    let started = start(
        Metaharness::new(Kind::Claude)
            .with_decisions(DecisionMode::Frame)
            .with_frame(frame_admitting(OperationSet::of([Operation::Shell]))),
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(call("t1", "Bash")),
            ScriptStep::line(call("t1", "Bash")),
            ScriptStep::line(END),
        ],
    );
    let mut run = started.run;
    run.drain().expect("drains");

    // Nothing is dropped: both presentations are in the record.
    assert_eq!(
        names(&run)
            .iter()
            .filter(|name| **name == "tool.requested")
            .count(),
        2
    );
    // One call, one decision: a second would double the census for one call.
    assert_eq!(run.census().allowed, 1);
    assert_eq!(started.log.written().len(), 1);
}

#[test]
fn a_call_the_turn_ended_on_is_too_late_and_the_run_says_it_was_abandoned() {
    let started = start(
        Metaharness::new(Kind::Claude).with_decisions(DecisionMode::Ask),
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(call("t1", "Bash")),
            ScriptStep::line(r#"{"emit":"turn.ended","turn":1,"stop_reason":"end_turn"}"#),
            ScriptStep::line(END),
        ],
    );
    let mut run = started.run;
    run.next_event().unwrap();
    run.next_event().unwrap();
    run.next_event().unwrap(); // tool.decided, from the window closing
    let outcome = run
        .send(Command::ToolDecide {
            call_id: "t1".to_string(),
            decision: Decision::Allow,
        })
        .unwrap();
    assert!(matches!(
        outcome,
        CommandOutcome::Refused { refused } if refused.code == RefusalCode::TooLate
    ));
    run.drain().unwrap();
    assert!(run.events().iter().any(|event| matches!(
        event,
        Event::Warning { code, .. } if code == warning::PENDING_CALL_ABANDONED
    )));
}
