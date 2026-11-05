//! Adversary cases for `story:session-close-decides-absent-fields`, pass 1, written 2026-10-21
//! against the working tree over base `4a72d8c`.
//!
//! The unit's grid (`ess_conformance.rs`,
//! `the_reference_model_closes_every_terminal_record_shape_as_production_does`) sends every absent
//! field to ESS as an explicit `null`, while its own doc comment says an absent field is left out of
//! the input, as a vendor record leaves it out. These cases send the absent fields as omitted keys,
//! check that a wrong expectation is never passed (so the grid is not vacuous), and check that the
//! declaration order `positive-terminal-record` now rests on is held by the suite. Expectations
//! come from production's own close over `ScriptedRunner`; nothing here edits the specification.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use metaharness::protocol::{DecisionMode, Event, Kind};
use metaharness::{
    Input, ManualClock, Metaharness, ScriptStep, ScriptedLog, ScriptedRunner, ScriptedSeams,
};
use serde_json::{Value, json};

const BUDGET: &str = "budget-exhausted";
const FIELDS: [&str; 4] = ["is_error", "subtype", "stop_reason", "terminal_reason"];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Production's close over one terminal record, as `completed`, `budget` or `error`.
fn production_reason(input: &Value) -> String {
    let mut record = input.clone();
    record.as_object_mut().unwrap().remove("terminal_present");
    record["emit"] = json!("session.ended");
    let script = if input["terminal_present"] == true {
        vec![ScriptStep::line(record.to_string())]
    } else {
        vec![]
    };
    let mut runner = ScriptedRunner::new(script, ScriptedLog::new());
    let mut run = Metaharness::new(Kind::Claude)
        .with_decisions(DecisionMode::Observe)
        .start_with_clock(
            Input::Prompt("synthetic".to_owned()),
            &mut runner,
            &mut ScriptedSeams,
            Box::new(ManualClock::new()),
        )
        .unwrap();
    run.drain().unwrap();
    let Event::StreamClosed { reason, .. } = run.events().last().unwrap().clone() else {
        panic!("no closure")
    };
    serde_json::to_value(reason)
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned()
}

fn outcome_of(reason: &str) -> &'static str {
    match reason {
        "completed" => "positive-terminal-record",
        "budget" => "observed-budget-stop",
        "error" => "failed-or-unknown-terminal",
        other => panic!("unexpected closure {other}"),
    }
}

/// The same 216 shapes as the unit's grid, built independently here.
fn shapes() -> Vec<(String, Value)> {
    let words = [Some(json!(BUDGET)), Some(json!("other")), None];
    let mut out = Vec::new();
    for terminal_present in [true, false] {
        for is_error in [Some(json!(true)), Some(json!(false)), None] {
            for subtype in [
                Some(json!("success")),
                Some(json!(BUDGET)),
                Some(json!("other")),
                None,
            ] {
                for stop_reason in words.clone() {
                    for terminal_reason in words.clone() {
                        let mut input = json!({"terminal_present": terminal_present});
                        let mut name = format!("adv-{}", u8::from(terminal_present));
                        for (field, value) in
                            FIELDS
                                .iter()
                                .zip([&is_error, &subtype, &stop_reason, &terminal_reason])
                        {
                            name.push('-');
                            name.push_str(match value {
                                None => "n",
                                Some(Value::Bool(true)) => "t",
                                Some(Value::Bool(false)) => "f",
                                Some(Value::String(word)) if word == "success" => "s",
                                Some(Value::String(word)) if word == BUDGET => "b",
                                Some(_) => "o",
                            });
                            if let Some(value) = value {
                                input[*field] = value.clone();
                            }
                        }
                        out.push((name, input));
                    }
                }
            }
        }
    }
    out
}

/// One `ess-scenario/1` whose input carries only the keys `input` carries: an absent field is a
/// missing key, never `null`.
fn scenario(name: &str, input: &Value, reason: &str) -> String {
    let mut lines = vec![
        "type: ess-scenario/1".to_owned(),
        "domain: metaharness.session".to_owned(),
        format!("scenario: {name}"),
        "summary: Adversary shape.".to_owned(),
        "timeline:".to_owned(),
        "  - at: 2026-10-21T00:00:00Z".to_owned(),
        "    command: metaharness.session.CloseUnsteeredStream".to_owned(),
        "    input:".to_owned(),
        format!("      terminal_present: {}", input["terminal_present"]),
    ];
    for field in FIELDS {
        if let Some(value) = input.get(field) {
            lines.push(format!("      {field}: {value}"));
        }
    }
    lines.extend([
        format!("    outcome: {}", outcome_of(reason)),
        "    events:".to_owned(),
        "      - event: metaharness.session.StreamClassified".to_owned(),
        format!("        payload: {{reason: {reason}}}"),
    ]);
    lines.join("\n") + "\n"
}

