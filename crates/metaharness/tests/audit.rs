//! The `--audit` floor, the auditor contract and the four exit codes.
//!
//! The floor rows are asserted against **hand-built opening records** rather than through a run,
//! so a row's verdict cannot silently change because an adapter's attestation changed. The whole
//! path — spec, plan, transcript, events, audit — is asserted once at the bottom.

use metaharness::protocol::{
    CredentialSource, DecisionCensus, DecisionMode, Digest, Event, HermeticAttestation,
    HermeticMode, HermeticRow, ImposedControl, Kind, McpServerRef, PluginRef, RunSpec, Severity,
    TranscriptRef, UnavailableControl, Verdict, WithheldTool,
};
use metaharness::{
    AuditReport, AuditorRun, BuiltinPlugin, ClaudeSeams, FakeAuditor, FloorInputs, Input,
    ManualClock, MemoryCanary, Metaharness, Refusal, RunExit, ScriptStep, ScriptedLog,
    ScriptedRunner, ScriptedSeams, auditor_argv, builtin_plugins, count_verdict_rows,
    decision_census, exit_without_audit, hermetic_floor, run_auditor, withheld_tools,
};

/// An opening record whose every observable field is present and correct.
fn good_record() -> Event {
    Event::SessionStarted {
        available_operations: None,
        // The vendor states no such thing, so this record does not either. `None` is *it did not
        // say*, which is what every row of the floor reads it as.
        withheld: None,
        context_files: Vec::new(),
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
        decisions: metaharness::protocol::DecisionMode::Frame,
        imposed: rows
            .iter()
            .map(|row| ImposedControl {
                row: *row,
                how: format!("imposed {}", row.id()),
            })
            .collect(),
        unavailable: Vec::new(),
        declared: Vec::new(),
        ambient_inputs: vec!["git status is in the system prompt".to_string()],
        installed_plugins: Vec::new(),
    }
}

fn inputs<'a>(spec: &'a RunSpec, pins: &'a [String], plugins: &'a [String]) -> FloorInputs<'a> {
    FloorInputs {
        spec,
        pinned_versions: pins,
        planned_cwd: Some("/scratch/work"),
        declared_plugins: plugins,
        builtin_plugins: &[],
        endpoint_key_posture: None,
        secret_key_source: None,
        planned_mcp_servers: &[],
        planned_output_style: None,
        record_silences: &[],
        memory_canaries: None,
    }
}

/// The same, for a launch that configured its own tool server.
fn inputs_serving<'a>(
    spec: &'a RunSpec,
    pins: &'a [String],
    servers: &'a [String],
) -> FloorInputs<'a> {
    FloorInputs {
        planned_mcp_servers: servers,
        ..inputs(spec, pins, &[])
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

// ------------------------------------------- H11 held to the memory canary (amendment a36)

/// A canary row for `version`: `planted` canaries, the control saw `control_sent`, and
/// `reached` got through the launch.
fn canary(version: &str, control_sent: usize, reached: usize) -> MemoryCanary {
    MemoryCanary {
        version: version.to_string(),
        planted: 12,
        control_sent,
        reached_through_launch: reached,
        not_covered: NOT_COVERED.to_string(),
    }
}

/// What the fixture canary says it does not cover.
const NOT_COVERED: &str = "managed memory is neither walked nor probed";

/// H11 under a kind whose launch control is held to a memory canary, over `good_record()`
/// (harness version 2.1.239, H11 imposed), with the version the record names replaced.
fn h11_with_canaries(
    version: Option<&str>,
    canaries: &[MemoryCanary],
) -> metaharness::protocol::RowVerdict {
    let spec = RunSpec::new(Kind::Claude);
    let pins = pins();
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted {
            harness_version, ..
        } = record
        {
            *harness_version = version.map(ToString::to_string);
        }
    });
    let rows = hermetic_floor(
        &[record],
        &FloorInputs {
            memory_canaries: Some(canaries),
            ..inputs(&spec, &pins, &[])
        },
    );
    rows.into_iter()
        .find(|row| row.row == HermeticRow::H11)
        .expect("every row is evaluated")
}

/// The claim holds only where the binary that ran has a canary that kept every shape out, and
/// the row says which canary it rests on.
#[test]
fn h11_is_ok_on_an_imposed_control_whose_version_kept_every_canary_out() {
    let row = h11_with_canaries(Some("2.1.239"), &[canary("2.1.239", 12, 0)]);
    assert_eq!(row.verdict, Verdict::Ok, "{}", row.detail);
    assert!(row.detail.contains("imposed H11"), "{}", row.detail);
    assert!(
        row.detail.contains(
            "memory canary recorded for 2.1.239: 12 planted, the control sent 12, 0 reached"
        ),
        "{}",
        row.detail
    );
    // a36 § 6: the row never reads wider than the measurement.
    assert!(
        row.detail.ends_with(&format!(
            "0 reached a request through the launch; {NOT_COVERED}"
        )),
        "{}",
        row.detail
    );
}

/// A canary that states nothing it leaves out adds nothing to the detail.
#[test]
fn h11_ok_adds_no_caveat_for_a_canary_that_states_none() {
    let silent = MemoryCanary {
        not_covered: String::new(),
        ..canary("2.1.239", 12, 0)
    };
    let row = h11_with_canaries(Some("2.1.239"), &[silent]);
    assert_eq!(row.verdict, Verdict::Ok, "{}", row.detail);
    assert!(
        row.detail
            .ends_with("0 reached a request through the launch"),
        "{}",
        row.detail
    );
}

/// A version with no recorded canary is a `gap`: nobody measured whether this binary honours
/// the control.
#[test]
fn h11_is_gap_on_a_version_without_a_memory_canary() {
    let row = h11_with_canaries(Some("2.1.240"), &[canary("2.1.239", 12, 0)]);
    assert_eq!(row.verdict, Verdict::Gap, "{}", row.detail);
    assert!(row.detail.contains("no memory canary"), "{}", row.detail);
    assert!(row.detail.contains("2.1.240"), "{}", row.detail);
}

/// A canary that reached a request through the launch turns H11 to `gap` (decision C's guard).
#[test]
fn h11_is_gap_when_the_recorded_canary_reached_the_model() {
    let row = h11_with_canaries(Some("2.1.239"), &[canary("2.1.239", 12, 1)]);
    assert_eq!(row.verdict, Verdict::Gap, "{}", row.detail);
    assert!(row.detail.contains("1 reached"), "{}", row.detail);
}

/// A control that did not see every canary shows nothing about the shapes it missed.
#[test]
fn h11_is_gap_when_the_canarys_control_missed_a_shape() {
    let row = h11_with_canaries(Some("2.1.239"), &[canary("2.1.239", 11, 0)]);
    assert_eq!(row.verdict, Verdict::Gap, "{}", row.detail);
    assert!(row.detail.contains("the control sent 11"), "{}", row.detail);
}

/// A record that names no version names no binary to look a canary up for.
#[test]
fn h11_is_unk_when_the_record_names_no_version_to_read_a_canary_for() {
    let row = h11_with_canaries(None, &[canary("2.1.239", 12, 0)]);
    assert_eq!(row.verdict, Verdict::Unk, "{}", row.detail);
}

/// The canary never upgrades a row the attestation did not claim: an unavailable H11 (a6) stays
/// a `gap`.
#[test]
fn a_memory_canary_never_turns_an_unavailable_h11_ok() {
    let spec = RunSpec::new(Kind::Claude);
    let pins = pins();
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted { hermetic, .. } = record {
            hermetic
                .imposed
                .retain(|control| control.row != HermeticRow::H11);
            hermetic.unavailable.push(UnavailableControl {
                row: HermeticRow::H11,
                why: "the operator's tree".to_string(),
            });
        }
    });
    let canaries = [canary("2.1.239", 12, 0)];
    let rows = hermetic_floor(
        &[record],
        &FloorInputs {
            memory_canaries: Some(&canaries),
            ..inputs(&spec, &pins, &[])
        },
    );
    assert_eq!(verdict_of(&rows, HermeticRow::H11), Verdict::Gap);
}

#[test]
fn a_direct_provider_with_no_plugin_or_style_surface_states_non_applicability() {
    let spec = RunSpec {
        credentials: CredentialSource::ApiKey,
        ..RunSpec::new(Kind::B10x)
    };
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted {
            adapter,
            adapter_class,
            credential_source,
            output_style,
            plugins,
            ..
        } = record
        {
            *adapter = "b10x".to_owned();
            *adapter_class = "direct_provider".to_owned();
            *credential_source = Some("api-key:environment".to_owned());
            *output_style = None;
            *plugins = None;
        }
    });
    let rows = hermetic_floor(&[record], &inputs(&spec, &pins(), &[]));
    assert_eq!(verdict_of(&rows, HermeticRow::H1a), Verdict::Ok);
    assert_eq!(verdict_of(&rows, HermeticRow::H1b), Verdict::Ok);
    assert_eq!(verdict_of(&rows, HermeticRow::H4), Verdict::Ok);
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

/// H5 asks whether the surface is the one the launch gave — not whether it is empty.
///
/// The failure this pins: `--tool-surface owned` configures metaharness's own tool server, and a
/// row that read every listed server as foreign reported the run's own tools as a hermetic breach.
/// An owned-surface run on 2026-08-23 failed H5 for exactly that, having done nothing wrong.
#[test]
fn the_launchs_own_mcp_server_is_the_expected_surface_and_not_a_breach() {
    let spec = RunSpec::new(Kind::Claude);
    let serving = ["metaharness".to_string()];
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted { mcp_servers, .. } = record {
            *mcp_servers = Some(vec![McpServerRef {
                name: Some("metaharness".to_string()),
                status: Some("connected".to_string()),
            }]);
        }
    });

    let rows = hermetic_floor(
        std::slice::from_ref(&record),
        &inputs_serving(&spec, &pins(), &serving),
    );
    let h5 = rows.iter().find(|row| row.row == HermeticRow::H5).unwrap();
    assert_eq!(h5.verdict, Verdict::Ok, "{}", h5.detail);
    assert!(h5.detail.contains("metaharness"), "{}", h5.detail);

    // The same record under a launch that configured nothing is still the gap it always was.
    let rows = hermetic_floor(&[record], &inputs(&spec, &pins(), &[]));
    assert_eq!(
        verdict_of(&rows, HermeticRow::H5),
        Verdict::Gap,
        "a server nobody configured is foreign whatever it is called"
    );
}

