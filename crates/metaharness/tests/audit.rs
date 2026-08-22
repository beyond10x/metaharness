//! The `--audit` floor, the auditor contract and the four exit codes.
//!
//! The floor rows are asserted against **hand-built opening records** rather than through a run,
//! so a row's verdict cannot silently change because an adapter's attestation changed. The whole
//! path — spec, plan, transcript, events, audit — is asserted once at the bottom.

use metaharness::protocol::{
    CredentialSource, DecisionCensus, DecisionMode, Digest, Event, HermeticAttestation,
    HermeticMode, HermeticRow, ImposedControl, Kind, McpServerRef, PluginRef, RunSpec, Severity,
    TranscriptRef, UnavailableControl, Verdict,
};
use metaharness::{
    AuditReport, AuditorRun, FakeAuditor, FloorInputs, Input, ManualClock, Metaharness, Refusal,
    RunExit, ScriptStep, ScriptedLog, ScriptedRunner, ScriptedSeams, auditor_argv,
    count_verdict_rows, decision_census, exit_without_audit, hermetic_floor, run_auditor,
};

/// An opening record whose every observable field is present and correct.
fn good_record() -> Event {
    Event::SessionStarted {
        adapter: "claude".to_string(),
        adapter_class: "harness".to_string(),
        harness_version: Some("2.1.239".to_string()),
        session_id: Some("s-1".to_string()),
        model: Some("sonnet".to_string()),
        permission_mode: Some("default".to_string()),
        credential_source: Some("operator-login".to_string()),
        output_style: Some("default".to_string()),
        cwd: Some("/scratch/work".to_string()),
        offered_tools: Some(vec!["Bash".to_string()]),
        slash_commands: Some(Vec::new()),
        skills: Some(Vec::new()),
        agents: Some(Vec::new()),
        plugins: Some(Vec::new()),
        mcp_servers: Some(Vec::new()),
        inputs_digest: Some(Digest::of(b"tree")),
        transcript: TranscriptRef {
            path: Some("/scratch/transcript.jsonl".to_string()),
            digest: Some(Digest::of(b"bytes")),
            bytes: Some(12),
        },
        hermetic: attestation_imposing(&[
            HermeticRow::H2,
            HermeticRow::H3,
            HermeticRow::H6,
            HermeticRow::H8,
            HermeticRow::H11,
        ]),
    }
}

fn attestation_imposing(rows: &[HermeticRow]) -> HermeticAttestation {
    HermeticAttestation {
        mode: HermeticMode::Strict,
        imposed: rows
            .iter()
            .map(|row| ImposedControl {
                row: *row,
                how: format!("imposed {}", row.id()),
            })
            .collect(),
        unavailable: Vec::new(),
        ambient_inputs: vec!["git status is in the system prompt".to_string()],
    }
}

fn inputs<'a>(spec: &'a RunSpec, pins: &'a [String], plugins: &'a [String]) -> FloorInputs<'a> {
    FloorInputs {
        spec,
        pinned_versions: pins,
        planned_cwd: Some("/scratch/work"),
        declared_plugins: plugins,
    }
}

fn verdict_of(rows: &[metaharness::protocol::RowVerdict], which: HermeticRow) -> Verdict {
    rows.iter()
        .find(|row| row.row == which)
        .map(|row| row.verdict)
        .expect("every row is evaluated")
}

fn pins() -> Vec<String> {
    vec!["2.1.239".to_string()]
}

/// One field of an otherwise clean opening record, changed. Reads as the failure it induces.
fn with_record(mut record: Event, edit: impl FnOnce(&mut Event)) -> Event {
    edit(&mut record);
    record
}

// ---------------------------------------------------------------- the twelve rows

#[test]
fn the_floor_evaluates_all_twelve_rows_every_time() {
    let spec = RunSpec::new(Kind::Claude);
    let rows = hermetic_floor(&[good_record()], &inputs(&spec, &pins(), &[]));
    assert_eq!(rows.len(), 12);
    assert_eq!(rows.len(), HermeticRow::ALL.len());
}

