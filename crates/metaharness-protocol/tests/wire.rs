//! The wire, exercised one variant at a time.
//!
//! Every event and every command is built, framed, written, read back and compared. A
//! vocabulary whose round trip is only tested for the variants somebody remembered is a
//! vocabulary with untested variants, and the untested one is always the one that carries the
//! denial.

use std::collections::BTreeMap;

use metaharness_protocol::{
    COMMAND_FORMAT, COMMAND_NAMES, CONTROL_PLANE_EVENTS, Command, CommandLine, CommandOutcome,
    DecidedBy, Decision, DecisionCensus, DecisionMode, Digest, EVENT_FORMAT, EVENT_NAMES, Emission,
    EntityList, Event, EventStream, EvidenceLine, Frame, FramingError, Handoff,
    HermeticAttestation, HermeticMode, ImposedControl, Kind, Line, McpServerRef, NodeRef,
    Operation, OperationSet, PermissionDenial, PluginRef, RateLimitInfo, RefusalCode, Refused,
    RunId, RunSpec, Seam, StepOutcome, StepRef, SubjectScope, ToolSurface, TranscriptRef, Usage,
    WorkflowRef, ir_family, parse_command_line, parse_event_line, project, required_commands,
    warning_code,
};
use serde_json::json;

fn step() -> StepRef {
    StepRef {
        workflow: "development/default".into(),
        state: "implement".into(),
        index: 2,
        attempt: 1,
    }
}

fn frame() -> Frame {
    Frame {
        workflow: WorkflowRef {
            id: "development/default".into(),
            version: "1".into(),
        },
        node: NodeRef {
            id: "implement".into(),
        },
        step: step(),
        prior: vec![EvidenceLine {
            text: "the specification is approved".into(),
            source: Some("docs/specs/one.md".into()),
        }],
        obligations: vec![Line {
            text: "the suite is red first".into(),
            asked_by: Some("workflows/development/default.yaml".into()),
        }],
        reaching: vec![Line {
            text: "to verify: the suite is green".into(),
            asked_by: None,
        }],
        next: vec![NodeRef {
            id: "verify".into(),
        }],
        handoff: Handoff::StructuredAnswer {
            schema: "verdict/1".into(),
        },
        subjects: SubjectScope::default(),
        operations: OperationSet::of([
            Operation::FileEdit,
            Operation::Shell,
            Operation::McpCall {
                server: "s".into(),
                tool: "t".into(),
            },
        ]),
        entities: Some(EntityList {
            source: "protocol artifact kinds".into(),
            members: vec!["spec".into(), "plan".into()],
        }),
        digest: Digest::of(b""),
    }
    .seal()
}

