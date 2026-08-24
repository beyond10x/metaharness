//! What the run loop promises, asserted through the scripted fake.
//!
//! Every test here is named as the property it holds, because a test called `test_run_2` tells
//! a later reader nothing about which guarantee they just broke.
//!
//! # No test in this file may start a session
//!
//! [`Metaharness::start`] spawns a real vendor binary and bills a real account — `claude` since
//! M2 and `codex` since CX-M2. Every `start(…)` below either hands in a
//! [`metaharness::ScriptedRunner`] (free, no process, no model) or is a **refusal raised before
//! the spawn**, and `no_start_in_this_file_can_reach_a_spawn` holds that line mechanically over
//! this file's own source. Paid runs live in `tests/live.rs` and `tests/live_codex.rs`, behind
//! `METAHARNESS_LIVE=1`.

use metaharness::protocol::{
    Command, CommandOutcome, CredentialSource, DecidedBy, Decision, DecisionMode, Digest, Event,
    EvidenceLine, Frame, Handoff, HermeticMode, Kind, NodeRef, Operation, OperationSet,
    RefusalCode, RunSpec, Seam, StepRef, SubjectScope, ToolSurface, WorkflowRef,
};
use metaharness::{
    ClaudeSeams, Input, ManualClock, Metaharness, Refusal, Run, ScriptStep, ScriptedLog,
    ScriptedRunner, ScriptedSeams, capabilities, metaharness_deadline_ms, request_digest,
    start_refusals, vendor_hook_timeout_ms, warning,
};

const INIT: &str = r#"{"emit":"session.started","harness_version":"2.1.240","output_style":"default","plugins":[],"mcp_servers":[],"credential_source":"operator-login","inputs_digest":"tree"}"#;
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
        subjects: SubjectScope::default(),
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

/// The interlock, over this file's own source.
///
/// `Metaharness::start` is the **real** spawner on both adapters now. A call to it here is only
/// free if the very next thing the test does is `expect_err` — that is, if the run is refused
/// before a process exists. A call whose result is unwrapped is a call that spawned, and this test
/// refuses to let one in.
#[test]
fn no_start_in_this_file_can_reach_a_spawn() {
    let source = include_str!("run_loop.rs");
    // Split so the needle never appears whole in this file: a guard that matched its own source
    // would report itself and hide the line it exists to find.
    let needle = concat!(".start(", "Input::");
    let lines: Vec<&str> = source.lines().collect();
    for (index, line) in lines.iter().enumerate() {
        // `start_with_clock` and `start_refusals` are different functions and are free; the
        // argument is what pins this to the spawning one.
        if !line.contains(needle) {
            continue;
        }
        let next = lines
            .get(index + 1)
            .map(|line| line.trim())
            .unwrap_or_default();
        assert!(
            next.starts_with(".expect_err("),
            "a start whose result is not an expected refusal spawns the vendor and bills for it: \
             {line}"
        );
    }
}

/// **Since CX-M2 `Kind::Codex` spawns a real, paid `codex exec`.** So the codex refusal this file
/// may assert for free is a *pre-spawn* one, and this is it: a run with no prompt is a paid call
/// for no observation, and the adapter says so before it starts anything.
#[test]
fn a_codex_run_with_no_prompt_is_refused_before_anything_is_spawned() {
    let refusal = Metaharness::new(Kind::Codex)
        .start(Input::FromSpec)
        .expect_err("a run with nothing to do is refused");
    assert!(matches!(refusal, Refusal::Launch { .. }), "{refusal}");
    assert!(refusal.to_string().contains("no prompt"), "{refusal}");
}

/// An option `codex exec` cannot express is refused **by name** and never dropped, and this
/// refusal is also free: it is raised while the launch is being planned, before the spawn.
#[test]
fn an_option_the_codex_surface_cannot_carry_is_refused_by_name() {
    let refusal = Metaharness::new(Kind::Codex)
        .with_prompt("x")
        .with_max_turns(3)
        .start(Input::FromSpec)
        .expect_err("codex exec has no turn ceiling");
    assert!(matches!(refusal, Refusal::Launch { .. }), "{refusal}");
    assert!(refusal.to_string().contains("--max-turns"), "{refusal}");
}