#[test]
fn a_clean_opening_record_makes_every_row_ok() {
    let spec = RunSpec::new(Kind::Claude);
    let rows = hermetic_floor(&[good_record()], &inputs(&spec, &pins(), &[]));
    let not_ok: Vec<&str> = rows
        .iter()
        .filter(|row| row.verdict != Verdict::Ok)
        .map(|row| row.row.id())
        .collect();
    assert!(not_ok.is_empty(), "{not_ok:?}");
}

#[test]
fn a_run_with_no_opening_record_is_unk_everywhere_and_never_ok() {
    let spec = RunSpec::new(Kind::Claude);
    let rows = hermetic_floor(&[], &inputs(&spec, &pins(), &[]));
    assert!(rows.iter().all(|row| row.verdict == Verdict::Unk));
}

#[test]
fn a_missing_mcp_list_is_unk_and_never_zero() {
    let spec = RunSpec::new(Kind::Claude);
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted { mcp_servers, .. } = record {
            *mcp_servers = None;
        }
    });
    let rows = hermetic_floor(&[record], &inputs(&spec, &pins(), &[]));
    assert_eq!(verdict_of(&rows, HermeticRow::H5), Verdict::Unk);
    assert!(
        rows.iter()
            .find(|row| row.row == HermeticRow::H5)
            .expect("H5")
            .detail
            .contains("never zero")
    );
}

#[test]
fn an_mcp_server_the_launch_did_not_configure_is_a_gap_and_is_named() {
    let spec = RunSpec::new(Kind::Claude);
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted { mcp_servers, .. } = record {
            *mcp_servers = Some(vec![McpServerRef {
                name: Some("linear".to_string()),
                status: Some("failed".to_string()),
            }]);
        }
    });
    let rows = hermetic_floor(&[record], &inputs(&spec, &pins(), &[]));
    let h5 = rows.iter().find(|row| row.row == HermeticRow::H5).unwrap();
    assert_eq!(h5.verdict, Verdict::Gap);
    assert!(h5.detail.contains("linear"));
}

#[test]
fn a_missing_output_style_is_unk_and_a_foreign_one_is_a_gap() {
    let spec = RunSpec::new(Kind::Claude);

    let missing = with_record(good_record(), |record| {
        if let Event::SessionStarted { output_style, .. } = record {
            *output_style = None;
        }
    });
    assert_eq!(
        verdict_of(
            &hermetic_floor(&[missing], &inputs(&spec, &pins(), &[])),
            HermeticRow::H1b
        ),
        Verdict::Unk
    );

    let foreign = with_record(good_record(), |record| {
        if let Event::SessionStarted { output_style, .. } = record {
            *output_style = Some("Explanatory".to_string());
        }
    });
    assert_eq!(
        verdict_of(
            &hermetic_floor(&[foreign], &inputs(&spec, &pins(), &[])),
            HermeticRow::H1b
        ),
        Verdict::Gap
    );
}

#[test]
fn a_plugin_the_run_did_not_declare_is_a_gap() {
    let spec = RunSpec::new(Kind::Claude);
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted { plugins, .. } = record {
            *plugins = Some(vec![PluginRef {
                name: Some("someone-elses".to_string()),
                source: None,
                version: None,
            }]);
        }
    });
    assert_eq!(
        verdict_of(
            &hermetic_floor(&[record], &inputs(&spec, &pins(), &[])),
            HermeticRow::H1a
        ),
        Verdict::Gap
    );
}

#[test]
fn a_credential_source_the_run_did_not_declare_is_a_gap() {
    let spec = RunSpec::new(Kind::Claude); // defaults to operator-login
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted {
            credential_source, ..
        } = record
        {
            *credential_source = Some("ANTHROPIC_API_KEY".to_string());
        }
    });
    assert_eq!(
        verdict_of(
            &hermetic_floor(&[record], &inputs(&spec, &pins(), &[])),
            HermeticRow::H4
        ),
        Verdict::Gap
    );

    let declared = RunSpec {
        credentials: CredentialSource::ApiKey,
        ..RunSpec::new(Kind::Claude)
    };
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted {
            credential_source, ..
        } = record
        {
            *credential_source = Some("ANTHROPIC_API_KEY".to_string());
        }
    });
    assert_eq!(
        verdict_of(
            &hermetic_floor(&[record], &inputs(&declared, &pins(), &[])),
            HermeticRow::H4
        ),
        Verdict::Ok
    );
}