/// Runs ESS's interpreted model over `spec` with the authored scenarios in `scenarios`.
fn interpreted(spec: &Path, scenarios: &Path) -> (Value, String) {
    let report = scenarios.join("report.json");
    let output = Command::new("ess")
        .args(["verify", "conform", "run", "--strict-requires", "--path"])
        .arg(spec)
        .arg("--scenarios")
        .arg(scenarios)
        .args(["--suite-format", "5", "--target", "interpreted"])
        .args(["--report-format", "2", "--report-out"])
        .arg(&report)
        .output()
        .expect("ESS 0.86.0 is a gate prerequisite");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = fs::read(&report).unwrap_or_else(|_| panic!("no report written:\n{text}"));
    let report = serde_json::from_slice(&bytes).unwrap();
    assert_ran(output.status, &report, &text);
    (report, text)
}

/// The report is of a run ESS made and finished. It was not stopped by a signal, it met no error,
/// the specification's own generated scenarios ran, and the exit status is the one the report's
/// verdict gives. ESS 0.86.0 exits 1 on every run of this specification, because the interpreted
/// target reports ten of its generated scenarios `unsupported`, so the exit is held to the
/// report's verdict rather than to `success()`, which no run here reaches.
fn assert_ran(status: ExitStatus, report: &Value, text: &str) {
    let head = || text.chars().take(2000).collect::<String>();
    assert!(
        status.code().is_some(),
        "ess was stopped by a signal: {status}\n{}",
        head()
    );
    assert_eq!(
        report["counts"]["error"],
        0,
        "ess met an error in the run:\n{}",
        head()
    );
    assert!(
        report["coverage"]["counts"]["generated"]
            .as_u64()
            .is_some_and(|generated| generated > 0),
        "ess ran none of the specification's own scenarios:\n{}",
        head()
    );
    assert_eq!(
        status.success(),
        report["conformance_status"] == "passed",
        "ess exited {status} on a {} verdict:\n{}",
        report["conformance_status"],
        head()
    );
}