#[test]
fn a_missing_frame_document_is_refused_naming_the_path() {
    let refusal = Metaharness::new(Kind::Claude)
        .with_frame_file("frames/absent.frame.json")
        .start(Input::FromSpec)
        .expect_err("an unreadable frame document is refused");
    assert!(matches!(refusal, Refusal::FrameUnreadable { .. }));
    assert!(refusal.to_string().contains("frames/absent.frame.json"));
}

/// The file boundary must not weaken the digest rule: a document edited after sealing is the
/// on-disk spelling of a frame mutated after the model saw it.
#[test]
fn a_frame_document_whose_digest_lies_is_refused_before_anything_runs() {
    let directory = tempfile::tempdir().expect("a scratch directory");
    let path = directory.path().join("edited.frame.json");
    let document = frame_admitting(OperationSet::of([Operation::FileRead]))
        .seal()
        .to_document()
        .replace("the specification is approved", "something else entirely");
    std::fs::write(&path, document).expect("written");

    let refusal = Metaharness::new(Kind::Claude)
        .with_frame_file(&path)
        .start(Input::FromSpec)
        .expect_err("a digest that lies is refused");
    assert!(matches!(refusal, Refusal::FrameInvalid { .. }));
    assert!(refusal.to_string().contains("digest"), "{refusal}");
}

/// One frame in force, from exactly one place: both spellings at once is a refusal, not a
/// precedence rule the loser never learns about.
#[test]
fn an_in_memory_frame_and_a_frame_document_together_are_refused_by_name() {
    let directory = tempfile::tempdir().expect("a scratch directory");
    let path = directory.path().join("other.frame.json");
    std::fs::write(
        &path,
        frame_admitting(OperationSet::of([Operation::FileRead]))
            .seal()
            .to_document(),
    )
    .expect("written");

    let refusal = Metaharness::new(Kind::Claude)
        .with_frame(frame_admitting(OperationSet::of([Operation::Shell])))
        .with_frame_file(&path)
        .start(Input::FromSpec)
        .expect_err("two frames are refused");
    assert!(matches!(refusal, Refusal::FrameConflict { .. }));
}

#[test]
fn an_owned_tool_surface_is_refused_for_a_kind_whose_tools_cannot_be_taken_away() {
    // Not "unbuilt" any more — `metaharness mcp-serve` serves the tools. What is missing is a
    // vendor surface to put them on, and the refusal names which vendor and why.
    for kind in [Kind::Codex, Kind::B10x] {
        let refusal = Metaharness::new(kind)
            .with_tool_surface(ToolSurface::Owned)
            .start(Input::FromSpec)
            .expect_err("no surface to replace");
        assert_eq!(refusal, Refusal::ToolSurfaceOwned { kind });
        let said = refusal.to_string();
        assert!(said.contains(kind.as_str()), "{said}");
    }
}