/// The two lifecycle events, whose payloads are the IR's whole field sets (finding F11).
fn session_events() -> Vec<Event> {
    vec![
        Event::SessionStarted {
            // What the run could *do*, beside what it was offered — the two come apart entirely
            // behind a surface that publishes three verbs over a catalogue.
            available_operations: Some(vec!["file.read".into(), "shell".into()]),
            adapter: "claude".into(),
            adapter_class: "harness".into(),
            harness_version: Some("2.1.239".into()),
            session_id: Some("s-1".into()),
            model: Some("a-model".into()),
            permission_mode: Some("default".into()),
            credential_source: Some("none".into()),
            output_style: Some("default".into()),
            cwd: Some("/run/work".into()),
            offered_tools: Some(vec!["Read".into(), "Edit".into()]),
            slash_commands: Some(vec!["compact".into()]),
            skills: Some(vec![]),
            agents: Some(vec![]),
            plugins: Some(vec![PluginRef {
                name: Some("only-ours".into()),
                source: Some("local".into()),
                version: Some("0".into()),
            }]),
            mcp_servers: Some(vec![McpServerRef {
                name: Some("ours".into()),
                status: Some("connected".into()),
            }]),
            inputs_digest: Some(Digest::of(b"tree")),
            transcript: TranscriptRef {
                path: Some("run/transcript.jsonl".into()),
                digest: Some(Digest::of(b"bytes")),
                bytes: Some(5),
            },
            hermetic: HermeticAttestation {
                mode: HermeticMode::Strict,
                decisions: metaharness_protocol::DecisionMode::Observe,
                imposed: vec![ImposedControl {
                    row: metaharness_protocol::HermeticRow::H5,
                    how: "--strict-mcp-config".into(),
                }],
                unavailable: vec![],
                ambient_inputs: vec!["git status is in the system prompt".into()],
                installed_plugins: vec![metaharness_protocol::InstalledPlugin {
                    name: "claude-code".into(),
                    source: "/operator/integrations/claude-code".into(),
                    installed_at: "/scratch/run-1/plugins/claude-code".into(),
                    digest: Digest::of(b"plugin"),
                    loaded_by: "--plugin-dir in the argv".into(),
                }],
            },
        },
        Event::SessionEnded {
            is_error: Some(false),
            subtype: Some("success".into()),
            stop_reason: Some("end_turn".into()),
            terminal_reason: Some("completed".into()),
            api_error_status: None,
            num_turns: Some(4),
            duration_ms: Some(1000),
            duration_api_ms: Some(900),
            ttft_ms: Some(300),
            time_to_request_ms: Some(30),
            total_cost_usd: Some(0.12),
            permission_denials: Some(vec![PermissionDenial {
                tool_name: Some("Bash".into()),
                tool_use_id: Some("call-1".into()),
                tool_input: Some(json!({"command": "true"})),
            }]),
            subagents_spawned: Some(0),
            usage: Some(Usage {
                input_tokens: Some(10),
                output_tokens: Some(20),
                cache_read_input_tokens: Some(0),
                cache_creation_input_tokens: Some(0),
                service_tier: Some("standard".into()),
                thinking_tokens: Some(64),
                iterations: Some(3),
                speed: Some("standard".into()),
                // The aggregate carries no cost: the run's own figure is `total_cost_usd` above
                // and the priced slice is the per-model record below (amendment a9).
                cost_usd: None,
            }),
            model_usage: Some(BTreeMap::from([(
                "a-model".to_string(),
                Usage {
                    cost_usd: Some(0.12),
                    ..Usage::default()
                },
            )])),
            census: DecisionCensus {
                allowed: 3,
                denied: 1,
                replaced: 0,
                abstained: 2,
                by_seam: BTreeMap::from([("hook".to_string(), 4)]),
                by_decider: BTreeMap::from([("frame".to_string(), 4)]),
            },
        },
    ]
}

/// The boundaries and the content, which is where a step and a turn stay distinguishable.
fn boundary_and_content_events() -> Vec<Event> {
    vec![
        Event::StepEntered {
            step: step(),
            frame_digest: Some(frame().digest.clone()),
        },
        Event::StepLeft {
            step: step(),
            outcome: StepOutcome::NoVerdict {
                reason: "the harness died before a terminal record".into(),
            },
        },
        Event::TurnStarted {
            turn: 1,
            frame_digest: Some(frame().digest.clone()),
        },
        Event::TurnEnded {
            turn: 1,
            stop_reason: Some("end_turn".into()),
        },
        Event::Text {
            text: "the plan is written".into(),
            request_id: Some("req-1".into()),
        },
        Event::Thinking {
            text: "weighing two options".into(),
            request_id: Some("req-1".into()),
        },
        Event::ThinkingEstimate {
            estimate: 512,
            delta: Some(64),
        },
        Event::Injection {
            text: "the frame, rendered".into(),
            origin: Some("frame".into()),
        },
    ]
}