/// A server the launch gave that never appeared is a *different* failure from an extra one: the
/// model did not have the tools the run believes it published.
#[test]
fn a_configured_mcp_server_absent_from_the_record_is_a_gap_that_says_it_is_absent() {
    let spec = RunSpec::new(Kind::Claude);
    let serving = ["metaharness".to_string()];
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted { mcp_servers, .. } = record {
            *mcp_servers = Some(Vec::new());
        }
    });

    let rows = hermetic_floor(&[record], &inputs_serving(&spec, &pins(), &serving));
    let h5 = rows.iter().find(|row| row.row == HermeticRow::H5).unwrap();
    assert_eq!(h5.verdict, Verdict::Gap);
    assert!(h5.detail.contains("absent"), "{}", h5.detail);
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

// ---------------------------------------------------------------- Codex's record rows (a34)

/// A Codex opening record as the rollout reader completes it at 0.153.4: no plugin list, no
/// credential source and no MCP server list, and `output_style` holding the personality the first
/// `turn_context` recorded.
fn codex_record(output_style: Option<&str>) -> Event {
    with_record(good_record(), |record| {
        if let Event::SessionStarted {
            adapter,
            harness_version,
            credential_source,
            output_style: style,
            plugins,
            mcp_servers,
            ..
        } = record
        {
            *adapter = "codex".to_string();
            *harness_version = Some(metaharness_codex::PINNED_VERSIONS[0].to_string());
            *credential_source = None;
            *style = output_style.map(ToString::to_string);
            *plugins = None;
            *mcp_servers = None;
        }
    })
}

