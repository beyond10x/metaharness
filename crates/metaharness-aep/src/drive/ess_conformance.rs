//! Bounded ESS suite executed against production seams; unsupported steps fail closed.
use super::*;
use metaharness::protocol::Event;
use serde_json::{Value, json};

fn suite() -> Value {
    serde_json::from_str(include_str!("../../../../spec/conformance.json")).unwrap()
}

#[derive(Default)]
struct Observed {
    outcome: String,
    error: Option<String>,
    events: std::collections::BTreeMap<String, Value>,
    retained: Option<(tempfile::TempDir, String, String)>,
}

fn event(outcome: &str, name: &str, payload: Value) -> Observed {
    Observed {
        outcome: outcome.to_owned(),
        events: [(name.to_owned(), payload)].into(),
        ..Observed::default()
    }
}

fn checked(ok: bool, accepted: &str, refused: &str, error: &str) -> Observed {
    Observed {
        outcome: if ok { accepted } else { refused }.to_owned(),
        error: (!ok).then(|| error.to_owned()),
        ..Observed::default()
    }
}

fn process_write_plan(input: &Value) -> Observed {
    use metaharness::protocol::{CredentialSource, DecisionMode, Kind};
    use metaharness::{Input, Metaharness, ScriptedLog, ScriptedRunner};
    use metaharness_b10x::B10xSeams;
    let root = tempfile::tempdir().unwrap();
    let cwd = root.path().join("work");
    fs::create_dir(&cwd).unwrap();
    let mut builder = Metaharness::new(Kind::B10x)
        .with_credentials(CredentialSource::None)
        .with_decisions(DecisionMode::Observe)
        .with_model("ess-fixture")
        .with_model_endpoint("http://fixture.invalid")
        .with_cwd(cwd)
        .with_substrate_embedded(true);
    let probe = input["probe"].as_str().unwrap();
    builder = match probe {
        "empty" => builder,
        "explicit" => builder
            .with_process_write_subtree("target")
            .with_process_write_subtree("generated"),
        "invalid" => builder.with_process_write_subtree("../outside"),
        "file-scope-only" => builder.with_write_scope("**=allowed"),
        other => panic!("unsupported process write probe {other}"),
    };
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::of_lines(Vec::<String>::new(), log.clone());
    let result = builder.start_with(
        Input::Prompt("fixture".to_owned()),
        &mut runner,
        &mut B10xSeams::new(None, None, None),
    );
    let admitted = result.is_ok();
    let launched = log.launched();
    let paths: Vec<&str> = launched
        .iter()
        .flat_map(|argv| argv.windows(2))
        .filter(|pair| pair[0] == "--process-write-subtree")
        .map(|pair| pair[1].as_str())
        .collect();
    let count = paths.len();
    let outcome = if !admitted {
        assert_eq!(log.spawns(), 0);
        "process-invalid-directory"
    } else if count == 2 {
        assert_eq!(paths, ["target", "generated"]);
        "process-explicit-subtrees"
    } else {
        assert_eq!(
            count, 0,
            "an unexpected process grant must fail conformance"
        );
        if launched
            .iter()
            .any(|argv| argv.iter().any(|arg| arg == "--write-scope"))
        {
            "file-scope-is-not-process-scope"
        } else {
            "process-read-only-default"
        }
    };
    event(
        outcome,
        "metaharness.workspace.ProcessWritePlanned",
        json!({"admitted":admitted,"count":count}),
    )
}

fn completion(input: &Value) -> Observed {
    let record = metaharness_codex::conformance::completion(
        input["error_property"].as_str().unwrap(),
        input["last_message"].as_str().unwrap(),
    )
    .unwrap();
    let Event::SessionEnded {
        is_error,
        total_cost_usd,
        ..
    } = &record
    else {
        panic!("no terminal observation")
    };
    assert_eq!(*total_cost_usd, None, "unknown evidence is never zero cost");
    let (outcome, verdict) = match is_error {
        Some(true) => ("explicit-failure", "failure"),
        Some(false) => ("positive-completion", "success"),
        None => ("incomplete-evidence", "unknown"),
    };
    event(
        outcome,
        "metaharness.session.CompletionClassified",
        json!({"verdict":verdict}),
    )
}