/// The three tool events, the accounting and the escape hatch.
fn tool_and_accounting_events() -> Vec<Event> {
    vec![
        Event::ToolRequested {
            call_id: "call-1".into(),
            name: "Bash".into(),
            input: json!({"command": "cargo test"}),
            // The neutral answer beside the vendor's name, which is the field a consumer reads:
            // `Bash` here, `run` on b10x, `tool_invoke` under an owned surface — one `shell`.
            operations: vec!["shell".into()],
            // Empty on a shell call, and deliberately: a command string is not a program, and
            // pulling an argv[0] out of one would be a claim about what ran.
            subjects: Vec::new(),
            decision_required: true,
            deadline_ms: Some(5_000),
            seam: Seam::Hook,
        },
        Event::ToolDecided {
            call_id: "call-1".into(),
            decision: Decision::Deny {
                reason: "this step admits no shell".into(),
            },
            decided_by: DecidedBy::Frame,
            seam: Seam::Hook,
            latency_ms: Some(4),
        },
        Event::ToolResult {
            call_id: "call-1".into(),
            is_error: Some(false),
            content: Some(json!("ok")),
            bytes: Some(2),
            tool_use_result: Some(json!({"commandName": "verify", "success": true})),
        },
        Event::Usage {
            request_id: Some("req-1".into()),
            model: Some("a-model".into()),
            usage: Usage::default(),
        },
        Event::RateLimit {
            info: RateLimitInfo {
                status: Some("allowed".into()),
                window: Some("five_hour".into()),
                resets_at: Some(1_700_000_000),
                utilization: Some(0.5),
                using_overage: Some(false),
            },
        },
        Event::CommandResult {
            id: "c-1".into(),
            outcome: CommandOutcome::Refused {
                refused: Refused::new(
                    RefusalCode::Shadowed,
                    "a bare --allowedTools entry would auto-approve this before the callback",
                ),
            },
        },
        Event::Warning {
            code: warning_code::VERSION_OFF_PIN.into(),
            message: "2.1.239 is outside the pin".into(),
        },
        Event::Opaque {
            vendor_type: Some("system".into()),
            vendor_subtype: Some("something_new".into()),
            digest: Digest::of(b"{}"),
            source_line: Some(12),
        },
        Event::AuthExpired {
            credential_source: Some("operator-login".into()),
            detail: Some("the vendor said the session could not be refreshed".into()),
            source_line: Some(90),
        },
    ]
}

/// One of every event, in the design's § 4.1 order.
fn every_event() -> Vec<Event> {
    let mut events = session_events();
    events.extend(boundary_and_content_events());
    events.extend(tool_and_accounting_events());
    events
}

fn every_command() -> Vec<Command> {
    vec![
        Command::ToolDecide {
            call_id: "call-1".into(),
            decision: Decision::Replace {
                input: json!({"command": "cargo test --workspace"}),
            },
        },
        Command::FrameSet {
            frame: Box::new(frame()),
        },
        Command::MessageInject {
            text: "the next step wants a red suite".into(),
        },
        Command::Steer {
            text: "stop and summarise".into(),
        },
        Command::PermissionSet {
            posture: "default".into(),
        },
        Command::Interrupt {
            reason: "the operator asked".into(),
        },
        Command::Halt {
            reason: "the budget is spent".into(),
        },
    ]
}

#[test]
fn every_event_round_trips_through_a_framed_line() {
    let mut stream = EventStream::new(RunId::new("r-1"));
    for event in every_event() {
        let expected = event.clone();
        let line = stream.stamp(Emission::at("2026-08-22T10:00:00Z", event));
        let written = serde_json::to_string(&line).expect("an event line serializes");
        let read = parse_event_line(&written).expect("an event line we wrote parses");
        assert_eq!(read.event, expected, "round trip differs for {written}");
        assert_eq!(read.format, EVENT_FORMAT);
        assert_eq!(read.at.as_deref(), Some("2026-08-22T10:00:00Z"));
    }
}