fn codex_pins() -> Vec<String> {
    metaharness_codex::PINNED_VERSIONS
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// What `start_codex` hands the floor: the personality the launch wrote, and why the record is
/// silent on H1a, H4 and H5.
fn codex_inputs<'a>(spec: &'a RunSpec, pins: &'a [String]) -> FloorInputs<'a> {
    FloorInputs {
        planned_output_style: Some(metaharness_codex::PERSONALITY),
        record_silences: &metaharness_codex::RECORD_SILENCES,
        ..inputs(spec, pins, &[])
    }
}

fn row_of(
    rows: &[metaharness::protocol::RowVerdict],
    which: HermeticRow,
) -> metaharness::protocol::RowVerdict {
    rows.iter()
        .find(|row| row.row == which)
        .cloned()
        .expect("every row is evaluated")
}

/// Amendment a34, one shape per verdict: H1b reads `ok` when the record names the personality the
/// launch wrote, `gap` when it names another, and `unk` when it names none — and the launch's
/// value is never read as the record's (invariant 3).
#[test]
fn codex_h1b_compares_the_recorded_personality_with_the_one_the_launch_wrote() {
    let spec = RunSpec::new(Kind::Codex);
    let pins = codex_pins();
    let floor = |style: Option<&str>| {
        row_of(
            &hermetic_floor(&[codex_record(style)], &codex_inputs(&spec, &pins)),
            HermeticRow::H1b,
        )
    };

    let recorded = floor(Some(metaharness_codex::PERSONALITY));
    assert_eq!(recorded.verdict, Verdict::Ok, "{}", recorded.detail);
    assert!(
        recorded.detail.contains(metaharness_codex::PERSONALITY),
        "{}",
        recorded.detail
    );

    let foreign = floor(Some("friendly"));
    assert_eq!(foreign.verdict, Verdict::Gap, "{}", foreign.detail);
    assert!(
        foreign.detail.contains("friendly")
            && foreign.detail.contains(metaharness_codex::PERSONALITY),
        "the gap names both values: {}",
        foreign.detail
    );

    // `default` is the word another kind's record uses for its own default. Under a launch that
    // wrote a value it is just another value, and never the one written.
    let default = floor(Some("default"));
    assert_eq!(default.verdict, Verdict::Gap, "{}", default.detail);

    let silent = floor(None);
    assert_eq!(silent.verdict, Verdict::Unk, "{}", silent.detail);
    assert!(
        silent.detail.contains(metaharness_codex::PERSONALITY),
        "the unk says what the launch wrote and the record did not confirm: {}",
        silent.detail
    );
}

/// A config layer the launch does not control can change what the model is told while the record
/// still names the personality the launch wrote (amendment a34: a managed
/// `features.personality = false` still recorded `"pragmatic"`). When the launch attests H1b
/// unavailable for such a layer, a matching record reads `unk` with that reason, never `ok`; a
/// record naming another value is still a `gap`.
#[test]
fn codex_h1b_is_unk_when_the_launch_attests_a_layer_it_does_not_control() {
    let spec = RunSpec::new(Kind::Codex);
    let pins = codex_pins();
    let why =
        "the binary also reads /etc/codex/managed_config.toml, which this launch does not control";
    let layered = |style: &str| {
        with_record(codex_record(Some(style)), |record| {
            if let Event::SessionStarted { hermetic, .. } = record {
                hermetic
                    .imposed
                    .retain(|control| control.row != HermeticRow::H1b);
                hermetic.unavailable.push(UnavailableControl {
                    row: HermeticRow::H1b,
                    why: why.to_string(),
                });
            }
        })
    };
    let matching = row_of(
        &hermetic_floor(
            &[layered(metaharness_codex::PERSONALITY)],
            &codex_inputs(&spec, &pins),
        ),
        HermeticRow::H1b,
    );
    assert_eq!(matching.verdict, Verdict::Unk, "{}", matching.detail);
    assert!(matching.detail.contains(why), "{}", matching.detail);
    let foreign = row_of(
        &hermetic_floor(&[layered("friendly")], &codex_inputs(&spec, &pins)),
        HermeticRow::H1b,
    );
    assert_eq!(foreign.verdict, Verdict::Gap, "{}", foreign.detail);
}

/// A kind whose launch writes no output style keeps the reading it had: the record must name the
/// default, whatever the value (amendment a34 changes nothing for Claude).
#[test]
fn a_launch_that_wrote_no_output_style_still_needs_the_default() {
    let spec = RunSpec::new(Kind::Claude);
    let pragmatic = with_record(good_record(), |record| {
        if let Event::SessionStarted { output_style, .. } = record {
            *output_style = Some("pragmatic".to_string());
        }
    });
    assert_eq!(
        verdict_of(
            &hermetic_floor(&[pragmatic], &inputs(&spec, &pins(), &[])),
            HermeticRow::H1b
        ),
        Verdict::Gap
    );
}

/// Amendment a34 (decision D1 = B): H1a, H4 and H5 stay `unk` on Codex, and each row says why — the
/// pinned binary's rollout carries no such field, and deciding the row from the launch is open
/// under invariant 3. The adapter's sentence is appended to the floor's own, never substituted.
#[test]
fn codex_h1a_h4_and_h5_stay_unk_and_each_says_why() {
    let spec = RunSpec::new(Kind::Codex);
    let pins = codex_pins();
    let rows = hermetic_floor(
        &[codex_record(Some(metaharness_codex::PERSONALITY))],
        &codex_inputs(&spec, &pins),
    );
    for (which, base) in [
        (
            HermeticRow::H1a,
            "the opening record carries no plugin list",
        ),
        (
            HermeticRow::H4,
            "the opening record does not say where the credential came from",
        ),
        (
            HermeticRow::H5,
            "the opening record carries no MCP server list",
        ),
    ] {
        let found = row_of(&rows, which);
        assert_eq!(
            found.verdict,
            Verdict::Unk,
            "{}: {}",
            which.id(),
            found.detail
        );
        assert!(
            found.detail.contains(base),
            "{}: {}",
            which.id(),
            found.detail
        );
        let why = metaharness_codex::RECORD_SILENCES
            .iter()
            .find(|(row, _)| *row == which)
            .map(|(_, why)| *why)
            .expect("the adapter states why");
        assert!(
            found.detail.contains(why),
            "{}: {}",
            which.id(),
            found.detail
        );
        assert!(found.detail.contains("invariant 3"), "{}", found.detail);
    }
    // A strict run on this record is still nobody-found-out, now with H1b decided.
    assert_eq!(row_of(&rows, HermeticRow::H1b).verdict, Verdict::Ok);
    assert_eq!(report_with(rows).exit(), RunExit::NoVerdict);
}

/// A silence is the adapter's statement about a field its record lacks. A record that carries the
/// field is decided from it, and the sentence is not printed over a verdict it did not reach.
#[test]
fn a_silence_is_never_printed_over_a_record_that_carries_the_field() {
    let spec = RunSpec::new(Kind::Codex);
    let pins = codex_pins();
    let carried = with_record(codex_record(Some("pragmatic")), |record| {
        if let Event::SessionStarted {
            plugins,
            mcp_servers,
            credential_source,
            ..
        } = record
        {
            *plugins = Some(Vec::new());
            *mcp_servers = Some(Vec::new());
            *credential_source = Some("none".to_string());
        }
    });
    let rows = hermetic_floor(&[carried], &codex_inputs(&spec, &pins));
    for which in [HermeticRow::H1a, HermeticRow::H4, HermeticRow::H5] {
        let found = row_of(&rows, which);
        assert_eq!(
            found.verdict,
            Verdict::Ok,
            "{}: {}",
            which.id(),
            found.detail
        );
        assert!(!found.detail.contains("invariant 3"), "{}", found.detail);
    }
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

// ------------------------------------------------- built-in plugins (amendment a33)

/// A plugin entry as a vendor opening record spells it: a name and a source, no version.
fn plugin(name: &str, source: Option<&str>) -> PluginRef {
    PluginRef {
        name: Some(name.to_string()),
        source: source.map(ToString::to_string),
        version: None,
    }
}

/// The four plugins takt's operator-login session on Claude Code 2.1.304 opened with, in the
/// order its `system/init` record listed them (relayed 2026-10-20; the story's *Observed in an
/// operator-login session*).
fn operator_login_builtins() -> Vec<PluginRef> {
    [
        "cc-plugin-sec-default",
        "cc-plugin-agents-md",
        "cc-plugin-telemetry",
        "cc-plugin-plugin-authoring",
    ]
    .into_iter()
    .map(|name| plugin(name, Some(&format!("{name}@builtin"))))
    .collect()
}

/// An opening record on the first pinned Claude version carrying these plugins: the built-in lists
/// are keyed by the version the record reports.
fn with_plugins(plugins: Vec<PluginRef>) -> Event {
    with_plugins_on(metaharness_claude::PINNED_VERSIONS[0], plugins)
}

fn with_plugins_on(version: &str, plugins: Vec<PluginRef>) -> Event {
    with_record(good_record(), |record| {
        if let Event::SessionStarted {
            plugins: list,
            harness_version,
            ..
        } = record
        {
            *list = Some(plugins);
            *harness_version = Some(version.to_string());
        }
    })
}

fn h1a(record: Event, declared: &[String], builtins: &[BuiltinPlugin]) -> (Verdict, String) {
    let spec = RunSpec::new(Kind::Claude);
    let rows = hermetic_floor(
        &[record],
        &FloorInputs {
            builtin_plugins: builtins,
            ..inputs(&spec, &pins(), declared)
        },
    );
    let row = rows
        .iter()
        .find(|row| row.row == HermeticRow::H1a)
        .expect("H1a is evaluated");
    (row.verdict, row.detail.clone())
}

#[test]
fn the_claude_adapter_declares_every_builtin_of_each_pinned_version_with_the_builtin_source() {
    let builtins = builtin_plugins(Kind::Claude);
    let mut expected = Vec::new();
    for version in metaharness_claude::PINNED_VERSIONS {
        let names = metaharness_claude::builtin_plugins(version).expect("every pin has a list");
        assert!(!names.is_empty(), "{version}");
        for name in names {
            expected.push(BuiltinPlugin {
                version: version.to_string(),
                name: (*name).to_string(),
                source: format!("{name}@builtin"),
            });
        }
    }
    assert_eq!(builtins, expected);
    assert_eq!(builtin_plugins(Kind::Codex), Vec::<BuiltinPlugin>::new());
    assert_eq!(builtin_plugins(Kind::B10x), Vec::<BuiltinPlugin>::new());
}

/// The lists are keyed by version: a record on a version the adapter does not pin carries
/// plugins nobody declared a list for, whatever their source says.
#[test]
fn a_builtin_in_a_record_on_an_unpinned_version_is_a_gap_and_on_each_pinned_one_is_ok() {
    for version in metaharness_claude::PINNED_VERSIONS {
        let (verdict, detail) = h1a(
            with_plugins_on(version, operator_login_builtins()),
            &[],
            &builtin_plugins(Kind::Claude),
        );
        assert_eq!(verdict, Verdict::Ok, "{version}: {detail}");
    }
    let (verdict, detail) = h1a(
        with_plugins_on("2.1.303", operator_login_builtins()),
        &[],
        &builtin_plugins(Kind::Claude),
    );
    assert_eq!(verdict, Verdict::Gap, "{detail}");
}

/// The vector built from takt's operator-login list: four built-ins, each matched on its name
/// **and** its `@builtin` source, and the row names every one it saw.
#[test]
fn an_operator_login_record_carrying_the_four_builtins_the_pin_compiles_is_ok_and_names_each() {
    let (verdict, detail) = h1a(
        with_plugins(operator_login_builtins()),
        &[],
        &builtin_plugins(Kind::Claude),
    );
    assert_eq!(verdict, Verdict::Ok, "{detail}");
    for name in [
        "cc-plugin-sec-default",
        "cc-plugin-agents-md",
        "cc-plugin-telemetry",
        "cc-plugin-plugin-authoring",
    ] {
        assert!(
            detail.contains(&format!("{name}@builtin")),
            "the row names {name}: {detail}"
        );
    }
}

/// The safety fact: an ambient plugin that borrows a built-in's name and arrives from anywhere
/// else is a plugin nobody declared. A comparison on the name alone would pass it.
#[test]
fn an_ambient_plugin_named_like_a_builtin_with_another_source_is_a_gap() {
    let mut plugins = operator_login_builtins();
    plugins[2] = plugin(
        "cc-plugin-telemetry",
        Some("cc-plugin-telemetry@some-marketplace"),
    );
    let (verdict, detail) = h1a(with_plugins(plugins), &[], &builtin_plugins(Kind::Claude));
    assert_eq!(verdict, Verdict::Gap, "{detail}");
    assert!(
        detail.contains("cc-plugin-telemetry@some-marketplace"),
        "the row names the foreign source: {detail}"
    );
}

#[test]
fn a_builtin_name_with_no_source_is_a_gap_because_a_name_alone_matches_nothing() {
    let (verdict, detail) = h1a(
        with_plugins(vec![plugin("cc-plugin-agents-md", None)]),
        &[],
        &builtin_plugins(Kind::Claude),
    );
    assert_eq!(verdict, Verdict::Gap, "{detail}");
}

#[test]
fn a_builtin_source_the_pinned_binary_does_not_compile_is_a_gap() {
    let (verdict, detail) = h1a(
        with_plugins(vec![plugin(
            "cc-plugin-not-in-this-binary",
            Some("cc-plugin-not-in-this-binary@builtin"),
        )]),
        &[],
        &builtin_plugins(Kind::Claude),
    );
    assert_eq!(verdict, Verdict::Gap, "{detail}");
}

/// Name and source are one pair: a compiled name carrying another built-in's source is neither.
#[test]
fn a_builtin_name_carrying_another_builtins_source_is_a_gap() {
    let (verdict, detail) = h1a(
        with_plugins(vec![plugin(
            "cc-plugin-agents-md",
            Some("cc-plugin-telemetry@builtin"),
        )]),
        &[],
        &builtin_plugins(Kind::Claude),
    );
    assert_eq!(verdict, Verdict::Gap, "{detail}");
}

/// Built-ins widen what the record may carry; they never stand in for a declared plugin.
#[test]
fn a_declared_plugin_beside_builtins_is_ok_and_one_missing_beside_them_is_still_a_gap() {
    let declared = vec!["mine".to_string()];
    let mut plugins = operator_login_builtins();
    plugins.push(plugin("mine", Some("mine@inline")));
    let (verdict, detail) = h1a(
        with_plugins(plugins),
        &declared,
        &builtin_plugins(Kind::Claude),
    );
    assert_eq!(verdict, Verdict::Ok, "{detail}");
    assert!(detail.contains("mine"), "{detail}");

    let (verdict, detail) = h1a(
        with_plugins(operator_login_builtins()),
        &declared,
        &builtin_plugins(Kind::Claude),
    );
    assert_eq!(verdict, Verdict::Gap, "{detail}");
}

/// A kind that declares no built-in admits none, whatever the source says.
#[test]
fn a_builtin_source_on_a_kind_that_declares_no_builtins_is_a_gap() {
    let (verdict, detail) = h1a(with_plugins(operator_login_builtins()), &[], &[]);
    assert_eq!(verdict, Verdict::Gap, "{detail}");
}

/// A plugin entry with no name is a plugin the record carried and nobody declared, not a plugin
/// the comparison may skip.
#[test]
fn a_plugin_the_record_gives_no_name_is_a_gap_and_is_not_dropped() {
    let nameless = PluginRef {
        name: None,
        source: Some("somewhere@inline".to_string()),
        version: None,
    };
    let (verdict, detail) = h1a(
        with_plugins(vec![nameless]),
        &[],
        &builtin_plugins(Kind::Claude),
    );
    assert_eq!(verdict, Verdict::Gap, "{detail}");
}

/// takt's opening record read by the Claude adapter's own reader and judged with the adapter's own
/// pin and built-in list: H1a and H9 both read what the record observed.
#[test]
fn takts_operator_login_init_record_read_by_the_claude_reader_passes_h1a_and_h9() {
    let init = r#"{"type":"system","subtype":"init","claude_code_version":"2.1.304","cwd":"/scratch/work","tools":["Bash"],"mcp_servers":[],"output_style":"default","apiKeySource":"none","plugins":[{"name":"cc-plugin-sec-default","source":"cc-plugin-sec-default@builtin"},{"name":"cc-plugin-agents-md","source":"cc-plugin-agents-md@builtin"},{"name":"cc-plugin-telemetry","source":"cc-plugin-telemetry@builtin"},{"name":"cc-plugin-plugin-authoring","source":"cc-plugin-plugin-authoring@builtin"}]}"#;
    let mut reader = metaharness_claude::TranscriptReader::new(
        TranscriptRef {
            path: Some("/scratch/transcript.jsonl".to_string()),
            digest: Some(Digest::of(b"bytes")),
            bytes: Some(1),
        },
        attestation_imposing(&[HermeticRow::H2, HermeticRow::H3, HermeticRow::H8]),
    );
    let events: Vec<Event> = reader
        .push_line(init)
        .into_iter()
        .map(|emission| emission.event)
        .collect();
    let spec = RunSpec::new(Kind::Claude);
    let pins: Vec<String> = metaharness_claude::PINNED_VERSIONS
        .iter()
        .map(ToString::to_string)
        .collect();
    let builtins = builtin_plugins(Kind::Claude);
    let rows = hermetic_floor(
        &events,
        &FloorInputs {
            builtin_plugins: &builtins,
            ..inputs(&spec, &pins, &[])
        },
    );
    let row = |which| rows.iter().find(|row| row.row == which).expect("evaluated");
    assert_eq!(
        row(HermeticRow::H1a).verdict,
        Verdict::Ok,
        "{}",
        row(HermeticRow::H1a).detail
    );
    assert_eq!(
        row(HermeticRow::H9).verdict,
        Verdict::Ok,
        "{}",
        row(HermeticRow::H9).detail
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

/// The defect the **first live run** found, kept as a row.
///
/// 2.1.239 writes `apiKeySource: "none"` under an operator login — there is no API *key*,
/// because the session authenticates from the copied credential file. H4 read that as a
/// mismatch and failed a run that was hermetic in every way the row exists to check. No free
/// tier could have caught it: there is no real opening record below C4 (amendment a4).
#[test]
fn an_operator_login_reporting_no_api_key_source_is_exactly_what_h4_wants() {
    let spec = RunSpec::new(Kind::Claude); // defaults to operator-login
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted {
            credential_source, ..
        } = record
        {
            *credential_source = Some("none".to_string());
        }
    });
    assert_eq!(
        verdict_of(
            &hermetic_floor(&[record], &inputs(&spec, &pins(), &[])),
            HermeticRow::H4
        ),
        Verdict::Ok
    );
}

/// A credential-free run with a declared endpoint, whose record names the API-key variable and
/// whose launch attested H4 with `how` (design amendment a33).
fn endpoint_record(how: &str) -> Event {
    with_record(good_record(), |record| {
        if let Event::SessionStarted {
            credential_source,
            hermetic,
            ..
        } = record
        {
            *credential_source = Some("ANTHROPIC_API_KEY".to_string());
            hermetic
                .imposed
                .retain(|control| control.row != HermeticRow::H4);
            hermetic.imposed.push(ImposedControl {
                row: HermeticRow::H4,
                how: how.to_string(),
            });
        }
    })
}

fn h4_of(record: Event, posture: Option<&str>) -> (Verdict, String) {
    let spec = RunSpec {
        credentials: CredentialSource::None,
        model_endpoint: Some("http://127.0.0.1:1".to_string()),
        ..RunSpec::new(Kind::Claude)
    };
    let rows = hermetic_floor(
        &[record],
        &FloorInputs {
            endpoint_key_posture: posture,
            secret_key_source: None,
            ..inputs(&spec, &pins(), &[])
        },
    );
    let row = rows
        .iter()
        .find(|row| row.row == HermeticRow::H4)
        .expect("H4 is evaluated");
    (row.verdict, row.detail.clone())
}

/// The key the record names is the launch's own endpoint placeholder, and the launch's attestation
/// — not the variable's name — is what says so (design amendment a33).
#[test]
fn h4_reads_the_launchs_own_endpoint_placeholder_as_no_operator_key() {
    let posture = metaharness_claude::ENDPOINT_KEY_POSTURE;
    let (verdict, detail) = h4_of(endpoint_record(posture), Some(posture));
    assert_eq!(verdict, Verdict::Ok, "{detail}");
    assert!(detail.contains("placeholder"), "{detail}");
}

/// The same record without that attestation is the undeclared key it always was: the name alone
/// proves nothing, and neither does a posture sentence on a kind that declares none.
#[test]
fn h4_still_reads_a_key_the_launch_did_not_attest_as_its_placeholder_as_a_gap() {
    let posture = metaharness_claude::ENDPOINT_KEY_POSTURE;
    let (verdict, detail) = h4_of(
        endpoint_record("ANTHROPIC_API_KEY is absent from the child environment"),
        Some(posture),
    );
    assert_eq!(verdict, Verdict::Gap, "{detail}");
    let (verdict, detail) = h4_of(endpoint_record(posture), None);
    assert_eq!(verdict, Verdict::Gap, "{detail}");
}

/// When the launch could not make the placeholder claim — managed settings, which
/// `--setting-sources ""` does not switch off, were present — H4 reads the key the record names
/// as it did before a33, and its reason carries what the launch said (design amendment a33).
#[test]
fn h4_over_an_endpoint_run_beside_managed_settings_is_a_gap_that_names_them() {
    let why = "managed settings exist at /etc/claude-code/managed-settings.json, which \
               --setting-sources \"\" does not switch off";
    let record = with_record(endpoint_record("unused"), |record| {
        if let Event::SessionStarted { hermetic, .. } = record {
            hermetic
                .imposed
                .retain(|control| control.row != HermeticRow::H4);
            hermetic.unavailable.push(UnavailableControl {
                row: HermeticRow::H4,
                why: why.to_string(),
            });
        }
    });
    let (verdict, detail) = h4_of(record, Some(metaharness_claude::ENDPOINT_KEY_POSTURE));
    assert_eq!(verdict, Verdict::Gap, "{detail}");
    assert!(detail.contains("managed settings"), "{detail}");
}

/// The other half of the same live failure: a run that pins no input tree has no governing
/// document that could move under it, so H10 is satisfied rather than unknown. Reading it as
/// `unk` made `--hermetic strict` unpassable for every run that pins nothing — including the
/// design's own worked example (amendment a4).
#[test]
fn a_run_that_pinned_no_input_tree_satisfies_h10_rather_than_leaving_it_unknown() {
    let spec = RunSpec::new(Kind::Claude);
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted { inputs_digest, .. } = record {
            *inputs_digest = None;
        }
    });
    let rows = hermetic_floor(&[record], &inputs(&spec, &pins(), &[]));
    assert_eq!(verdict_of(&rows, HermeticRow::H10), Verdict::Ok);
    let detail = rows
        .iter()
        .find(|row| row.row == HermeticRow::H10)
        .map(|row| row.detail.clone())
        .unwrap_or_default();
    assert!(
        detail.contains("pinned no input tree"),
        "the row must say why it passed, not merely that it did: {detail}"
    );
}