/// A caller with two problems should learn about the one they can fix — and should learn it
/// **before** anything is spawned, because a refusal that costs a process start is a refusal that
/// costs money on the run after this one.
#[test]
fn a_bad_spec_is_refused_before_anything_is_spawned() {
    let refusal = Metaharness::new(Kind::Codex)
        .with_frame_file("x.yaml")
        .with_prompt("this must never reach a spawn")
        .start(Input::FromSpec)
        .expect_err("refused");
    // The frame document is resolved **above** the kind dispatch, so an unreadable one is a free
    // refusal on every adapter and the prompt beside it never costs anything.
    assert!(
        matches!(refusal, Refusal::FrameUnreadable { .. }),
        "{refusal}"
    );
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
        .with_model_endpoint("https://llmgw.example")
        .with_effort("medium")
        .with_max_turns(30)
        .with_plugin_dir("plugins/one")
        .with_cwd("/operator/repo")
        .with_retain_dir("/operator/keep")
        .with_strict_version(true)
        .with_audit(true)
        .with_spec_file("expectations.yaml")
        .with_auditor("protocol trace check")
        .with_prices("rates.json")
        .with_substrate("/run/substrate.sock")
        .with_cgroup_root("/sys/fs/cgroup/run.slice")
        .with_toolchain("rust")
        .with_auditor_arg("--advisory")
        .with_auditor_arg("billed-to-the-session");

    let expected = RunSpec {
        kind: Kind::Claude,
        hermetic: HermeticMode::Strict,
        prompt: Some("p".to_string()),
        frame: Some("f.yaml".into()),
        decisions: DecisionMode::Ask,
        tool_surface: ToolSurface::Owned,
        allow_program: Vec::new(),
        credentials: CredentialSource::ApiKey,
        model: Some("sonnet".to_string()),
        model_endpoint: Some("https://llmgw.example".to_string()),
        effort: Some("medium".to_string()),
        max_turns: Some(30),
        plugin_dir: vec!["plugins/one".into()],
        cwd: Some("/operator/repo".into()),
        retain_dir: Some("/operator/keep".into()),
        strict_version: true,
        audit: true,
        spec: Some("expectations.yaml".into()),
        substrate: Some("/run/substrate.sock".into()),
        substrate_embedded: false,
        cgroup_root: Some("/sys/fs/cgroup/run.slice".into()),
        toolchain: Some("rust".to_string()),
        write_scope: Vec::new(),
        scope_announce: metaharness_protocol::ScopeAnnounce::Stated,
        context: Vec::new(),
        prices: Some("rates.json".into()),
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

/// The document is the same frame by construction, so it must be the same run: same decisions,
/// same deciders, same census. A file that behaved differently from the value it spells would
/// make the CLI face weaker than the library face.
#[test]
fn a_frame_document_drives_frame_mode_exactly_like_the_in_memory_frame() {
    let directory = tempfile::tempdir().expect("a scratch directory");
    let path = directory.path().join("implement.frame.json");
    std::fs::write(
        &path,
        frame_admitting(OperationSet::of([Operation::FileRead]))
            .seal()
            .to_document(),
    )
    .expect("written");

    let started = start(
        Metaharness::new(Kind::Claude)
            .with_decisions(DecisionMode::Frame)
            .with_frame_file(&path),
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(call("t1", "Read")),
            ScriptStep::line(call("t2", "Bash")),
            ScriptStep::line(END),
        ],
    );
    let mut run = started.run;
    run.drain().expect("the run drains");

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
                r#"{"type":"system","subtype":"init","version":"2.1.240","cwd":"/w","tools":["Bash"]}"#,
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

// ---------------------------------------------------------------- one vocabulary, every harness

/// One call, as it arrives on a wire, with an input the caller chooses.
fn requested_with(tool: &str, input: &str) -> String {
    format!(r#"{{"emit":"tool.requested","call_id":"c1","name":"{tool}","input":{input}}}"#)
}

/// Runs one scripted call and answers the `operations` its `tool.requested` carried.
fn operations_of(surface: ToolSurface, frame: Option<Frame>, line: &str) -> Vec<String> {
    let mut builder = Metaharness::new(Kind::Claude)
        .with_tool_surface(surface)
        .with_decisions(DecisionMode::Frame);
    if let Some(frame) = frame {
        builder = builder.with_frame(frame);
    }
    let mut started = start(
        builder,
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(line),
            ScriptStep::line(END),
        ],
    );
    started.run.drain().expect("pumps");
    started
        .run
        .events()
        .iter()
        .find_map(|event| match event {
            Event::ToolRequested { operations, .. } => Some(operations.clone()),
            _ => None,
        })
        .expect("the call was recorded")
}

/// The blindness, closed. Two harness vocabularies, one answer a consumer selects on.
///
/// The corpus in `engineering-protocols/conformance/eval/` selected on the **vendor's** tool name,
/// so it was written in Claude Code's and could not see a b10x run at all. Two patches widened its
/// write-set with `workspace_write` and `workspace_edit`, which put more vendor names into a
/// document that should hold none. This field is what ends that.
#[test]
fn one_act_reads_as_one_operation_whatever_the_harness_called_the_tool() {
    assert_eq!(
        operations_of(
            ToolSurface::Native,
            None,
            &requested_with("Write", r#"{"file_path":"a.rs","content":"x"}"#)
        ),
        vec!["file.write".to_string()],
        "native: resolved through the adapter's own published rendering"
    );

    assert_eq!(
        operations_of(
            ToolSurface::Owned,
            None,
            &requested_with(
                "mcp__metaharness__tool_invoke",
                r#"{"name":"file_write","arguments":{"path":"a.rs","text":"x"}}"#
            )
        ),
        vec!["file.write".to_string()],
        "owned: the entry is inside the call, where no rendering table can see it"
    );
}

/// A `native` run must not have an invented `tool_invoke` read as whatever entry it names.
///
/// That would launder an unknown call into a recognised operation, in the record a judge reads —
/// and the tool it claims to be does not exist in that run.
#[test]
fn the_verb_road_is_taken_only_by_a_run_that_actually_published_the_verbs() {
    assert!(
        operations_of(
            ToolSurface::Native,
            None,
            &requested_with(
                "tool_invoke",
                r#"{"name":"run","arguments":{"argv":["sh"]}}"#
            )
        )
        .is_empty(),
        "no operation, because this run had no such tool"
    );
}

/// A frame admits a question about the catalogue and still refuses an act it does not admit.
///
/// Before this, an owned-surface run under **any** frame denied every call: the verb rendered to
/// no operation, and the uncovered-tool road is a denial. So the arm that was built to have no
/// shell also had no file tools, and would have measured as a model that would not do the task.
#[test]
fn a_frame_admits_asking_what_tools_exist_and_still_judges_the_act_by_what_it_is() {
    let mut started = start(
        Metaharness::new(Kind::Claude)
            .with_tool_surface(ToolSurface::Owned)
            .with_decisions(DecisionMode::Frame)
            .with_frame(frame_admitting(OperationSet::of([Operation::FileRead]))),
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(requested_with("mcp__metaharness__tool_search", "{}")),
            ScriptStep::line(END),
        ],
    );
    started.run.drain().expect("pumps");

    let decided: Vec<&Decision> = started
        .run
        .events()
        .iter()
        .filter_map(|event| match event {
            Event::ToolDecided { decision, .. } => Some(decision),
            _ => None,
        })
        .collect();
    assert!(
        !decided.is_empty()
            && decided
                .iter()
                .all(|decision| matches!(decision, Decision::Allow)),
        "listing the catalogue is not an act a frame can narrow: {decided:?}"
    );

    // …and the act itself is still named, so a denial is a denial *of something*.
    assert_eq!(
        operations_of(
            ToolSurface::Owned,
            Some(frame_admitting(OperationSet::of([Operation::FileRead]))),
            &requested_with(
                "mcp__metaharness__tool_invoke",
                r#"{"name":"file_write","arguments":{"path":"a","text":"x"}}"#
            )
        ),
        vec!["file.write".to_string()]
    );
}

/// A run whose child produced nothing, having said this on the way out.
fn silent_run(said: &str) -> Run {
    let mut runner = ScriptedRunner::new(Vec::new(), ScriptedLog::new()).saying_on_stderr(said);
    let mut seams = ScriptedSeams;
    Metaharness::new(Kind::Claude)
        .start_with_clock(
            Input::Prompt("do the thing".to_string()),
            &mut runner,
            &mut seams,
            Box::new(ManualClock::new()),
        )
        .expect("the run starts")
}

fn warned(run: &mut Run, code: &str) -> Option<String> {
    while run.next_event().expect("the stream drains").is_some() {}
    run.events().iter().find_map(|event| match event {
        Event::Warning {
            code: seen,
            message,
        } if seen == code => Some(message.clone()),
        _ => None,
    })
}

#[test]
fn a_run_that_produced_no_record_says_what_the_child_said_on_the_way_out() {
    // The silent exit 3 this closes: a b10x launch died on one bad argument, wrote one sentence to
    // stderr and nothing to stdout, and metaharness reported *nobody found out* with both streams
    // empty. The spawner had been retaining stderr for exactly this since it was written, and
    // nothing read it.
    let mut run = silent_run("error: unexpected argument '--nope' found");
    let warning = warned(&mut run, "NO_TERMINAL_RECORD").expect("the run says why");

    assert!(!run.saw_terminal_record());
    assert!(warning.contains("--nope"), "{warning}");
}

#[test]
fn a_child_that_was_silent_too_gets_no_invented_explanation() {
    // No stderr means no warning: "it said nothing" is already what an empty record says, and a
    // warning carrying an empty string would be noise wearing the shape of a finding.
    let mut run = silent_run("   ");
    assert!(warned(&mut run, "NO_TERMINAL_RECORD").is_none());
}

// --- a step's write scope, enforced at the seam --------------------------------------------------

/// A frame admitting writes everywhere, then narrowed to the planning store's own rule.
fn frame_scoped_to_the_store() -> Frame {
    let mut frame = frame_admitting(OperationSet::of([
        Operation::FileWrite,
        Operation::FileEdit,
        Operation::FileRead,
    ]));
    frame.subjects = metaharness::protocol::SubjectScope {
        rules: vec![
            metaharness::protocol::SubjectRule {
                subjects: vec!["file:.engineering/planning/**".to_owned()],
                operations: OperationSet::of([Operation::FileEdit, Operation::FileRead]),
            },
            metaharness::protocol::SubjectRule {
                subjects: vec!["**".to_owned()],
                operations: OperationSet::of([
                    Operation::FileWrite,
                    Operation::FileEdit,
                    Operation::FileRead,
                ]),
            },
        ],
    };
    frame.seal()
}

/// One `tool.requested` on the vendor road: its own tool name, its own path argument.
///
/// Deliberately not the three-verb form. The operation and the subject are both resolved by the run
/// from what this harness records, which is the road a Claude Code arm actually takes — and the
/// point of a scope is that it works there too.
fn scoped_call(id: &str, tool: &str, path: &str) -> String {
    format!(
        r#"{{"emit":"tool.requested","call_id":"{id}","name":"{tool}","input":{{"file_path":"{path}"}},"decision_required":true,"seam":"hook"}}"#
    )
}

fn decisions(run: &mut Run) -> Vec<(String, Decision)> {
    while run.next_event().expect("the stream drains").is_some() {}
    run.events()
        .iter()
        .filter_map(|event| match event {
            Event::ToolDecided {
                call_id, decision, ..
            } => Some((call_id.clone(), decision.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn a_write_the_step_admits_is_refused_on_a_path_it_does_not_own() {
    // The rule no `OperationSet` can express: both are writes, and under the planning store one is
    // right and the other re-types frontmatter the CLI owns. `crates/protocol-cli/src/drive.rs`
    // has enforced it for a year in Claude Code's tool names, so every other arm walked past it.
    let mut started = start(
        Metaharness::new(Kind::Claude)
            .with_decisions(DecisionMode::Frame)
            .with_frame(frame_scoped_to_the_store()),
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(scoped_call(
                "c1",
                "Write",
                ".engineering/planning/story/a.md",
            )),
            ScriptStep::line(scoped_call(
                "c2",
                "Edit",
                ".engineering/planning/story/a.md",
            )),
            ScriptStep::line(scoped_call(
                "c3",
                "Write",
                "crates/protocol-cli/src/planning.rs",
            )),
            ScriptStep::line(END),
        ],
    );
    let decided = decisions(&mut started.run);

    let by_id = |id: &str| {
        decided
            .iter()
            .find(|(call, _)| call == id)
            .map(|(_, decision)| decision.clone())
            .expect("every call is decided")
    };
    assert!(
        matches!(by_id("c1"), Decision::Deny { .. }),
        "a whole-file write under the store is refused"
    );
    assert_eq!(by_id("c2"), Decision::Allow, "an edit there is the way in");
    assert_eq!(
        by_id("c3"),
        Decision::Allow,
        "and the same operation is fine where the step owns the path"
    );
}

#[test]
fn the_refusal_says_what_would_work_instead() {
    // A denial that says only "denied" gets retried until the turn budget runs out, which is money
    // spent on a wall. Where the scope admits a narrower operation on the same path, it says which.
    let mut started = start(
        Metaharness::new(Kind::Claude)
            .with_decisions(DecisionMode::Frame)
            .with_frame(frame_scoped_to_the_store()),
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(scoped_call(
                "c1",
                "Write",
                ".engineering/planning/story/a.md",
            )),
            ScriptStep::line(END),
        ],
    );
    let decided = decisions(&mut started.run);
    let Decision::Deny { reason } = &decided[0].1 else {
        panic!("refused");
    };
    assert!(
        reason.contains(".engineering/planning/story/a.md"),
        "{reason}"
    );
    assert!(reason.contains("file.edit"), "names the way in: {reason}");
}

#[test]
fn a_call_that_named_no_subject_is_decided_by_the_operation_set_alone() {
    // Silence is not a violation. Refusing a call the harness could not describe would deny work
    // for being unobservable rather than for being wrong.
    let mut started = start(
        Metaharness::new(Kind::Claude)
            .with_decisions(DecisionMode::Frame)
            .with_frame(frame_scoped_to_the_store()),
        vec![
            ScriptStep::line(INIT),
            ScriptStep::line(
                r#"{"emit":"tool.requested","call_id":"c1","name":"Write","input":{},"decision_required":true,"seam":"hook"}"#,
            ),
            ScriptStep::line(END),
        ],
    );
    assert_eq!(decisions(&mut started.run)[0].1, Decision::Allow);
}