#[test]
fn every_command_round_trips_through_a_framed_line() {
    for (index, command) in every_command().into_iter().enumerate() {
        let expected = command.clone();
        let line = CommandLine::new(format!("c-{index}"), command);
        let written = serde_json::to_string(&line).expect("a command line serializes");
        let read = parse_command_line(&written).expect("a command line we wrote parses");
        assert_eq!(read.command, expected, "round trip differs for {written}");
        assert_eq!(read.format, COMMAND_FORMAT);
    }
}

/// The vocabulary is what the design decided, and its size is asserted so a variant added
/// without the document moving is caught here rather than in a reader somewhere else.
#[test]
fn the_vocabulary_is_the_one_the_design_lists() {
    let names: Vec<&str> = every_event().iter().map(Event::name).collect();
    assert_eq!(names, EVENT_NAMES, "every event, in the design's order");
    assert_eq!(
        EVENT_NAMES.len(),
        19,
        "18 from § 4.1 plus auth.expired (Q13)"
    );

    let commands: Vec<&str> = every_command().iter().map(Command::name).collect();
    assert_eq!(commands, COMMAND_NAMES);
    assert_eq!(COMMAND_NAMES.len(), 7);
}

/// `seq` is assigned in one place and nowhere else, so a verdict cites one thing.
#[test]
fn sequence_numbers_come_from_one_place_and_are_monotone() {
    let mut stream = EventStream::new(RunId::new("r-1"));
    let seqs: Vec<u64> = every_event()
        .into_iter()
        .map(|event| stream.stamp(Emission::untimed(event)).seq)
        .collect();
    assert_eq!(seqs, (1..=19).collect::<Vec<u64>>());
    assert_eq!(stream.emitted(), 19);
}

/// metaharness never measures: an event the vendor recorded no time for carries no time, and
/// the field is absent from the line rather than filled with now.
#[test]
fn an_untimed_event_carries_no_timestamp() {
    let mut stream = EventStream::new(RunId::new("r-1"));
    let line = stream.stamp(Emission::untimed(Event::Warning {
        code: warning_code::COVERAGE_GAP.into(),
        message: "Write is offered and uncovered".into(),
    }));
    let written = serde_json::to_string(&line).expect("serializes");
    assert!(!written.contains("\"at\""), "{written}");
}

/// An unknown **field** is ignored in silence — a reader that refused a line for carrying a new
/// key is a reader that stops working on the next patch release.
#[test]
fn an_unknown_field_on_a_known_event_is_ignored() {
    let line = r#"{"format":"metaharness.event/1","seq":1,"run":"r-1","event":"warning","code":"X","message":"m","a_field_from_the_future":42}"#;
    let read = parse_event_line(line).expect("the line parses");
    assert_eq!(
        read.event,
        Event::Warning {
            code: "X".into(),
            message: "m".into()
        }
    );
}

/// An unknown **name** is a named refusal — this wire is an authored schema, so a misspelling is
/// a mistake the author wants to be told about.
#[test]
fn an_unknown_event_name_is_refused_by_name() {
    let line = r#"{"format":"metaharness.event/1","seq":1,"run":"r-1","event":"tool.aproved"}"#;
    match parse_event_line(line) {
        Err(FramingError::UnknownName { name, vocabulary }) => {
            assert_eq!(name, "tool.aproved");
            assert_eq!(vocabulary, "event");
        }
        other => panic!("expected a named refusal, got {other:?}"),
    }
}

/// A tag this build does not know is refused rather than guessed.
#[test]
fn an_unknown_format_tag_is_refused() {
    let line = r#"{"format":"metaharness.event/2","seq":1,"run":"r-1","event":"warning","code":"X","message":"m"}"#;
    match parse_event_line(line) {
        Err(FramingError::UnknownFormat { tag, expected }) => {
            assert_eq!(tag.as_deref(), Some("metaharness.event/2"));
            assert_eq!(expected, EVENT_FORMAT);
        }
        other => panic!("expected a format refusal, got {other:?}"),
    }
}