/// And the row still fails nothing quietly: a run that **did** pin a tree reports its digest.
#[test]
fn a_run_that_pinned_a_tree_reports_its_digest_in_h10() {
    let spec = RunSpec::new(Kind::Claude);
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted { inputs_digest, .. } = record {
            *inputs_digest = Some(metaharness::protocol::Digest::of(b"the copied tree"));
        }
    });
    let rows = hermetic_floor(&[record], &inputs(&spec, &pins(), &[]));
    assert_eq!(verdict_of(&rows, HermeticRow::H10), Verdict::Ok);
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

/// Design a30, kept by a31: a direct-provider opening record that carries no working directory —
/// the pinned loop names none under `--substrate <socket>` or for a root whose name is not UTF-8 —
/// reads H7 `unk` and says whose silence it is, beside H1a and H1b, the floor's other
/// `direct_provider` arms.
#[test]
fn a_direct_provider_record_with_no_working_directory_is_unk_in_the_loops_words() {
    let spec = RunSpec::new(Kind::B10x);
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted {
            adapter,
            adapter_class,
            cwd,
            ..
        } = record
        {
            *adapter = "b10x".to_owned();
            *adapter_class = "direct_provider".to_owned();
            *cwd = None;
        }
    });
    let rows = hermetic_floor(&[record], &inputs(&spec, &pins(), &[]));
    let h7 = rows
        .iter()
        .find(|row| row.row == HermeticRow::H7)
        .expect("H7");
    assert_eq!(h7.verdict, Verdict::Unk, "{}", h7.detail);
    assert_eq!(h7.detail, "the loop did not report its working directory");
}

/// The vendor kinds keep their own words: their record has a cwd field, and a record without one
/// is a record that did not carry it, not a loop that never reports one.
#[test]
fn a_vendor_record_with_no_working_directory_is_unk_in_the_records_words() {
    let spec = RunSpec::new(Kind::Claude);
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted { cwd, .. } = record {
            *cwd = None;
        }
    });
    let rows = hermetic_floor(&[record], &inputs(&spec, &pins(), &[]));
    let h7 = rows
        .iter()
        .find(|row| row.row == HermeticRow::H7)
        .expect("H7");
    assert_eq!(h7.verdict, Verdict::Unk, "{}", h7.detail);
    assert_eq!(h7.detail, "the opening record carries no working directory");
}

/// One directory under two spellings: `<root>/real/work`, canonical, and `<root>/link/work`
/// through a link to `real`. The guard keeps both alive while the floor resolves them.
#[cfg(unix)]
fn one_directory_two_spellings() -> (tempfile::TempDir, String, String) {
    let root = tempfile::tempdir().expect("a scratch root");
    let real = root.path().join("real");
    std::fs::create_dir_all(real.join("work")).expect("the directory");
    let link = root.path().join("link");
    std::os::unix::fs::symlink(&real, &link).expect("the link");
    let canonical = std::fs::canonicalize(real.join("work"))
        .expect("the directory resolves")
        .display()
        .to_string();
    let linked = link.join("work").display().to_string();
    assert_ne!(canonical, linked, "two spellings");
    (root, canonical, linked)
}

/// The floor's H7 verdict and words for this record class, planned and recorded directory.
#[cfg(unix)]
fn h7_for(class: (&str, &str), planned: &str, recorded: &str) -> (Verdict, String) {
    let spec = RunSpec::new(if class.0 == "b10x" {
        Kind::B10x
    } else {
        Kind::Claude
    });
    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted {
            adapter,
            adapter_class,
            cwd,
            ..
        } = record
        {
            class.0.clone_into(adapter);
            class.1.clone_into(adapter_class);
            *cwd = Some(recorded.to_owned());
        }
    });
    let rows = hermetic_floor(
        &[record],
        &FloorInputs {
            planned_cwd: Some(planned),
            ..inputs(&spec, &pins(), &[])
        },
    );
    let h7 = rows
        .iter()
        .find(|row| row.row == HermeticRow::H7)
        .expect("H7");
    (h7.verdict, h7.detail.clone())
}

/// Design a31: H7 compares **directories, not spellings**. The pinned b10x loop records the
/// canonical root its file tools act in (harness 0.15.1, `LoopEvent::Started::cwd`), while the
/// directory metaharness creates is spelled under `TMPDIR`, which may pass through a link — a
/// `TMPDIR` under a linked home is the ordinary case. Two names are one directory when they are
/// byte-equal, or when both are absolute and resolve (links followed) to the same path.
/// Both spellings,
/// in both places, on the b10x class and on a vendor class: the rule is a fact about directories,
/// not about a kind.
#[cfg(unix)]
#[test]
fn a_recorded_directory_is_the_planned_one_under_either_spelling() {
    let (_root, canonical, linked) = one_directory_two_spellings();
    for class in [("b10x", "direct_provider"), ("claude", "harness")] {
        for (planned, recorded) in [
            (&linked, &canonical),
            (&canonical, &linked),
            (&canonical, &canonical),
            (&linked, &linked),
        ] {
            let (verdict, detail) = h7_for(class, planned, recorded);
            assert_eq!(
                verdict,
                Verdict::Ok,
                "{class:?}: planned {planned}, recorded {recorded}: {detail}"
            );
            assert!(detail.contains("which metaharness created"), "{detail}");
        }
    }
}

/// The other half of the rule: a recorded directory that resolves somewhere else is a `gap`
/// whichever spelling names it, and so is one that does not exist on this machine at all.
#[cfg(unix)]
#[test]
fn a_recorded_directory_that_resolves_elsewhere_is_a_gap_under_either_spelling() {
    let (root, canonical, linked) = one_directory_two_spellings();
    let other = root.path().join("other");
    std::fs::create_dir(&other).expect("another directory");
    let other_link = root.path().join("other-link");
    std::os::unix::fs::symlink(&other, &other_link).expect("a link to it");
    for planned in [&canonical, &linked] {
        for recorded in [
            other.display().to_string(),
            other_link.display().to_string(),
            "/nonexistent/metaharness/work".to_owned(),
        ] {
            let (verdict, detail) = h7_for(("b10x", "direct_provider"), planned, &recorded);
            assert_eq!(
                verdict,
                Verdict::Gap,
                "planned {planned}, recorded {recorded}: {detail}"
            );
            assert!(
                detail.contains(&format!(
                    "metaharness planned {planned} and the session ran in {recorded}"
                )),
                "{detail}"
            );
        }
    }
}

/// A relative name is never resolved: resolving it would resolve it against
/// metaharness's own working directory, which says nothing about where the loop worked. So a
/// record that says `.` is not the planned directory even when metaharness itself runs there.
#[cfg(unix)]
#[test]
fn a_relative_recorded_directory_is_not_resolved_against_metaharnesss_own() {
    let here = std::fs::canonicalize(".")
        .expect("the test runs somewhere")
        .display()
        .to_string();
    for recorded in [".", "./"] {
        let (verdict, detail) = h7_for(("b10x", "direct_provider"), &here, recorded);
        assert_eq!(
            verdict,
            Verdict::Gap,
            "planned {here}, recorded {recorded}: {detail}"
        );
    }
}