fn closed_stream(input: &Value) -> Event {
    use metaharness::{
        Input, ManualClock, Metaharness, ScriptStep, ScriptedLog, ScriptedRunner, ScriptedSeams,
    };
    let mut terminal = input.clone();
    terminal.as_object_mut().unwrap().remove("terminal_present");
    terminal["emit"] = json!("session.ended");
    let script = if input["terminal_present"] == true {
        vec![ScriptStep::line(terminal.to_string())]
    } else {
        vec![]
    };
    let mut runner = ScriptedRunner::new(script, ScriptedLog::new());
    let mut run = Metaharness::new(metaharness::protocol::Kind::Claude)
        .with_decisions(metaharness::protocol::DecisionMode::Observe)
        .start_with_clock(
            Input::Prompt("synthetic".to_owned()),
            &mut runner,
            &mut ScriptedSeams,
            Box::new(ManualClock::new()),
        )
        .unwrap();
    run.drain().unwrap();
    let Event::StreamClosed { events, .. } = run.events().last().unwrap() else {
        panic!("no closure")
    };
    assert_eq!(*events, u64::try_from(run.events().len() - 1).unwrap());
    run.events().last().unwrap().clone()
}

fn closure(input: &Value) -> Observed {
    let Event::StreamClosed { reason, .. } = closed_stream(input) else {
        unreachable!()
    };
    let reason = serde_json::to_value(reason).unwrap();
    let outcome = match reason.as_str().unwrap() {
        "completed" => "positive-terminal-record",
        "budget" => "observed-budget-stop",
        "error" => "failed-or-unknown-terminal",
        other => panic!("unexpected closure {other}"),
    };
    event(
        outcome,
        "metaharness.session.StreamClassified",
        json!({"reason":reason}),
    )
}

fn frame(input: &Value) -> Observed {
    let tools = tests::config(&[]);
    let state = "implement".parse().unwrap();
    let task = tests::driven_task();
    let context = tests::step_context(&tools, &state, &task);
    let mut frame = metaharness_frame(&context, &[], "test/linear", "1");
    match input["evidence"].as_str().unwrap() {
        "sealed" => (),
        "modified" => frame["node"]["id"] = json!("changed"),
        "untagged" => {
            frame.as_object_mut().unwrap().remove("format");
        }
        other => panic!("unknown frame evidence {other}"),
    }
    let admitted =
        metaharness::protocol::Frame::parse_document(&frame_document(&frame).unwrap()).is_ok();
    event(
        if admitted { "admitted" } else { "refused" },
        "metaharness.boundaries.FrameRead",
        json!({"admitted":admitted}),
    )
}

fn governed(input: &Value) -> Observed {
    use aep_engine::ProtocolEngine as _;
    let profile = if input["engine_allows"] == true {
        tests::AUTHORIZE_PROFILE.replace(
            "allow: [repository.read]",
            "allow: [repository.read, command.execute]",
        )
    } else {
        tests::AUTHORIZE_PROFILE.to_owned()
    };
    let (engine, mut execution) = tests::authorizing_execution_with_profile(&profile);
    let tools = tests::config(&[Capability::RepositoryRead, Capability::CommandExecution]);
    let state = "implement".parse().unwrap();
    let task = tests::driven_task();
    let context = tests::step_context(&tools, &state, &task);
    let (name, payload) =
        metaharness_codex::conformance::governed_request(input["shape"].as_str().unwrap()).unwrap();
    let line = json!({"event":"tool.requested","decision_required":true,"call_id":"ess-call","name":name,"input":payload}).to_string()+"\n";
    let mut commands = Vec::new();
    answer_events(
        Harness::Codex,
        &context,
        WriteSurface {
            scope: &[],
            root: Path::new("."),
        },
        line.as_bytes(),
        &mut commands,
        &mut Vec::new(),
        &mut |request| engine.authorize(&mut execution, request),
    );
    let response: Value = serde_json::from_slice(&commands).unwrap();
    let allowed = response["decision"]["decision"] == "allow";
    event(
        if allowed { "allowed" } else { "denied" },
        "metaharness.boundaries.CallDecided",
        json!({"allowed":allowed}),
    )
}