fn passed(report: &Value) -> Vec<String> {
    report["outcomes"]["passed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_str().unwrap().to_owned())
        .collect()
}

fn id(name: &str) -> String {
    format!("metaharness.session/authored/{name}")
}

/// The authored scenarios ESS decided against: those it ran and failed, and those it refused
/// because the command's guards decide the input otherwise than the scenario expects
/// (`ESS-AUTHOR-041`). A scenario refused for any other reason was never decided.
fn decided_against(report: &Value) -> Vec<String> {
    let failed = report["outcomes"]["failed"].as_array().unwrap().iter();
    let refused = report["coverage"]["refused"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|refusal| refusal["code"] == "ESS-AUTHOR-041")
        .map(|refusal| &refusal["scenario"]);
    failed
        .chain(refused)
        .map(|id| id.as_str().unwrap().to_owned())
        .collect()
}

/// A vendor record leaves an absent field out; it does not write `null`. The reference model must
/// decide every shape with the absent keys omitted from the input exactly as production closes it.
#[test]
fn every_shape_with_absent_keys_omitted_is_decided_as_production_closes_it() {
    let dir = tempfile::tempdir().unwrap();
    let all = shapes();
    assert_eq!(all.len(), 216);
    for (name, input) in &all {
        let reason = production_reason(input);
        fs::write(
            dir.path().join(format!("{name}.yaml")),
            scenario(name, input, &reason),
        )
        .unwrap();
    }
    let (report, text) = interpreted(&repository_root().join("spec"), dir.path());
    let held = passed(&report);
    let missed: Vec<&String> = all
        .iter()
        .map(|(name, _)| name)
        .filter(|name| !held.contains(&id(name)))
        .collect();
    assert!(
        missed.is_empty(),
        "{} of 216 shapes with omitted keys not passed: {missed:?}\n{}",
        missed.len(),
        text.chars().take(4000).collect::<String>()
    );
}

/// The grid is not vacuous: an expectation production does not produce is never passed. Each shape
/// is sent with the next outcome in rotation and its matching payload.
///
/// The positive control: beside each wrong expectation the same run carries the shape with the
/// expectation production gives, and every one of those must pass, and every wrong one must be
/// decided against rather than dropped. Without them a run that evaluated no authored shape (a
/// scenario ESS refuses before running it, a domain it does not know) passes no wrong expectation
/// either, and the case would hold.
#[test]
fn a_wrong_expectation_is_never_passed_for_any_shape() {
    let dir = tempfile::tempdir().unwrap();
    let all = shapes();
    for (name, input) in &all {
        let reason = production_reason(input);
        let wrong = match reason.as_str() {
            "completed" => "budget",
            "budget" => "error",
            _ => "completed",
        };
        fs::write(
            dir.path().join(format!("{name}.yaml")),
            scenario(name, input, wrong),
        )
        .unwrap();
        let control = format!("{name}-control");
        fs::write(
            dir.path().join(format!("{control}.yaml")),
            scenario(&control, input, &reason),
        )
        .unwrap();
    }
    let (report, text) = interpreted(&repository_root().join("spec"), dir.path());
    let held = passed(&report);
    let wrongly: Vec<&String> = all
        .iter()
        .map(|(name, _)| name)
        .filter(|name| held.contains(&id(name)))
        .collect();
    assert!(
        wrongly.is_empty(),
        "{} wrong expectations passed: {wrongly:?}",
        wrongly.len()
    );
    let unheld: Vec<String> = all
        .iter()
        .map(|(name, _)| format!("{name}-control"))
        .filter(|control| !held.contains(&id(control)))
        .collect();
    assert!(
        unheld.is_empty(),
        "{} of 216 right expectations not passed in the run that passed no wrong one: {unheld:?}\n{}",
        unheld.len(),
        text.chars().take(4000).collect::<String>()
    );
    let decided = decided_against(&report);
    let undecided: Vec<&String> = all
        .iter()
        .map(|(name, _)| name)
        .filter(|name| !decided.contains(&id(name)))
        .collect();
    assert!(
        undecided.is_empty(),
        "{} of 216 wrong expectations neither failed nor refused by the guards: {undecided:?}",
        undecided.len()
    );
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// With the `none:` budget clause gone, `observed-budget-stop` and `positive-terminal-record` both
/// hold on `is_error: false` beside a budget word, and only declaration order answers `budget`.
/// Exchanging the two branches in a copy of the specification must change the reference model's
/// answer for that shape, and the repository's own authored scenarios must catch it.
#[test]
fn exchanging_the_two_accepting_branches_is_caught() {
    let copy = tempfile::tempdir().unwrap();
    let spec = copy.path().join("spec");
    copy_tree(&repository_root().join("spec"), &spec);
    let session = spec.join("domains/session.yaml");
    let text = fs::read_to_string(&session).unwrap();
    let budget_start = text.find("      - name: observed-budget-stop\n").unwrap();
    let positive_start = text
        .find("      - name: positive-terminal-record\n")
        .unwrap();
    let failed_start = text
        .find("      - name: failed-or-unknown-terminal\n")
        .unwrap();
    assert!(budget_start < positive_start && positive_start < failed_start);
    let exchanged = format!(
        "{}{}{}{}",
        &text[..budget_start],
        &text[positive_start..failed_start],
        &text[budget_start..positive_start],
        &text[failed_start..]
    );
    fs::write(&session, exchanged).unwrap();

    let dir = tempfile::tempdir().unwrap();
    let shape = json!({"terminal_present": true, "is_error": false, "subtype": "other", "stop_reason": BUDGET});
    assert_eq!(production_reason(&shape), "budget");
    fs::write(
        dir.path().join("exchanged.yaml"),
        scenario("exchanged", &shape, "budget"),
    )
    .unwrap();
    let (report, text) = interpreted(&spec, dir.path());
    assert!(
        !passed(&report).contains(&id("exchanged")),
        "the exchanged specification still answers budget:\n{}",
        text.chars().take(2000).collect::<String>()
    );
    let (unchanged, _) = interpreted(&repository_root().join("spec"), dir.path());
    assert!(passed(&unchanged).contains(&id("exchanged")));
}