/// A known name with a payload that does not fit is malformed, and says which name it was.
#[test]
fn a_known_name_with_a_bad_payload_is_malformed() {
    let line = r#"{"format":"metaharness.command/1","id":"c-1","command":"tool.decide"}"#;
    match parse_command_line(line) {
        Err(FramingError::Malformed { name, .. }) => assert_eq!(name, "tool.decide"),
        other => panic!("expected malformed, got {other:?}"),
    }
}

/// An absent payload field is `null` on the wire and never skipped: absence is the `unk` verdict
/// and it has to be visible to be read as one.
#[test]
fn an_absent_payload_field_is_null_and_not_missing() {
    let mut stream = EventStream::new(RunId::new("r-1"));
    let line = stream.stamp(Emission::untimed(Event::ToolResult {
        call_id: "call-1".into(),
        is_error: None,
        content: None,
        bytes: None,
        tool_use_result: None,
    }));
    let written = serde_json::to_string(&line).expect("serializes");
    assert!(written.contains(r#""is_error":null"#), "{written}");
    assert!(written.contains(r#""content":null"#), "{written}");
    // Amendment a9's field obeys the same rule as the fields it was added beside. A vendor that
    // writes no per-tool result record and a metaharness build that predates the amendment are
    // two different facts, and only an explicit `null` keeps them apart.
    assert!(written.contains(r#""tool_use_result":null"#), "{written}");
}

/// Amendment a9's four fields, from the seam's side: each is carried where the vendor reported it
/// and each survives the round trip under the key a reader looks for.
///
/// The four exist because `engineering-protocols`' gap register recorded four expectation kinds
/// that could not be decided about a driven run — `skill.completed`, `tokens.thinking`,
/// `iterations` and a `cost.total` scoped to one model — with the entry *"not this repository's to
/// close: it is four fields at the seam"*. This is the test that says the seam carries them.
#[test]
fn the_four_fields_amendment_a9_added_survive_the_round_trip() {
    let mut stream = EventStream::new(RunId::new("r-1"));

    let line = stream.stamp(Emission::untimed(Event::ToolResult {
        call_id: "call-1".into(),
        is_error: Some(false),
        content: Some(json!("Skill ran")),
        bytes: Some(9),
        tool_use_result: Some(json!({"commandName": "verify", "success": true})),
    }));
    let written = serde_json::to_string(&line).expect("serializes");
    let Event::ToolResult {
        tool_use_result, ..
    } = parse_event_line(&written).expect("parses").event
    else {
        panic!("expected tool.result");
    };
    let recorded = tool_use_result.expect("the vendor's sibling is carried");
    assert_eq!(recorded["commandName"], json!("verify"));
    assert_eq!(recorded["success"], json!(true));

    let line = stream.stamp(Emission::untimed(Event::Usage {
        request_id: None,
        model: Some("a-model".into()),
        usage: Usage {
            thinking_tokens: Some(64),
            iterations: Some(3),
            speed: Some("standard".into()),
            cost_usd: Some(0.5),
            ..Usage::default()
        },
    }));
    let written = serde_json::to_string(&line).expect("serializes");
    let Event::Usage { usage, .. } = parse_event_line(&written).expect("parses").event else {
        panic!("expected usage");
    };
    assert_eq!(usage.thinking_tokens, Some(64));
    assert_eq!(usage.iterations, Some(3));
    assert_eq!(usage.speed.as_deref(), Some("standard"));
    assert_eq!(usage.cost_usd, Some(0.5));
}

/// The projection is total: every event lands in exactly one family or on the control-plane
/// list, and the list is exhaustive so "none" is a decision.
#[test]
fn the_projection_is_total_and_the_control_plane_is_exhaustive() {
    let events = every_event();
    let report = project(&events);
    assert_eq!(report.total() as usize, events.len());

    let control_plane: Vec<&str> = events
        .iter()
        .filter(|event| ir_family(event).is_none())
        .map(Event::name)
        .collect();
    assert_eq!(control_plane, CONTROL_PLANE_EVENTS);
}

/// Structural projectability: every field a family is filled from is present on the line. This
/// is the claim "losslessly projectable" reduced to something a test can fail, without a
/// dependency on the repository that owns the IR (finding F1, Q9).
#[test]
fn every_event_carries_the_fields_its_ir_family_is_filled_from() {
    let report = project(&every_event());
    assert!(
        report.gaps.is_empty(),
        "unfillable IR fields: {:?}",
        report.gaps
    );
}

/// Two of the IR's fields are properties of a file, not of an event stream, and they are
/// reachable only because the adapter retained the bytes (design D6a, § 8.4 O8).
#[test]
fn the_transcript_the_projection_needs_is_carried_by_the_opening_record() {
    let report = project(&every_event());
    let transcript = report
        .transcript
        .expect("session.started carries a transcript");
    assert!(transcript.digest.is_some(), "transcript_digest is fillable");
    let source_lines: Vec<Option<u64>> = every_event()
        .iter()
        .filter_map(|event| match event {
            Event::Opaque { source_line, .. } => Some(*source_line),
            _ => None,
        })
        .collect();
    assert_eq!(source_lines, vec![Some(12)], "source_line is fillable");
}

/// Abstaining is a fourth answer and not a spelling of `allow`. `allow` grants — on this wire it
/// overrides a stricter rule in the vendor's own settings — so the value that means "metaharness
/// adjudicated nothing here" has to be distinct, or a run that narrowed nothing would read as a
/// run that permitted everything (amendment a3).
#[test]
fn abstaining_is_its_own_decision_and_round_trips() {
    let line = CommandLine::new(
        "c-1",
        Command::ToolDecide {
            call_id: "call-1".into(),
            decision: Decision::Abstain,
        },
    );
    let written = serde_json::to_string(&line).expect("serializes");
    assert!(written.contains(r#""decision":"abstain""#), "{written}");
    let read = parse_command_line(&written).expect("parses");
    assert_eq!(
        read.command,
        Command::ToolDecide {
            call_id: "call-1".into(),
            decision: Decision::Abstain
        }
    );
    assert!(Decision::Abstain.is_well_formed());
    assert_ne!(Decision::Abstain, Decision::Allow);
}

/// An empty deny reason reaches the model as a wall, and every vendor wire rejects it.
#[test]
fn a_deny_without_a_reason_is_not_well_formed() {
    assert!(
        !Decision::Deny {
            reason: "  ".into()
        }
        .is_well_formed()
    );
    assert!(
        Decision::Deny {
            reason: "this step admits no shell".into()
        }
        .is_well_formed()
    );
    assert!(Decision::Allow.is_well_formed());
}

/// Which commands a run will need is computed from the spec, so the refusals can be emitted at
/// run start rather than discovered at the call.
#[test]
fn a_run_declares_the_commands_it_will_need_before_it_starts() {
    let plain = RunSpec::new(Kind::Claude);
    assert_eq!(required_commands(&plain), ["halt", "interrupt"]);

    let mut asking = RunSpec::new(Kind::Claude);
    asking.decisions = DecisionMode::Ask;
    assert_eq!(
        required_commands(&asking),
        ["halt", "interrupt", "tool.decide"]
    );

    // A launch-time frame document needs the decision channel, not the mid-session `frame.set`
    // command: the text reaches the model at launch and per-call decisions make it true (F9).
    let mut framed = RunSpec::new(Kind::Claude);
    framed.frame = Some("frame.json".into());
    framed.tool_surface = ToolSurface::Owned;
    assert_eq!(
        required_commands(&framed),
        ["halt", "interrupt", "tool.decide"]
    );
}