#[test]
fn a_vendor_off_its_pin_is_a_gap_and_the_report_names_both_versions() {
    let spec = RunSpec::new(Kind::Claude);
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted {
            harness_version, ..
        } = record
        {
            *harness_version = Some("2.2.0".to_string());
        }
    });
    let rows = hermetic_floor(&[record], &inputs(&spec, &pins(), &[]));
    let h9 = rows.iter().find(|row| row.row == HermeticRow::H9).unwrap();
    assert_eq!(h9.verdict, Verdict::Gap);
    assert!(h9.detail.contains("2.2.0") && h9.detail.contains("2.1.239"));
}

#[test]
fn a_working_directory_that_is_not_the_planned_one_is_a_gap() {
    let spec = RunSpec::new(Kind::Claude);
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted { cwd, .. } = record {
            *cwd = Some("/home/operator/project".to_string());
        }
    });
    assert_eq!(
        verdict_of(
            &hermetic_floor(&[record], &inputs(&spec, &pins(), &[])),
            HermeticRow::H7
        ),
        Verdict::Gap
    );
}

#[test]
fn a_launch_row_the_attestation_is_silent_about_is_unk_and_not_a_pass() {
    let spec = RunSpec::new(Kind::Claude);
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted { hermetic, .. } = record {
            *hermetic = attestation_imposing(&[HermeticRow::H2, HermeticRow::H6]);
        }
    });
    let rows = hermetic_floor(&[record], &inputs(&spec, &pins(), &[]));
    assert_eq!(verdict_of(&rows, HermeticRow::H3), Verdict::Unk);
    assert_eq!(verdict_of(&rows, HermeticRow::H8), Verdict::Unk);
    assert_eq!(verdict_of(&rows, HermeticRow::H11), Verdict::Unk);
}

#[test]
fn a_control_the_attestation_says_it_could_not_impose_is_a_gap_with_the_reason() {
    let spec = RunSpec::new(Kind::Claude);
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted { hermetic, .. } = record {
            hermetic.unavailable.push(UnavailableControl {
                row: HermeticRow::H11,
                why: "the scratch root has a CLAUDE.md ancestor".to_string(),
            });
            hermetic
                .imposed
                .retain(|control| control.row != HermeticRow::H11);
        }
    });
    let rows = hermetic_floor(&[record], &inputs(&spec, &pins(), &[]));
    let h11 = rows.iter().find(|row| row.row == HermeticRow::H11).unwrap();
    assert_eq!(h11.verdict, Verdict::Gap);
    assert!(h11.detail.contains("CLAUDE.md"));
}

// ---------------------------------------------------------------- severity (finding F3)

#[test]
fn the_two_advisory_rows_are_evaluated_and_do_not_move_the_exit_code() {
    let spec = RunSpec::new(Kind::Claude);
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted { hermetic, .. } = record {
            // H2 and H6 are now silent: unobservable as a property of the mechanism.
            hermetic
                .imposed
                .retain(|control| !matches!(control.row, HermeticRow::H2 | HermeticRow::H6));
        }
    });
    let rows = hermetic_floor(&[record], &inputs(&spec, &pins(), &[]));
    assert_eq!(verdict_of(&rows, HermeticRow::H2), Verdict::Unk);
    assert_eq!(verdict_of(&rows, HermeticRow::H6), Verdict::Unk);
    for row in &rows {
        if matches!(row.row, HermeticRow::H2 | HermeticRow::H6) {
            assert_eq!(row.severity, Severity::Advisory);
            assert!(!row.gates());
        }
    }
    let report = AuditReport {
        rows,
        census: DecisionCensus::default(),
        auditor: None,
        saw_terminal_record: true,
    };
    assert_eq!(
        report.exit(),
        RunExit::Ok,
        "if any unk failed a strict run then every strict run would fail forever (F3)"
    );
}

// ---------------------------------------------------------------- exit codes (§ 9.4)

fn report_with(rows: Vec<metaharness::protocol::RowVerdict>) -> AuditReport {
    AuditReport {
        rows,
        census: DecisionCensus::default(),
        auditor: None,
        saw_terminal_record: true,
    }
}