/// The same rule for an absolute name whose meaning is *the process that resolves it*: a magic
/// link under `/proc` (`/proc/self/`, `/proc/thread-self/`, `/proc/<pid>/`) or under `/dev/fd/`.
/// Resolved inside metaharness, each names metaharness's own working directory or file descriptor,
/// which says nothing about where the loop worked (a31 point 3), so each is compared as spelled and
/// reads `gap` even where metaharness itself runs in the planned directory. Every spelling below
/// resolves to the test process's working directory, which is the planned one here, and so read
/// `ok` before the rule: the plain prefixes, `.`/`..`/doubled-slash spellings of them, `/dev/stdin`'s
/// kind of link into them (`/dev/fd` is itself a link to `/proc/self/fd`), and an ordinary link
/// whose target is one of them, because the check is made at every step of the resolution and not on
/// the spelling.
#[cfg(target_os = "linux")]
#[test]
fn a_recorded_directory_that_names_the_resolving_process_is_compared_as_spelled() {
    use std::os::fd::AsRawFd as _;
    let here = std::fs::canonicalize(".")
        .expect("the test runs somewhere")
        .display()
        .to_string();
    let open = std::fs::File::open(&here).expect("the planned directory opens");
    let fd = open.as_raw_fd();
    let links = tempfile::tempdir().expect("a scratch root");
    let to_cwd = links.path().join("to-proc-self-cwd");
    std::os::unix::fs::symlink("/proc/self/cwd", &to_cwd).expect("a link into /proc");
    let to_fd = links.path().join("to-dev-fd");
    std::os::unix::fs::symlink(format!("/dev/fd/{fd}"), &to_fd).expect("a link into /dev/fd");
    let pid = std::process::id();
    let mut resolved_inside_metaharness = Vec::new();
    for recorded in [
        "/proc/self/cwd".to_owned(),
        "/proc/thread-self/cwd".to_owned(),
        format!("/proc/{pid}/cwd"),
        format!("/proc/self/task/{pid}/cwd"),
        format!("/proc/self/root{here}"),
        format!("/proc/self/fd/{fd}"),
        format!("/dev/fd/{fd}"),
        "/proc/./self/cwd".to_owned(),
        "//proc//self/cwd".to_owned(),
        "/tmp/../proc/self/cwd".to_owned(),
        to_cwd.display().to_string(),
        to_fd.display().to_string(),
    ] {
        assert_eq!(
            std::fs::canonicalize(&recorded)
                .ok()
                .map(|p| p.display().to_string()),
            Some(here.clone()),
            "{recorded} resolves to the test process's working directory, so it tests the rule"
        );
        let (verdict, detail) = h7_for(("b10x", "direct_provider"), &here, &recorded);
        if (verdict, detail.as_str())
            != (
                Verdict::Gap,
                format!("metaharness planned {here} and the session ran in {recorded}").as_str(),
            )
        {
            resolved_inside_metaharness.push(format!("{recorded}: {verdict:?} — {detail}"));
        }
    }
    drop(open);
    assert!(
        resolved_inside_metaharness.is_empty(),
        "planned {here}; these were resolved against metaharness's own process:\n{}",
        resolved_inside_metaharness.join("\n")
    );
}

/// A name that passes through something that is not a directory does not resolve, as `realpath`
/// and the kernel refuse it (`ENOTDIR`): a file has no `..`, no `.`, and no trailing `/`. Each name
/// below would arrive lexically at the planned name, and so read `ok` if a file were treated as a
/// directory; each is compared as spelled and reads `gap`. Through a file directly, through a link
/// to a file, and through a link whose own target ends in `/`.
#[cfg(unix)]
#[test]
fn a_recorded_directory_that_passes_through_a_file_does_not_resolve() {
    let root = tempfile::tempdir().expect("a scratch root");
    let work = std::fs::canonicalize(root.path())
        .expect("the root resolves")
        .join("work");
    std::fs::create_dir(&work).expect("the planned directory");
    std::fs::write(work.join("README.md"), "a file\n").expect("a file in it");
    std::os::unix::fs::symlink("README.md", work.join("to-file")).expect("a link to the file");
    std::os::unix::fs::symlink("README.md/", work.join("to-file-slash"))
        .expect("a link to the file spelled as a directory");
    let dir = work.display().to_string();
    let file = work.join("README.md").display().to_string();
    let mut read_as_resolved = Vec::new();
    for (planned, recorded) in [
        (&dir, format!("{file}/..")),
        (&dir, format!("{file}/./..")),
        (&dir, format!("{file}/../.")),
        (&dir, format!("{dir}/to-file/..")),
        (&file, format!("{file}/")),
        (&file, format!("{file}/.")),
        (&file, format!("{file}//")),
        (&file, format!("{dir}/to-file/")),
        (&file, format!("{dir}/to-file-slash")),
    ] {
        let refused = std::fs::canonicalize(&recorded).expect_err("realpath refuses it");
        assert_eq!(refused.raw_os_error(), Some(20), "ENOTDIR for {recorded}");
        let (verdict, detail) = h7_for(("b10x", "direct_provider"), planned, &recorded);
        if (verdict, detail.as_str())
            != (
                Verdict::Gap,
                format!("metaharness planned {planned} and the session ran in {recorded}").as_str(),
            )
        {
            read_as_resolved.push(format!("{recorded}: {verdict:?} — {detail}"));
        }
    }
    assert!(
        read_as_resolved.is_empty(),
        "names realpath refuses with ENOTDIR were resolved:\n{}",
        read_as_resolved.join("\n")
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
        withheld: None,
        installed_plugins: Vec::new(),
        auditor: None,
        saw_terminal_record: true,
        // Supply successful synthetic termination independently of the rows under
        // test: missing termination must now fail even when every row passes.
        stream: metaharness::protocol::StreamCompleteness::Complete {
            events: 1,
            reason: metaharness::protocol::CloseReason::Completed,
        },
        child_exit: metaharness::protocol::ProcessTermination::Unknown,
    };
    assert_eq!(
        report.exit(),
        RunExit::Ok,
        "if any unk failed a strict run then every strict run would fail forever (F3)"
    );
    let rendered = report.render();
    assert!(
        rendered.lines().any(|line| line == "control verdict: ok"),
        "an advisory unk does not enter the control verdict (amendment a48): {rendered}"
    );
}

// ---------------------------------------------------------------- exit codes (§ 9.4)

fn report_with(rows: Vec<metaharness::protocol::RowVerdict>) -> AuditReport {
    report_withholding(rows, None)
}

fn report_withholding(
    rows: Vec<metaharness::protocol::RowVerdict>,
    withheld: Option<Vec<WithheldTool>>,
) -> AuditReport {
    AuditReport {
        rows,
        census: DecisionCensus::default(),
        withheld,
        installed_plugins: Vec::new(),
        auditor: None,
        saw_terminal_record: true,
        // These row tests hold the independent terminal outcome at success.
        stream: metaharness::protocol::StreamCompleteness::Complete {
            events: 1,
            reason: metaharness::protocol::CloseReason::Completed,
        },
        // Nothing measured a child here, and the report says `unknown` rather than `0` (a48).
        child_exit: metaharness::protocol::ProcessTermination::Unknown,
    }
}

fn floor_for(record: Event) -> Vec<metaharness::protocol::RowVerdict> {
    let spec = RunSpec::new(Kind::Claude);
    hermetic_floor(&[record], &inputs(&spec, &pins(), &[]))
}