fn contained_read(input: &Value) -> Observed {
    use metaharness_tools::{Catalogue, LocalOperations, Server, Verbs};
    let directory = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("inside"), "inside-marker").unwrap();
    fs::write(outside.path().join("secret"), "outside-marker").unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("secret"),
        directory.path().join("outside-symlink"),
    )
    .unwrap();
    let mut server = Server::new(Verbs::new(Catalogue::of(
        LocalOperations::new(directory.path()).unwrap(),
    )));
    let response = server
        .handle(
            &json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
        "name":"tool_invoke","arguments":{"name":"file_read","arguments":{"path":input["path"]}}}}),
        )
        .unwrap();
    let contained = response["result"]["isError"] != true && response["error"].is_null();
    if contained {
        assert!(response.to_string().contains("inside-marker"));
    } else {
        assert!(
            response.to_string().contains("outside the workspace"),
            "{response}"
        );
    }
    event(
        if contained { "read" } else { "refused" },
        "metaharness.boundaries.FileRead",
        json!({"contained":contained}),
    )
}

fn retained(command: &str) -> Observed {
    let directory = tempfile::tempdir().unwrap();
    let (event_name, view_name, field) = match command {
        "metaharness.session.RecordClosedStream" => {
            let record = closed_stream(&json!({"terminal_present":false}));
            fs::write(
                directory.path().join(SPEND_FILE),
                serde_json::to_vec(&record).unwrap(),
            )
            .unwrap();
            (
                "metaharness.session.ClosureRecorded",
                "metaharness.session.RetainedClosure",
                "closure",
            )
        }
        "metaharness.spending.RecordFiniteLedger" => {
            SpendBudget::start(
                directory.path(),
                SpendTerms {
                    cap_micro_usd: 10,
                    assumed_micro_usd_per_run: 1,
                },
            )
            .unwrap();
            (
                "metaharness.spending.FiniteLedgerRecorded",
                "metaharness.spending.RetainedFiniteLedger",
                "ledger",
            )
        }
        "metaharness.spending.RecordInvocationAdmission" => {
            let mut budget = AdmissionBudget::open(
                directory.path(),
                spending::SpendPolicy::Uncapped {
                    authorization_ref: "conformance:no-model".to_owned(),
                },
                false,
            )
            .unwrap();
            let tools = tests::config(&[]);
            let state = "implement".parse().unwrap();
            let task = tests::driven_task();
            budget
                .reserve(&tests::step_context(&tools, &state, &task))
                .unwrap();
            (
                "metaharness.spending.AdmissionRecorded",
                "metaharness.spending.RetainedAdmission",
                "admission",
            )
        }
        other => panic!("unsupported retained operation {other}"),
    };
    let mut observation = event(
        "recorded",
        event_name,
        json!({"record_id":directory.path().display().to_string()}),
    );
    observation.retained = Some((directory, view_name.to_owned(), field.to_owned()));
    observation
}