fn floor_for(record: Event) -> Vec<metaharness::protocol::RowVerdict> {
    let spec = RunSpec::new(Kind::Claude);
    hermetic_floor(&[record], &inputs(&spec, &pins(), &[]))
}

#[test]
fn a_clean_audited_run_exits_zero() {
    assert_eq!(report_with(floor_for(good_record())).exit(), RunExit::Ok);
    assert_eq!(RunExit::Ok.code(), 0);
}

#[test]
fn a_gating_gap_exits_one() {
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted {
            harness_version, ..
        } = record
        {
            *harness_version = Some("9.9.9".to_string());
        }
    });
    let report = report_with(floor_for(record));
    assert_eq!(report.exit(), RunExit::Gap);
    assert_eq!(RunExit::Gap.code(), 1);
}

#[test]
fn a_gating_unknown_exits_three_because_nobody_found_out() {
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted {
            harness_version, ..
        } = record
        {
            *harness_version = None;
        }
    });
    let report = report_with(floor_for(record));
    assert_eq!(report.exit(), RunExit::NoVerdict);
    assert_eq!(RunExit::NoVerdict.code(), 3);
}

#[test]
fn a_gap_outranks_an_unknown_because_a_definite_failure_is_a_fact() {
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted {
            harness_version,
            output_style,
            ..
        } = record
        {
            *harness_version = None; // unk
            *output_style = Some("Explanatory".to_string()); // gap
        }
    });
    assert_eq!(report_with(floor_for(record)).exit(), RunExit::Gap);
}

#[test]
fn a_harness_that_died_without_a_record_exits_three_even_with_every_row_ok() {
    let mut report = report_with(floor_for(good_record()));
    report.saw_terminal_record = false;
    assert_eq!(report.exit(), RunExit::NoVerdict);
}

#[test]
fn an_auditor_that_exited_one_exits_one_and_one_that_exited_three_exits_three() {
    for (auditor_code, expected) in [(1, RunExit::Gap), (3, RunExit::NoVerdict), (0, RunExit::Ok)] {
        let mut report = report_with(floor_for(good_record()));
        report.auditor = Some(metaharness::AuditorVerdict {
            argv: vec!["protocol".to_string()],
            exit_code: Some(auditor_code),
            verdict_rows: 7,
        });
        assert_eq!(report.exit(), expected, "auditor exit {auditor_code}");
    }
}

#[test]
fn a_run_without_an_audit_never_exits_one() {
    assert_eq!(exit_without_audit(true), RunExit::Ok);
    assert_eq!(exit_without_audit(false), RunExit::NoVerdict);
    for saw_record in [true, false] {
        assert_ne!(exit_without_audit(saw_record).code(), 1);
    }
}

#[test]
fn metaharness_could_not_do_its_job_is_always_two() {
    assert_eq!(RunExit::Broken.code(), 2);
}

// ---------------------------------------------------------------- the census is always printed

#[test]
fn the_report_always_prints_the_census_even_when_it_is_zero() {
    let rendered = report_with(floor_for(good_record())).render();
    assert!(rendered.contains("decision census: allowed=0 denied=0 replaced=0"));
    assert!(
        rendered.contains("cannot distinguish enforcement holding from nothing being attempted")
    );
}

#[test]
fn the_report_says_the_attestation_is_metaharnesss_own_claim() {
    let rendered = report_with(floor_for(good_record())).render();
    assert!(rendered.contains("not independent evidence"));
}

#[test]
fn the_census_is_read_from_the_terminal_record_when_there_is_one() {
    let census = DecisionCensus {
        allowed: 3,
        denied: 1,
        ..DecisionCensus::default()
    };
    let ended = Event::SessionEnded {
        is_error: Some(false),
        subtype: Some("success".to_string()),
        stop_reason: None,
        terminal_reason: None,
        api_error_status: None,
        num_turns: Some(2),
        duration_ms: None,
        duration_api_ms: None,
        ttft_ms: None,
        time_to_request_ms: None,
        total_cost_usd: None,
        permission_denials: Some(Vec::new()),
        subagents_spawned: None,
        usage: None,
        model_usage: None,
        census: census.clone(),
    };
    assert_eq!(decision_census(&[ended]), census);
}