#[test]
fn codex_terminal_regression_audit_needs_complete_success_evidence() {
    use metaharness::protocol::{CloseReason, StreamCompleteness};
    for stream in [
        StreamCompleteness::Truncated,
        StreamCompleteness::Inconsistent {
            detail: "synthetic count mismatch".into(),
            reason: CloseReason::Completed,
        },
        StreamCompleteness::Complete {
            events: 1,
            reason: CloseReason::Error,
        },
        StreamCompleteness::Complete {
            events: 1,
            reason: CloseReason::Budget,
        },
        StreamCompleteness::Complete {
            events: 1,
            reason: CloseReason::SteerHalt,
        },
    ] {
        let mut report = report_with(Vec::new());
        report.stream = stream;
        assert_eq!(report.exit(), RunExit::NoVerdict);
    }
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

// ---------------------------------------------------------------- the three verdict lines (a48)

/// The last three lines of the report: the child's exit, the control verdict and the audit
/// verdict, each on a line of its own (amendment a48).
fn verdict_lines(report: &AuditReport) -> Vec<String> {
    let rendered = report.render();
    let lines: Vec<&str> = rendered.lines().collect();
    lines[lines.len().saturating_sub(3)..]
        .iter()
        .map(|line| (*line).to_owned())
        .collect()
}

#[test]
fn a_gating_unknown_still_exits_three_and_the_report_names_each_verdict_apart() {
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
    assert_eq!(
        verdict_lines(&report),
        [
            "child exit: unknown",
            "control verdict: unk",
            "audit verdict: no verdict (exit 3)"
        ]
    );
}

#[test]
fn a_child_nothing_measured_prints_unknown_and_never_zero() {
    let report = report_with(floor_for(good_record()));
    assert_eq!(report.exit(), RunExit::Ok);
    let rendered = report.render();
    assert!(
        rendered.lines().any(|line| line == "child exit: unknown"),
        "{rendered}"
    );
    assert!(!rendered.contains("child exit: 0"), "{rendered}");
}

#[test]
fn the_control_verdict_folds_the_gating_rows_gap_over_unk_over_ok() {
    let version = |value: Option<&str>| {
        with_record(good_record(), |record| {
            if let Event::SessionStarted {
                harness_version, ..
            } = record
            {
                *harness_version = value.map(str::to_owned);
            }
        })
    };
    let both = with_record(version(None), |record| {
        if let Event::SessionStarted { output_style, .. } = record {
            *output_style = Some("Explanatory".to_string());
        }
    });
    for (record, control, audit) in [
        (good_record(), "ok", "ok (exit 0)"),
        (version(None), "unk", "no verdict (exit 3)"),
        (version(Some("9.9.9")), "gap", "gap (exit 1)"),
        (both, "gap", "gap (exit 1)"),
    ] {
        let lines = verdict_lines(&report_with(floor_for(record)));
        assert_eq!(
            lines[1..],
            [
                format!("control verdict: {control}"),
                format!("audit verdict: {audit}")
            ]
        );
    }
}

#[test]
fn an_auditors_exit_moves_the_audit_verdict_and_never_the_control_verdict() {
    for (auditor_code, audit) in [
        (1, "audit verdict: gap (exit 1)"),
        (3, "audit verdict: no verdict (exit 3)"),
        (0, "audit verdict: ok (exit 0)"),
    ] {
        let mut report = report_with(floor_for(good_record()));
        report.auditor = Some(metaharness::AuditorVerdict {
            argv: vec!["protocol".to_string()],
            exit_code: Some(auditor_code),
            verdict_rows: 7,
        });
        assert_eq!(
            verdict_lines(&report),
            ["child exit: unknown", "control verdict: ok", audit],
            "auditor exit {auditor_code}"
        );
    }
}

/// A stream that never carried a terminal record exits `3` while every control held: the two
/// verdicts are different facts, and the report says both.
#[test]
fn a_run_with_no_terminal_record_exits_three_beside_a_control_verdict_of_ok() {
    let mut report = report_with(floor_for(good_record()));
    report.saw_terminal_record = false;
    assert_eq!(report.exit(), RunExit::NoVerdict);
    assert_eq!(
        verdict_lines(&report),
        [
            "child exit: unknown",
            "control verdict: ok",
            "audit verdict: no verdict (exit 3)"
        ]
    );
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

// ------------------------------------------------- what the machine would not admit (a12)

/// The census cannot answer this and neither can the tool lists. A tool the machine would not
/// admit was never put in front of the model, so nothing was ever refused for it and every
/// denial count stays zero; and it is missing from `offered_tools` and `available_operations` in
/// exactly the way a tool nobody wanted is. On 2026-08-29 that read as a model failure for weeks.
#[test]
fn the_report_names_each_tool_the_machine_would_not_admit_and_the_predicate_that_decided() {
    let rendered = report_withholding(
        floor_for(good_record()),
        Some(vec![
            WithheldTool {
                tool: "run".to_string(),
                reason: "`exec.argv-only` must be true and this machine says nothing.".to_string(),
            },
            WithheldTool {
                tool: "net".to_string(),
                reason: "no egress fact.".to_string(),
            },
        ]),
    )
    .render();
    assert!(
        rendered.contains(
            "withheld: run (`exec.argv-only` must be true and this machine says nothing.); \
             net (no egress fact.)"
        ),
        "{rendered}"
    );
}

/// Amendment a16's reporting half: **the report always has a `plugins:` line**, and it says
/// `none` rather than saying nothing.
///
/// A report that printed nothing for a plugin-less run would make *this run installed nothing* and
/// *this build does not report installations* the same bytes, which is the reading § 8.1 refuses
/// everywhere else.
#[test]
fn a_run_with_no_plugin_prints_plugins_none_rather_than_printing_nothing() {
    let rendered = report_with(floor_for(good_record())).render();
    assert!(rendered.contains("plugins: none"), "{rendered}");
}

/// And a run that installed one lists it, with where it came from, where it went, its digest and
/// **how strong the claim that it will load is** — which is the whole of `loaded_by`.
#[test]
fn a_run_that_installed_a_plugin_lists_it_with_its_digest_and_how_it_is_loaded() {
    let mut report = report_with(floor_for(good_record()));
    report.installed_plugins = vec![metaharness::protocol::InstalledPlugin {
        name: "aep-plan".to_string(),
        source: "beyond10x/agentplugins@aep-plan@0.6.2 (marketplace beyond10x)".to_string(),
        installed_at: "/scratch/claude-home/plugins/cache/beyond10x/aep-plan/0.6.2".to_string(),
        digest: metaharness::protocol::Digest::of(b"the tree"),
        loaded_by: "placed in the scratch config home's plugin registry — not driven".to_string(),
    }];
    let rendered = report.render();
    assert!(rendered.contains("plugins: aep-plan"), "{rendered}");
    assert!(rendered.contains("0.6.2"), "{rendered}");
    assert!(rendered.contains("not driven"), "{rendered}");
    assert!(!rendered.contains("plugins: none"), "{rendered}");
}

/// The two silences the field exists to keep apart, printed apart. A harness that states `[]`
/// has said *this run got everything it asked for*; one that states nothing has said nothing,
/// and printing the second as the first would assert a fact nobody observed (invariant 3).
#[test]
fn a_harness_that_withheld_nothing_and_one_that_did_not_say_render_differently() {
    let declared = report_withholding(floor_for(good_record()), Some(Vec::new())).render();
    assert!(declared.contains("withheld: none declared"), "{declared}");

    let silent = report_with(floor_for(good_record())).render();
    assert!(
        silent.contains("withheld: not stated by the harness"),
        "{silent}"
    );
    assert!(
        !silent.contains("none declared"),
        "silence is not an empty list: {silent}"
    );
}

/// The line is read off the opening record, and a stream with no opening record states nothing
/// rather than an empty list.
#[test]
fn the_withheld_line_is_read_from_the_opening_record() {
    assert_eq!(withheld_tools(&[]), None);
    assert_eq!(withheld_tools(&[good_record()]), None);

    let record = with_record(good_record(), |record| {
        if let Event::SessionStarted { withheld, .. } = record {
            *withheld = Some(vec![WithheldTool {
                tool: "run".to_string(),
                reason: "the capability facts this gate reads are absent.".to_string(),
            }]);
        }
    });
    let named = withheld_tools(&[record]).expect("the record named one");
    assert_eq!(named.len(), 1);
    assert_eq!(named[0].tool, "run");
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
        final_answer: None,
        observed_models: None,
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
fn the_auditor_prefix_is_argv_and_a_multi_word_subcommand_is_not_a_special_case() {
    let argv = auditor_argv(
        "protocol observe trace check",
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
            "observe",
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
    assert_eq!(auditor.calls(), Vec::<Vec<String>>::new());
}

#[test]
fn an_auditor_with_no_spec_is_a_refusal_because_there_is_nothing_to_check() {
    let mut auditor = FakeAuditor::default();
    let refused = run_auditor(
        None,
        Some("protocol observe trace check"),
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
            Some("protocol observe trace check"),
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
        Some("protocol observe trace check"),
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
            // The version is read off the pin rather than written out, because this record
            // stands for "a well-formed run on the version the adapter is pinned to" — and a
            // literal here means every pin move fails a test about something else entirely.
            ScriptStep::line(format!(
                r#"{{"emit":"session.started","harness_version":"{}","output_style":"default","plugins":[],"mcp_servers":[],"credential_source":"operator-login","inputs_digest":"tree"}}"#,
                metaharness_claude::PINNED_VERSIONS[0]
            )),
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
    // a12 travels the whole path: the field is read off this run's own opening record, and the
    // claude wire that record models names no withheld tool, so the report says *it did not say*.
    assert_eq!(report.withheld, None);
    assert!(
        report
            .render()
            .contains("withheld: not stated by the harness")
    );
    assert!(
        run.transcript().path.is_some(),
        "O8: the bytes are retained"
    );

    // H7 is unk here because the scratch cwd is not in the fixture record — which is exactly
    // the point: absence of evidence is not hermeticity. H10 passes, because a run that pinned
    // no tree has none that could move (amendment a4).
    assert_eq!(run.exit(Some(&report)), RunExit::NoVerdict);
}

/// The launch hands the floor the adapter's built-in list (amendment a33): a Claude run whose
/// opening record carries takt's four built-ins reads H1a `ok` through the run, and one whose
/// record carries an ambient plugin under a built-in's name reads `gap`.
#[test]
fn a_claude_run_judges_builtins_against_the_list_its_launch_declared() {
    let judge = |telemetry_source: &str| {
        let init = format!(
            r#"{{"type":"system","subtype":"init","claude_code_version":"{}","tools":[],"mcp_servers":[],"output_style":"default","apiKeySource":"none","plugins":[{{"name":"cc-plugin-sec-default","source":"cc-plugin-sec-default@builtin"}},{{"name":"cc-plugin-agents-md","source":"cc-plugin-agents-md@builtin"}},{{"name":"cc-plugin-telemetry","source":"{telemetry_source}"}},{{"name":"cc-plugin-plugin-authoring","source":"cc-plugin-plugin-authoring@builtin"}}]}}"#,
            metaharness_claude::PINNED_VERSIONS[0]
        );
        let mut runner = ScriptedRunner::new(
            vec![
                ScriptStep::line(init),
                ScriptStep::line(r#"{"type":"result","subtype":"success","is_error":false}"#),
            ],
            ScriptedLog::new(),
        );
        let mut run = Metaharness::new(Kind::Claude)
            .with_audit(true)
            .start_with_clock(
                Input::Prompt("do it".to_string()),
                &mut runner,
                &mut ClaudeSeams,
                Box::new(ManualClock::new()),
            )
            .expect("starts");
        run.drain().expect("drains");
        let rows = run.hermetic_floor();
        let row = |which| {
            rows.iter()
                .find(|row| row.row == which)
                .cloned()
                .expect("evaluated")
        };
        (row(HermeticRow::H1a), row(HermeticRow::H9))
    };

    let (h1a, h9) = judge("cc-plugin-telemetry@builtin");
    assert_eq!(h1a.verdict, Verdict::Ok, "{}", h1a.detail);
    assert!(
        h1a.detail.contains("cc-plugin-sec-default@builtin"),
        "{}",
        h1a.detail
    );
    assert_eq!(h9.verdict, Verdict::Ok, "{}", h9.detail);

    let (h1a, _) = judge("cc-plugin-telemetry@some-marketplace");
    assert_eq!(h1a.verdict, Verdict::Gap, "{}", h1a.detail);
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

// ---------------------------------------------------------------- the pairing, before the spawn
//
// Amendment a43. Every case starts over `ScriptedRunner`, whose log counts the children it was
// asked to start, so "refused before the harness starts" is read off that count and never off the
// refusal alone.

/// An auditor `start` resolves: a script given mode `0700`, named by absolute path, then the three
/// words the reference auditor takes after its program. The directory holds it; nothing here runs
/// it, because every audit below goes through `FakeAuditor`.
///
/// Every case that names an auditor names this one unless the case is about the auditor, so the
/// row under test is the only row of amendment a43 that applies.
fn invokable_auditor() -> (tempfile::TempDir, String) {
    let directory = tempfile::tempdir().expect("a directory");
    let program = directory.path().join("auditor");
    std::fs::write(&program, "#!/bin/sh\necho 'one ok'\n").expect("written");
    let mut permissions = std::fs::metadata(&program).expect("there").permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o700);
    std::fs::set_permissions(&program, permissions).expect("executable");
    let program = program.to_str().expect("a UTF-8 scratch path").to_string();
    assert!(
        !program.contains(char::is_whitespace),
        "the prefix splits on white space, so the program path must have none: {program}"
    );
    (directory, format!("{program} observe trace check"))
}

/// What `start` says about an auditor program it cannot start (amendment a43, row 7).
fn not_invokable(detail: String) -> Refusal {
    Refusal::AuditorNotInvokable {
        argv: Vec::new(),
        detail,
    }
}

/// What row 7 says about a path that names nothing, in the platform's own words.
fn cannot_be_found(word: &str) -> String {
    let error = std::fs::metadata(word).expect_err("nothing is there");
    format!("the auditor program {word:?} cannot be found: {error}")
}

/// The three kinds, so a check that held on one kind only fails by kind.
const PAIRING_KINDS: [Kind; 3] = [Kind::Claude, Kind::Codex, Kind::B10x];

/// The three ways a run can ask for an audit or not: none, `--audit`, and `--hermetic strict`.
const AUDIT_REQUESTS: [(bool, HermeticMode); 3] = [
    (false, HermeticMode::Off),
    (true, HermeticMode::Off),
    (false, HermeticMode::Strict),
];

/// A builder of this kind that starts over the scripted runner and reaches nothing real.
///
/// No credential is copied on any kind. Codex is observed through an endpoint nothing listens on,
/// so it needs no frame. b10x keeps its default decisions, which its own start refuses before any
/// program is resolved, so a case that got that far fails by the wrong name and never launches.
fn pairing_builder(kind: Kind) -> Metaharness {
    let builder = Metaharness::new(kind).with_credentials(CredentialSource::None);
    match kind {
        Kind::Codex => builder
            .with_decisions(DecisionMode::Observe)
            .with_model_endpoint("http://127.0.0.1:1")
            .with_model("synthetic-never-called"),
        Kind::Claude | Kind::B10x => builder,
    }
}

/// Start it over the scripted runner: what the start said, and how many children it started.
fn start_counting(builder: Metaharness) -> (Result<(), Refusal>, u32) {
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::new(
        vec![ScriptStep::line(
            r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#,
        )],
        log.clone(),
    );
    let started = builder
        .start_with_clock(
            Input::Prompt("check the pairing".to_string()),
            &mut runner,
            &mut ScriptedSeams,
            Box::new(ManualClock::new()),
        )
        .map(|_run| ());
    (started, log.spawns())
}

/// An auditor answer with one verdict row, so an invocation that should not happen would succeed.
fn one_row() -> AuditorRun {
    AuditorRun {
        exit_code: Some(0),
        stdout: "one ok\n".to_string(),
        stderr: String::new(),
    }
}

#[test]
fn a_spec_without_an_auditor_is_refused_before_the_harness_starts() {
    let document = tempfile::NamedTempFile::new().expect("a document");
    for kind in PAIRING_KINDS {
        for (audit, hermetic) in AUDIT_REQUESTS {
            let (started, spawns) = start_counting(
                pairing_builder(kind)
                    .with_audit(audit)
                    .with_hermetic(hermetic)
                    .with_spec_file(document.path()),
            );
            let case = format!("{kind:?}, --audit {audit}, --hermetic {hermetic:?}");
            assert_eq!(started, Err(Refusal::SpecWithoutAuditor), "{case}");
            assert_eq!(spawns, 0, "{case}: no harness process");
        }
    }
}

#[test]
fn an_auditor_without_a_spec_is_refused_before_the_harness_starts() {
    let (_held, auditor) = invokable_auditor();
    for kind in PAIRING_KINDS {
        for (audit, hermetic) in AUDIT_REQUESTS {
            let (started, spawns) = start_counting(
                pairing_builder(kind)
                    .with_audit(audit)
                    .with_hermetic(hermetic)
                    .with_auditor(auditor.as_str()),
            );
            let case = format!("{kind:?}, --audit {audit}, --hermetic {hermetic:?}");
            assert_eq!(started, Err(Refusal::AuditorWithoutSpec), "{case}");
            assert_eq!(spawns, 0, "{case}: no harness process");
        }
    }
}

#[test]
fn auditor_arguments_without_an_auditor_are_refused_before_the_harness_starts() {
    let document = tempfile::NamedTempFile::new().expect("a document");
    let named = document.path().to_str().expect("a UTF-8 scratch path");
    for kind in PAIRING_KINDS {
        for (audit, hermetic) in AUDIT_REQUESTS {
            let (started, spawns) = start_counting(
                pairing_builder(kind)
                    .with_audit(audit)
                    .with_hermetic(hermetic)
                    .with_auditor_arg("--spec")
                    .with_auditor_arg(named),
            );
            let case = format!("{kind:?}, --audit {audit}, --hermetic {hermetic:?}, -- --spec");
            assert_eq!(started, Err(Refusal::AuditorArgsWithoutAuditor), "{case}");
            assert_eq!(spawns, 0, "{case}: no harness process");
        }
    }
}

#[test]
fn a_spec_and_an_auditor_on_a_run_that_wants_no_audit_are_refused_before_the_harness_starts() {
    let document = tempfile::NamedTempFile::new().expect("a document");
    let (_held, auditor) = invokable_auditor();
    for kind in PAIRING_KINDS {
        for hermetic in [HermeticMode::Off, HermeticMode::On] {
            let (started, spawns) = start_counting(
                pairing_builder(kind)
                    .with_hermetic(hermetic)
                    .with_spec_file(document.path())
                    .with_auditor(auditor.as_str()),
            );
            let case = format!("{kind:?}, --hermetic {hermetic:?}, no --audit");
            match &started {
                Err(Refusal::Launch { detail }) => {
                    assert!(
                        detail.starts_with(
                            "--spec and --auditor were given on a run that wants no audit"
                        ),
                        "{case}: {detail}"
                    );
                    assert!(
                        detail.contains("--audit or --hermetic strict"),
                        "{case}: the refusal says how to ask for one: {detail}"
                    );
                }
                other => panic!("{case}: {other:?}"),
            }
            assert_eq!(spawns, 0, "{case}: no harness process");
        }
    }
}

#[test]
fn an_expectation_document_that_is_not_a_readable_regular_file_is_refused_before_the_harness_starts()
 {
    let scratch = tempfile::tempdir().expect("a directory");
    let missing = scratch.path().join("no-such-expectations.yaml");
    let directory = scratch.path().join("a-directory");
    std::fs::create_dir(&directory).expect("made");
    let mut documents = vec![(missing, None), (directory, Some("not a regular file"))];
    // A file this process may not open. A process that can open it anyway (root can) has nothing
    // to show here, so the case is only listed when the open fails.
    let locked = scratch.path().join("locked.yaml");
    std::fs::write(&locked, "kinds: []\n").expect("written");
    let mut permissions = std::fs::metadata(&locked).expect("there").permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o000);
    std::fs::set_permissions(&locked, permissions).expect("locked");
    if std::fs::File::open(&locked).is_err() {
        documents.push((locked, None));
    }

    let (_held, auditor) = invokable_auditor();
    for kind in PAIRING_KINDS {
        for (document, detail_names) in &documents {
            let (started, spawns) = start_counting(
                pairing_builder(kind)
                    .with_audit(true)
                    .with_spec_file(document)
                    .with_auditor(auditor.as_str()),
            );
            let case = format!("{kind:?}, {}", document.display());
            match &started {
                Err(Refusal::SpecUnreadable { path, detail }) => {
                    assert_eq!(path, document, "{case}: the refusal names the path");
                    if let Some(words) = detail_names {
                        assert!(detail.contains(words), "{case}: {detail}");
                    }
                }
                other => panic!("{case}: {other:?}"),
            }
            assert_eq!(spawns, 0, "{case}: no harness process");
        }
    }
}

#[test]
fn an_expectation_document_whose_path_is_not_utf8_is_refused_naming_its_bytes() {
    let scratch = tempfile::tempdir().expect("a directory");
    let document = scratch.path().join(
        <std::ffi::OsStr as std::os::unix::ffi::OsStrExt>::from_bytes(b"expectations-\xff.yaml"),
    );
    // A readable regular file: the path is the only thing wrong with it.
    std::fs::write(&document, "kinds: []\n").expect("written");
    let bytes = document.as_os_str().as_encoded_bytes().escape_ascii();
    assert!(bytes.to_string().ends_with(r"expectations-\xff.yaml"));
    let refusal = Refusal::SpecUnreadable {
        path: document.clone(),
        detail: format!(
            "its path {bytes} is not UTF-8, and the auditor is handed the path as text, in which \
             it would name another file. Name the document by a path that is UTF-8"
        ),
    };
    let (_held, auditor) = invokable_auditor();
    for kind in PAIRING_KINDS {
        let (started, spawns) = start_counting(
            pairing_builder(kind)
                .with_audit(true)
                .with_spec_file(&document)
                .with_auditor(auditor.as_str()),
        );
        assert_eq!(started, Err(refusal.clone()), "{kind:?}");
        assert_eq!(spawns, 0, "{kind:?}: no harness process");
    }

    let mut invoker = FakeAuditor::answering(one_row());
    let refused = run_auditor(
        Some(&document),
        Some(auditor.as_str()),
        &[],
        std::path::Path::new("t.jsonl"),
        &mut invoker,
    );
    assert_eq!(refused, Err(refusal), "refused at audit time too");
    assert!(invoker.calls().is_empty(), "nothing was invoked");
}

#[test]
fn an_auditor_prefix_with_no_word_is_refused_before_the_harness_starts() {
    let document = tempfile::NamedTempFile::new().expect("a document");
    for kind in PAIRING_KINDS {
        for prefix in ["", "   "] {
            let (started, spawns) = start_counting(
                pairing_builder(kind)
                    .with_audit(true)
                    .with_spec_file(document.path())
                    .with_auditor(prefix),
            );
            let case = format!("{kind:?}, --auditor {prefix:?}");
            assert_eq!(
                started,
                Err(Refusal::AuditorNotInvokable {
                    argv: Vec::new(),
                    detail: "the auditor prefix is empty".to_string(),
                }),
                "{case}"
            );
            assert_eq!(spawns, 0, "{case}: no harness process");
        }
    }
}

#[test]
fn an_auditor_prefix_whose_first_word_is_an_option_is_refused_before_the_harness_starts() {
    let document = tempfile::NamedTempFile::new().expect("a document");
    let prefixes = [
        ("--spec x.yaml", "--spec"),
        ("-v protocol", "-v"),
        ("  -", "-"),
    ];
    for kind in PAIRING_KINDS {
        for (prefix, word) in prefixes {
            let (started, spawns) = start_counting(
                pairing_builder(kind)
                    .with_audit(true)
                    .with_spec_file(document.path())
                    .with_auditor(prefix),
            );
            let case = format!("{kind:?}, --auditor {prefix:?}");
            assert_eq!(
                started,
                Err(not_invokable(format!(
                    "the auditor prefix starts with {word:?}, which reads as an option and names \
                     no program"
                ))),
                "{case}"
            );
            assert_eq!(spawns, 0, "{case}: no harness process");
        }
    }

    for (prefix, word) in prefixes {
        let mut invoker = FakeAuditor::answering(one_row());
        let refused = run_auditor(
            Some(document.path()),
            Some(prefix),
            &[],
            std::path::Path::new("t.jsonl"),
            &mut invoker,
        )
        .expect_err("an option names no program");
        match refused {
            Refusal::AuditorNotInvokable { detail, .. } => assert!(
                detail.starts_with(&format!("the auditor prefix starts with {word:?}")),
                "{prefix:?}: {detail}"
            ),
            other => panic!("{prefix:?}: {other:?}"),
        }
        assert!(
            invoker.calls().is_empty(),
            "{prefix:?}: nothing was invoked"
        );
    }
}

#[test]
fn an_auditor_whose_program_cannot_be_started_is_refused_before_the_harness_starts() {
    let document = tempfile::NamedTempFile::new().expect("a document");
    let scratch = tempfile::tempdir().expect("a directory");
    let base = scratch
        .path()
        .to_str()
        .expect("a UTF-8 scratch path")
        .to_string();
    let nothing = format!("{base}/no-such-auditor");
    let not_executable = format!("{base}/not-executable");
    std::fs::write(&not_executable, "#!/bin/sh\necho 'one ok'\n").expect("written");
    let mut permissions = std::fs::metadata(&not_executable)
        .expect("there")
        .permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o644);
    std::fs::set_permissions(&not_executable, permissions).expect("not executable");
    let relative = "no-such-directory/no-such-auditor";
    let unlisted = "metaharness-pairing-test-no-such-auditor";
    let on_path = std::env::var_os("PATH");
    if let Some(path) = &on_path {
        assert!(
            std::env::split_paths(path).all(|directory| !directory.join(unlisted).exists()),
            "the case needs {unlisted} to be on no PATH directory"
        );
    }
    let unlisted_detail = if on_path.is_some() {
        format!(
            "the auditor program {unlisted:?} is not an executable regular file in any \
             directory of PATH"
        )
    } else {
        format!(
            "the auditor program {unlisted:?} has no '/' in it and PATH is not set, so there is \
             no directory to find it in"
        )
    };
    let cases = [
        (nothing.clone(), cannot_be_found(&nothing)),
        (
            base.clone(),
            format!("the auditor program {base:?} is not a regular file"),
        ),
        (
            not_executable.clone(),
            format!("the auditor program {not_executable:?} has no execute bit"),
        ),
        (relative.to_string(), cannot_be_found(relative)),
        (unlisted.to_string(), unlisted_detail),
    ];
    for kind in PAIRING_KINDS {
        for (word, detail) in &cases {
            let (started, spawns) = start_counting(
                pairing_builder(kind)
                    .with_audit(true)
                    .with_spec_file(document.path())
                    .with_auditor(format!("{word} observe trace check")),
            );
            let case = format!("{kind:?}, --auditor {word:?}");
            assert_eq!(started, Err(not_invokable(detail.clone())), "{case}");
            assert_eq!(spawns, 0, "{case}: no harness process");
        }
    }
}

/// Two rows of amendment a43 that apply to one run, the builder of that run, and whether a refusal
/// is the one the earlier row names.
type TwoRows = (&'static str, Metaharness, fn(&Refusal) -> bool);

#[test]
fn when_two_rows_apply_the_first_in_the_table_is_the_refusal() {
    let document = tempfile::NamedTempFile::new().expect("a document");
    let scratch = tempfile::tempdir().expect("a directory");
    let missing = scratch.path().join("no-such-expectations.yaml");
    let unresolvable = format!("{}/no-such-auditor check", scratch.path().display());
    let (_held, auditor) = invokable_auditor();
    let audited = |builder: Metaharness| builder.with_audit(true);
    let pairs: [TwoRows; 7] = [
        (
            "1 and 3",
            audited(pairing_builder(Kind::Claude))
                .with_spec_file(document.path())
                .with_auditor_arg("--strict"),
            |refused| *refused == Refusal::SpecWithoutAuditor,
        ),
        (
            "1 and 5",
            audited(pairing_builder(Kind::Claude)).with_spec_file(&missing),
            |refused| *refused == Refusal::SpecWithoutAuditor,
        ),
        (
            "2 and 7",
            audited(pairing_builder(Kind::Claude)).with_auditor(unresolvable.as_str()),
            |refused| *refused == Refusal::AuditorWithoutSpec,
        ),
        (
            "4 and 5",
            pairing_builder(Kind::Claude)
                .with_spec_file(&missing)
                .with_auditor(auditor.as_str()),
            |refused| matches!(refused, Refusal::Launch { .. }),
        ),
        (
            "4 and 7",
            pairing_builder(Kind::Claude)
                .with_spec_file(document.path())
                .with_auditor(unresolvable.as_str()),
            |refused| matches!(refused, Refusal::Launch { .. }),
        ),
        (
            "5 and 6",
            audited(pairing_builder(Kind::Claude))
                .with_spec_file(&missing)
                .with_auditor("--spec x.yaml"),
            |refused| matches!(refused, Refusal::SpecUnreadable { .. }),
        ),
        (
            "5 and 7",
            audited(pairing_builder(Kind::Claude))
                .with_spec_file(&missing)
                .with_auditor(unresolvable.as_str()),
            |refused| matches!(refused, Refusal::SpecUnreadable { .. }),
        ),
    ];
    for (rows, builder, expected) in pairs {
        let (started, spawns) = start_counting(builder);
        let refused = started.expect_err(rows);
        assert!(expected(&refused), "rows {rows}: {refused:?}");
        assert_eq!(spawns, 0, "rows {rows}: no harness process");
    }
}

#[test]
fn an_audited_or_strict_run_that_names_both_starts_and_calls_the_auditor_as_before() {
    let document = tempfile::NamedTempFile::new().expect("a document");
    let (_held, prefix) = invokable_auditor();
    let program = prefix.split_whitespace().next().expect("a program");
    for (audit, hermetic) in [(true, HermeticMode::Off), (false, HermeticMode::Strict)] {
        let case = format!("--audit {audit}, --hermetic {hermetic:?}");
        let log = ScriptedLog::new();
        let mut runner = ScriptedRunner::new(
            vec![ScriptStep::line(
                r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#,
            )],
            log.clone(),
        );
        let mut run = pairing_builder(Kind::Claude)
            .with_audit(audit)
            .with_hermetic(hermetic)
            .with_spec_file(document.path())
            .with_auditor(prefix.as_str())
            .start_with_clock(
                Input::Prompt("check the pairing".to_string()),
                &mut runner,
                &mut ScriptedSeams,
                Box::new(ManualClock::new()),
            )
            .unwrap_or_else(|refusal| panic!("{case}: refused: {refusal}"));
        assert_eq!(log.spawns(), 1, "{case}: started once");
        run.drain().expect("drains");
        assert!(run.wants_audit(), "{case}");

        let mut auditor = FakeAuditor::answering(one_row());
        let report = run.audit(&mut auditor).expect("audited");
        let verdict = report.auditor.expect("the auditor ran");
        assert_eq!(verdict.verdict_rows, 1, "{case}");
        assert_eq!(auditor.calls().len(), 1, "{case}");
        let argv = &auditor.calls()[0];
        assert_eq!(argv[..4], [program, "observe", "trace", "check"], "{case}");
        assert_eq!(argv[4], "--spec", "{case}");
        assert_eq!(argv[5], document.path().display().to_string(), "{case}");
    }
}

#[test]
fn a_directory_named_as_the_expectation_document_is_refused_at_audit_time_too() {
    let directory = tempfile::tempdir().expect("a directory");
    let (_held, prefix) = invokable_auditor();
    let mut auditor = FakeAuditor::answering(one_row());
    let refused = run_auditor(
        Some(directory.path()),
        Some(prefix.as_str()),
        &[],
        std::path::Path::new("t.jsonl"),
        &mut auditor,
    )
    .expect_err("a directory is not a document");
    match refused {
        Refusal::SpecUnreadable { path, detail } => {
            assert_eq!(path, directory.path());
            assert!(detail.contains("not a regular file"), "{detail}");
        }
        other => panic!("{other:?}"),
    }
    assert!(auditor.calls().is_empty(), "nothing was invoked");
}

#[test]
fn an_auditor_prefix_with_no_word_is_refused_at_audit_time_without_an_invocation() {
    let document = tempfile::NamedTempFile::new().expect("a document");
    for prefix in ["", "   "] {
        let mut auditor = FakeAuditor::answering(one_row());
        let refused = run_auditor(
            Some(document.path()),
            Some(prefix),
            &[],
            std::path::Path::new("t.jsonl"),
            &mut auditor,
        )
        .expect_err("an empty prefix names no program");
        match refused {
            Refusal::AuditorNotInvokable { detail, .. } => {
                assert_eq!(detail, "the auditor prefix is empty", "{prefix:?}");
            }
            other => panic!("{prefix:?}: {other:?}"),
        }
        assert!(
            auditor.calls().is_empty(),
            "{prefix:?}: nothing was invoked"
        );
    }
}

#[test]
fn auditor_arguments_without_an_auditor_are_refused_at_audit_time_without_an_invocation() {
    let document = tempfile::NamedTempFile::new().expect("a document");
    let named = document.path().to_str().expect("a UTF-8 scratch path");
    let arguments = ["--spec".to_string(), named.to_string()];
    let mut auditor = FakeAuditor::answering(one_row());
    let refused = run_auditor(
        None,
        None,
        &arguments,
        std::path::Path::new("t.jsonl"),
        &mut auditor,
    );
    assert_eq!(refused, Err(Refusal::AuditorArgsWithoutAuditor));
    assert!(auditor.calls().is_empty(), "nothing was invoked");

    let refused = run_auditor(
        Some(document.path()),
        None,
        &arguments,
        std::path::Path::new("t.jsonl"),
        &mut auditor,
    );
    assert_eq!(
        refused,
        Err(Refusal::SpecWithoutAuditor),
        "with a document as well, row 1 matches first"
    );
    assert!(auditor.calls().is_empty(), "nothing was invoked");
}