fn query(observed: &Observed, view: &str) -> Result<Value, String> {
    if ![
        "metaharness.session.RetainedClosure",
        "metaharness.spending.RetainedAdmission",
        "metaharness.spending.RetainedFiniteLedger",
    ]
    .contains(&view)
    {
        return Err(format!("unsupported view {view}"));
    }
    let Some((directory, expected, field)) = observed.retained.as_ref() else {
        return Ok(json!([]));
    };
    if view != expected {
        return Ok(json!([]));
    }
    let record: Value = serde_json::from_slice(
        &fs::read(directory.path().join(SPEND_FILE)).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let projected = if field == "admission" {
        json!({"launches":record["invocations"].as_array().ok_or("missing actual admissions")?.len(),"authorization_ref":record["policy"]["authorization_ref"]})
    } else {
        record
    };
    Ok(json!({field:projected}))
}

fn observed_codex(command: &str, input: &Value) -> Observed {
    let events =
        metaharness_codex::conformance::observation(input["evidence"].as_str().unwrap()).unwrap();
    if command.ends_with("ReadToolOutcome") {
        let (error, code) = events
            .iter()
            .rev()
            .find_map(|event| match event {
                Event::ToolResult {
                    is_error,
                    exit_code,
                    ..
                } => Some((*is_error, *exit_code)),
                _ => None,
            })
            .unwrap();
        let known = error.is_some();
        let failed = error == Some(true);
        let numeric_exit = code.is_some();
        let outcome = match (known, failed, numeric_exit) {
            (false, _, _) => "unverified",
            (true, false, true) => "command-success",
            (true, true, true) => "command-failure",
            (true, false, false) => "success-without-code",
            (true, true, false) => "failure-without-code",
        };
        event(
            outcome,
            "metaharness.observations.ToolOutcomeRead",
            json!({"known":known,"failed":failed,"numeric_exit":numeric_exit}),
        )
    } else {
        let Event::SessionEnded {
            final_answer,
            observed_models,
            ..
        } = events.last().unwrap()
        else {
            panic!("missing terminal")
        };
        let (present, yes, no) = if command.ends_with("ReadFinalAnswer") {
            (final_answer.is_some(), "authoritative", "unavailable")
        } else {
            (
                observed_models
                    .as_ref()
                    .is_some_and(|models| models.iter().any(|model| model.model.is_some())),
                "observed-selection",
                "unreported",
            )
        };
        event(
            if present { yes } else { no },
            "metaharness.observations.PresenceRead",
            json!({"present":present}),
        )
    }
}

fn execute(command: &str, input: &Value) -> Observed {
    match command {
        "metaharness.observations.ReadFinalAnswer"
        | "metaharness.observations.ReadObservedModel"
        | "metaharness.observations.ReadToolOutcome" => observed_codex(command, input),
        "metaharness.observations.ReadNativeTermination" => {
            use metaharness::protocol::ProcessTermination;
            let mut closure = json!({"event":"stream.closed","events":0,"run_id":"ess-native","reason":"completed"});
            match input["evidence"].as_str().unwrap() {
                "missing" => (),
                "exited-zero" => closure["process"] = json!({"kind":"exited","code":0}),
                "exited-nonzero" => closure["process"] = json!({"kind":"exited","code":7}),
                "signaled" => closure["process"] = json!({"kind":"signaled","signal":9}),
                other => panic!("unsupported native evidence {other}"),
            }
            let Event::StreamClosed { process, .. } = serde_json::from_value(closure).unwrap()
            else {
                unreachable!()
            };
            let (outcome, known, native_success) = match process {
                ProcessTermination::Unknown => ("unobserved", false, false),
                ProcessTermination::Exited { code: 0 } => ("measured-success", true, true),
                _ => ("measured-failure", true, false),
            };
            event(
                outcome,
                "metaharness.observations.NativeStatusClassified",
                json!({"known":known,"native_success":native_success}),
            )
        }
        "metaharness.session.RecordClosedStream"
        | "metaharness.spending.RecordFiniteLedger"
        | "metaharness.spending.RecordInvocationAdmission" => retained(command),
        "metaharness.session.FinishCodexCompletion" => completion(input),
        "metaharness.session.CloseUnsteeredStream" => closure(input),
        "metaharness.workspace.DeclareProcessWrites" => process_write_plan(input),
        "metaharness.workspace.CheckSelectedName" => checked(
            metaharness_b10x::workspace_component_is_adoptable(input["name"].as_str().unwrap()),
            "eligible-component",
            "invalid-name-refused",
            "metaharness.workspace.InvalidName",
        ),
        "metaharness.spending.CheckFiniteTerms" => {
            let usd = |value: &Value| {
                let encoded = value.to_string();
                // ESS's integer witnesses may be encoded as 1.0. Accept only an exact
                // integral spelling in u64 range; never round a floating-point amount.
                encoded
                    .strip_suffix(".0")
                    .unwrap_or(&encoded)
                    .parse::<u64>()
                    .ok()
                    .map_or_else(
                        || "-1".to_owned(),
                        |value| format!("{}.{:06}", value / 1_000_000, value % 1_000_000),
                    )
            };
            let terms = &input["terms"];
            checked(
                spend_terms_with_live(
                    &tests::b10x_map(),
                    Some(&usd(&terms["cap_micro_usd"])),
                    Some(&usd(&terms["assumed_micro_usd_per_run"])),
                    true,
                )
                .is_ok(),
                "valid-finite-terms",
                "invalid-finite-terms",
                "metaharness.spending.InvalidFiniteTerms",
            )
        }
        "metaharness.spending.CheckUncappedAuthorization" => {
            let reference = match input["reference_evidence"].as_str().unwrap() {
                "missing" => None,
                "blank" => Some(" \t".to_owned()),
                "nonblank" => Some("operator:ess".to_owned()),
                other => panic!("unsupported reference shape {other}"),
            };
            let options = SpendOptions {
                uncapped_budget: input["explicit_opt_in"].as_bool().unwrap(),
                spend_authorization: reference,
            };
            checked(
                spending::policy(&tests::b10x_map(), None, None, None, &options, true).is_ok(),
                "explicitly-authorized",
                "unauthorized",
                "metaharness.spending.UncappedAuthorizationRequired",
            )
        }
        "metaharness.boundaries.ReadSealedFrame" => frame(input),
        "metaharness.boundaries.GovernCodexCall" => governed(input),
        "metaharness.boundaries.ReadSelectedWorkspaceFile" => contained_read(input),
        other => panic!("no production target for {other}; cannot silently skip"),
    }
}

fn event_matches(actual: Option<&Value>, step: &Value) -> Result<bool, String> {
    let Some(actual) = actual else {
        return Ok(false);
    };
    if let Some(payload) = step.get("payload")
        && actual != payload
    {
        return Ok(false);
    }
    for (field, shape) in step["shape"]
        .as_object()
        .ok_or("event has no declared shape")?
    {
        let matches = match shape["holds"].as_str() {
            Some("enum") => shape["variants"]
                .as_array()
                .ok_or("enum has no variants")?
                .contains(&actual[field]),
            Some("primitive") => match shape["kind"].as_str() {
                Some("string") => actual[field].is_string(),
                Some("boolean") => actual[field].is_boolean(),
                other => return Err(format!("unsupported primitive {other:?}")),
            },
            other => return Err(format!("unsupported event shape {other:?}")),
        };
        if !matches {
            return Ok(false);
        }
    }
    Ok(true)
}

fn view_matches(actual: &Value, expectation: &Value) -> Result<bool, String> {
    match expectation["expect"].as_str() {
        Some("contains") => Ok(actual.as_object().is_some_and(|row| !row.is_empty())
            && expectation["fields"]
                .as_object()
                .ok_or("missing expected fields")?
                .iter()
                .all(|(key, value)| actual[key] == *value)),
        Some("satisfies") => {
            let words: Vec<_> = expectation["predicate"]
                .as_str()
                .ok_or("missing predicate")?
                .split_whitespace()
                .collect();
            if words.len() != 3 {
                return Err("unsupported predicate grammar".to_owned());
            }
            let mut value = actual;
            for field in words[0].split('.') {
                value = &value[field];
            }
            let left = value
                .as_i64()
                .ok_or("invariant field is absent or not a representable integer")?;
            let right = words[2].parse::<i64>().map_err(|e| e.to_string())?;
            match words[1] {
                ">=" => Ok(left >= right),
                ">" => Ok(left > right),
                other => Err(format!("unsupported invariant operator {other}")),
            }
        }
        other => Err(format!("unsupported view expectation {other:?}")),
    }
}

fn run_scenario(scenario: &Value) -> Result<(), String> {
    let mut actual = Observed::default();
    let mut snapshots = std::collections::BTreeMap::new();
    for step in scenario["steps"]
        .as_array()
        .ok_or("scenario has no steps")?
    {
        let agrees = match step["step"].as_str().ok_or("step has no kind")? {
            "execute_command" => {
                let mut input = serde_json::Map::new();
                let empty = serde_json::Map::new();
                let fields = step.get("input").map_or(Ok(&empty), |value| {
                    value.as_object().ok_or("input is not an object")
                })?;
                for (key, value) in fields {
                    if value["kind"] != "literal" {
                        return Err(format!("unsupported input binding: {value}"));
                    }
                    input.insert(key.clone(), value["value"].clone());
                }
                let mut next = execute(
                    step["command"].as_str().ok_or("missing command")?,
                    &Value::Object(input),
                );
                if next.retained.is_none() {
                    next.retained = actual.retained.take();
                }
                actual = next;
                true
            }
            "expect_outcome" => actual.outcome == step["outcome"]["outcome"],
            "expect_event" => {
                event_matches(actual.events.get(step["event"].as_str().unwrap()), step)?
            }
            "query_view" => {
                query(&actual, step["view"].as_str().ok_or("missing view")?)?;
                true
            }
            "snapshot_view" => {
                let view = step["view"].as_str().ok_or("missing view")?;
                snapshots.insert(view.to_owned(), query(&actual, view)?);
                true
            }
            "expect_view_unchanged" => {
                let view = step["view"].as_str().ok_or("missing view")?;
                snapshots.get(view).ok_or("view was not snapshotted")? == &query(&actual, view)?
            }
            "expect_view" => view_matches(
                &query(&actual, step["view"].as_str().ok_or("missing view")?)?,
                &step["expectation"],
            )?,
            "expect_no_event" => !actual.events.contains_key(step["event"].as_str().unwrap()),
            "expect_no_events" => actual.events.is_empty(),
            "expect_error" => actual.error.as_deref() == step["error"].as_str(),
            "expect_no_error" => actual.error.is_none(),
            other => {
                return Err(format!(
                    "unsupported expectation {other}; cannot silently skip"
                ));
            }
        };
        if !agrees {
            return Err(format!(
                "unmet {step}; observed outcome {}, error {:?}, events {:?}",
                actual.outcome, actual.error, actual.events
            ));
        }
    }
    Ok(())
}

#[test]
fn ess_generated_scenarios_drive_production_targets() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let external = std::env::var_os("METAHARNESS_ESS_SUITE").map(PathBuf::from);
    let source = external
        .clone()
        .unwrap_or_else(|| root.join("spec/conformance.json"));
    let suite: Value = serde_json::from_slice(&fs::read(&source).unwrap()).unwrap();
    assert!(
        ["ess-conformance/22", "ess-conformance/23"]
            .iter()
            .any(|version| suite["provenance"]["suite_version"] == *version),
        "unsupported suite version"
    );
    let scenarios = suite["scenarios"].as_object().unwrap();
    if external.is_none() {
        assert_eq!(
            scenarios.len(),
            40,
            "review and update coverage deliberately"
        );
        assert_eq!(
            suite["coverage"]["counts"]["refused"], 0,
            "every invariant now has an observable production record"
        );
        for command in [
            "FinishCodexCompletion",
            "CloseUnsteeredStream",
            "CheckSelectedName",
            "DeclareProcessWrites",
            "CheckFiniteTerms",
            "CheckUncappedAuthorization",
            "ReadSealedFrame",
            "GovernCodexCall",
            "ReadSelectedWorkspaceFile",
            "ReadNativeTermination",
            "ReadFinalAnswer",
            "ReadObservedModel",
            "ReadToolOutcome",
        ] {
            assert!(
                scenarios.keys().any(|id| id.contains(command)),
                "missing required obligation {command}"
            );
        }
    }
    let mut failed = Vec::new();
    let mut results = Vec::new();
    for (id, scenario) in scenarios {
        let status = match run_scenario(scenario) {
            Ok(()) => "passed",
            Err(reason) => {
                failed.push(format!("{id}: {reason}"));
                "failed"
            }
        };
        results.push(json!({"scenario_id":id,"status":status}));
    }
    let report = json!({"format":"metaharness.ess-conformance/1","claim":"selected executable boundary projection; not full-system or current-vendor qualification",
        "provenance":suite["provenance"],"total":scenarios.len(),"passed":scenarios.len()-failed.len(),"failed":failed.len(),"skipped":0,
        "excluded_invariant_obligations":suite["coverage"]["counts"]["refused"],"failures":failed});
    fs::create_dir_all(root.join("target")).unwrap();
    fs::write(
        root.join("target/metaharness-ess-report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{report}");
    let output = std::env::var_os("METAHARNESS_ESS_REPORT").map_or_else(
        || root.join("target/ess-conformance-report.json"),
        PathBuf::from,
    );
    official_report(&source, &output, &results);
    if external.is_none() {
        let official: Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
        assert_eq!(official["conformance_status"], "passed", "{official}");
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

fn official_report(source: &Path, output: &Path, results: &[Value]) {
    let completed_at = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    let input = output.with_extension("results.json");
    fs::write(&input,serde_json::to_vec_pretty(&json!({"format":"ess-conformance-results/1","completed_at":completed_at,"results":results})).unwrap()).unwrap();
    let result = Process::new("ess")
        .args(["verify", "conform", "report", "--suite"])
        .arg(source)
        .arg("--results")
        .arg(input)
        .args([
            "--implementation",
            "metaharness-issue-repair",
            "--runner",
            "metaharness-rust-target@1",
            "--report-out",
        ])
        .arg(output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn ess_suite_matches_the_current_specification() {
    let scratch = tempfile::tempdir().unwrap();
    let output = scratch.path().join("suite.json");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let status = Process::new("ess")
        .args(["verify", "conform", "synthesize", "--path"])
        .arg(root.join("spec"))
        .args(["--suite-format", "5", "--strict-requires", "--out"])
        .arg(&output)
        .output()
        .expect("ESS 0.51.0 is a gate prerequisite");
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let fresh: Value = serde_json::from_slice(&fs::read(output).unwrap()).unwrap();
    assert_eq!(
        fresh,
        suite(),
        "regenerate spec/conformance.json with ESS 0.51.0"
    );
}

#[test]
fn ess_specification_is_part_of_the_repository_gate() {
    let taskfile = include_str!("../../../../Taskfile.yml");
    assert!(taskfile.contains("ess specify validate --path spec"));
}