// ---------------------------------------------------------------- the auditor contract

#[test]
fn the_auditor_prefix_is_argv_and_a_two_word_subcommand_is_not_a_special_case() {
    let argv = auditor_argv(
        "protocol trace check",
        std::path::Path::new("expectations.yaml"),
        std::path::Path::new("/scratch/transcript.jsonl"),
        &[
            "--advisory".to_string(),
            "billed-to-the-session".to_string(),
        ],
    );
    assert_eq!(
        argv,
        vec![
            "protocol",
            "trace",
            "check",
            "--spec",
            "expectations.yaml",
            "--transcript",
            "/scratch/transcript.jsonl",
            "--advisory",
            "billed-to-the-session",
        ]
    );
}

#[test]
fn a_single_word_auditor_is_a_degenerate_prefix() {
    let argv = auditor_argv(
        "my-checker",
        std::path::Path::new("s.yaml"),
        std::path::Path::new("t.jsonl"),
        &[],
    );
    assert_eq!(argv[0], "my-checker");
    assert_eq!(argv.len(), 5);
}

#[test]
fn a_spec_with_no_auditor_is_a_refusal_and_not_a_skip() {
    let mut auditor = FakeAuditor::default();
    let refused = run_auditor(
        Some(std::path::Path::new("s.yaml")),
        None,
        &[],
        std::path::Path::new("t.jsonl"),
        &mut auditor,
    )
    .expect_err("refused");
    assert_eq!(refused, Refusal::SpecWithoutAuditor);
    assert!(auditor.calls().is_empty());
}

#[test]
fn an_auditor_with_no_spec_is_a_refusal_because_there_is_nothing_to_check() {
    let mut auditor = FakeAuditor::default();
    let refused = run_auditor(
        None,
        Some("protocol trace check"),
        &[],
        std::path::Path::new("t.jsonl"),
        &mut auditor,
    )
    .expect_err("refused");
    assert_eq!(refused, Refusal::AuditorWithoutSpec);
}

#[test]
fn neither_a_spec_nor_an_auditor_runs_the_floor_and_nothing_else() {
    let mut auditor = FakeAuditor::default();
    let verdict = run_auditor(
        None,
        None,
        &[],
        std::path::Path::new("t.jsonl"),
        &mut auditor,
    )
    .expect("no ceiling asked for");
    assert!(verdict.is_none());
}

#[test]
fn an_unreadable_specification_is_a_setup_failure() {
    let mut auditor = FakeAuditor::default();
    let refused = run_auditor(
        Some(std::path::Path::new("/no/such/expectations.yaml")),
        Some("protocol"),
        &[],
        std::path::Path::new("t.jsonl"),
        &mut auditor,
    )
    .expect_err("refused");
    assert!(matches!(refused, Refusal::SpecUnreadable { .. }));
}

#[test]
fn an_auditor_that_is_not_there_is_a_setup_failure_naming_the_argv() {
    let spec = tempfile::NamedTempFile::new().expect("a spec file");
    let mut auditor = FakeAuditor::missing();
    let refused = run_auditor(
        Some(spec.path()),
        Some("no-such-checker"),
        &[],
        std::path::Path::new("t.jsonl"),
        &mut auditor,
    )
    .expect_err("refused");
    match refused {
        Refusal::AuditorNotInvokable { argv, .. } => assert_eq!(argv[0], "no-such-checker"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_audit_with_no_verdict_rows_is_a_setup_failure_and_never_a_verdict() {
    let spec = tempfile::NamedTempFile::new().expect("a spec file");
    for (stdout, exit_code) in [("", Some(0)), ("   \n\n", Some(1))] {
        let mut auditor = FakeAuditor::answering(AuditorRun {
            exit_code,
            stdout: stdout.to_string(),
            stderr: String::new(),
        });
        let refused = run_auditor(
            Some(spec.path()),
            Some("protocol trace check"),
            &[],
            std::path::Path::new("t.jsonl"),
            &mut auditor,
        )
        .expect_err("a table with nothing in it is a setup failure");
        assert!(matches!(refused, Refusal::NoVerdictRows { .. }));
    }
}

#[test]
fn verdict_rows_are_counted_as_non_blank_lines_of_stdout() {
    assert_eq!(count_verdict_rows(""), 0);
    assert_eq!(count_verdict_rows("\n   \n"), 0);
    assert_eq!(count_verdict_rows("H1a ok\nH2 gap\n"), 2);
}

#[test]
fn an_auditor_that_produced_rows_is_recorded_with_its_argv_and_its_exit_code() {
    let spec = tempfile::NamedTempFile::new().expect("a spec file");
    let mut auditor = FakeAuditor::answering(AuditorRun {
        exit_code: Some(1),
        stdout: "one ok\ntwo gap\n".to_string(),
        stderr: String::new(),
    });
    let verdict = run_auditor(
        Some(spec.path()),
        Some("protocol trace check"),
        &["--advisory".to_string()],
        std::path::Path::new("/scratch/transcript.jsonl"),
        &mut auditor,
    )
    .expect("the auditor ran")
    .expect("it produced a verdict");
    assert_eq!(verdict.exit_code, Some(1));
    assert_eq!(verdict.verdict_rows, 2);
    assert_eq!(verdict.argv.last().map(String::as_str), Some("--advisory"));
    assert_eq!(auditor.calls().len(), 1);
}

// ---------------------------------------------------------------- the whole path

#[test]
fn a_run_goes_from_spec_through_plan_and_transcript_to_a_judged_verdict() {
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::new(
        vec![
            ScriptStep::line(
                r#"{"emit":"session.started","harness_version":"2.1.239","output_style":"default","plugins":[],"mcp_servers":[],"credential_source":"operator-login","inputs_digest":"tree"}"#,
            ),
            ScriptStep::line(
                r#"{"emit":"tool.requested","call_id":"t1","name":"Bash","input":{"command":"ls"}}"#,
            ),
            ScriptStep::awaiting("t1"),
            ScriptStep::line(r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#),
        ],
        log,
    );
    let mut seams = ScriptedSeams;
    let mut run = Metaharness::new(Kind::Claude)
        .with_hermetic(HermeticMode::Strict)
        .with_decisions(DecisionMode::Ask)
        .with_audit(true)
        .start_with_clock(
            Input::Prompt("do it".to_string()),
            &mut runner,
            &mut seams,
            Box::new(ManualClock::new()),
        )
        .expect("starts");

    while let Some(line) = run.next_event().expect("pumps") {
        if let Event::ToolRequested {
            call_id,
            decision_required: true,
            ..
        } = &line.event
        {
            run.send(metaharness::protocol::Command::ToolDecide {
                call_id: call_id.clone(),
                decision: metaharness::protocol::Decision::Deny {
                    reason: "this step admits no shell".to_string(),
                },
            })
            .expect("sends");
        }
    }

    assert!(run.wants_audit());
    let mut auditor = FakeAuditor::default();
    let report = run.audit(&mut auditor).expect("the floor always runs");

    assert_eq!(report.rows.len(), 12);
    assert_eq!(report.census.denied, 1);
    assert!(report.render().contains("denied=1"));
    assert!(
        run.transcript().path.is_some(),
        "O8: the bytes are retained"
    );

    // H7 and H10 are unk here because the scratch cwd is not in the fixture record and no input
    // tree was copied — which is exactly the point: absence of evidence is not hermeticity.
    assert_eq!(run.exit(Some(&report)), RunExit::NoVerdict);
}

#[test]
fn a_strict_run_asks_for_the_floor_even_without_the_audit_flag() {
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::new(
        vec![ScriptStep::line(
            r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#,
        )],
        log,
    );
    let mut seams = ScriptedSeams;
    let mut run = Metaharness::new(Kind::Claude)
        .with_hermetic(HermeticMode::Strict)
        .start_with_clock(
            Input::Prompt("do the thing".to_string()),
            &mut runner,
            &mut seams,
            Box::new(ManualClock::new()),
        )
        .expect("starts");
    run.drain().expect("drains");
    assert!(run.wants_audit());

    let mut plain = Metaharness::new(Kind::Claude);
    plain = plain.with_hermetic(HermeticMode::On);
    assert!(!plain.spec().audit);
}
