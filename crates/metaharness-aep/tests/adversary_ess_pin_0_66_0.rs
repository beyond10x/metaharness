//! Adversary cases for `story:ess-pin-0-66-0`, written 2026-10-20 against the working tree over
//! base `8c553e1`.
//!
//! The specification's own reference model (`ess verify conform run --target interpreted`) is the
//! second program here: it executes `spec/` itself, so a scenario it cannot decide is one the
//! specification does not state, whatever production does. Expected outcomes are literals taken
//! from `spec/domains/session.yaml` and from the authored scenarios, never from the code under test.
//! Nothing here edits the target, the specification, a scenario or the suite.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use metaharness::protocol::{DecisionMode, Event, Kind};
use metaharness::{
    Input, ManualClock, Metaharness, ScriptStep, ScriptedLog, ScriptedRunner, ScriptedSeams,
};
use serde_json::{Value, json};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Runs ESS's reference model over `spec/` and returns its `ess-conformance-report/2` plus the text
/// it printed. `extra` selects either the committed suite or authored scenarios beside a fresh one.
fn reference_model(extra: &[&std::ffi::OsStr]) -> (Value, String) {
    let scratch = tempfile::tempdir().unwrap();
    let report = scratch.path().join("report.json");
    let output = Command::new("ess")
        .args(["verify", "conform", "run", "--strict-requires", "--path"])
        .arg(repository_root().join("spec"))
        .args(extra)
        .args([
            "--target",
            "interpreted",
            "--report-format",
            "2",
            "--report-out",
        ])
        .arg(&report)
        .output()
        .expect("ESS 0.67.0 is a gate prerequisite");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = fs::read(&report).unwrap_or_else(|_| panic!("no report written:\n{text}"));
    (serde_json::from_slice(&bytes).unwrap(), text)
}

fn status_of<'a>(report: &'a Value, id: &str) -> &'a str {
    let outcomes = report["outcomes"].as_object().unwrap();
    outcomes
        .iter()
        .find(|(_, ids)| ids.as_array().unwrap().iter().any(|held| held == id))
        .map_or("absent", |(status, _)| status.as_str())
}

fn reason_line(text: &str, id: &str) -> String {
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if line.trim_end().ends_with(id) {
            return lines
                .next()
                .unwrap_or("")
                .trim()
                .chars()
                .take(220)
                .collect();
        }
    }
    String::new()
}

/// The unit's new contract: every authored scenario `spec/ess-inputs.yaml` selects asserts an
/// outcome the specification's guards decide. A scenario the reference model reports unsupported
/// asserts behaviour no guard in `spec/` states, so the suite carries a rule the specification
/// does not.
#[test]
fn every_authored_scenario_is_decided_by_the_specifications_reference_model() {
    let suite = repository_root().join("spec/conformance.json");
    let committed: Value = serde_json::from_slice(&fs::read(&suite).unwrap()).unwrap();
    let authored: Vec<String> = committed["coverage"]["authored"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        authored.len(),
        4,
        "spec/ess-inputs.yaml selects four authored scenarios"
    );
    let (report, text) = reference_model(&["--suite".as_ref(), suite.as_os_str()]);
    let undecided: Vec<String> = authored
        .iter()
        .filter(|id| status_of(&report, id) != "passed")
        .map(|id| {
            format!(
                "{id}: {} ({})",
                status_of(&report, id),
                reason_line(&text, id)
            )
        })
        .collect();
    assert!(
        undecided.is_empty(),
        "authored scenarios the specification does not decide:\n{}",
        undecided.join("\n")
    );
}

/// The terminal record the repository's own vectors and the scripted runner's documented example
/// write for a successful run: `is_error` false, subtype `success`, no `stop_reason` and no
/// `terminal_reason` (`crates/metaharness/src/vectors.rs:33`, `builder.rs:2013`,
/// `scripted.rs:358`).
const SUCCESS_RECORD: &str = r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#;

/// Production closes that record `completed` (`session.yaml`, `positive-terminal-record` payload).
/// Pre-existing when written: the specification did not decide it, because an absent
/// `stop_reason`/`terminal_reason` left the budget comparisons unknown. The acceptance case of
/// `story:session-close-decides-absent-fields`, whose guards say what an absent field compares as.
#[test]
fn the_specification_decides_the_success_record_the_repository_writes() {
    let mut runner = ScriptedRunner::new(
        vec![ScriptStep::line(SUCCESS_RECORD.to_owned())],
        ScriptedLog::new(),
    );
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
    assert_eq!(serde_json::to_value(reason).unwrap(), json!("completed"));

    let scenarios = tempfile::tempdir().unwrap();
    fs::write(
        scenarios.path().join("adversary-success-record.yaml"),
        "type: ess-scenario/1\n\
         domain: metaharness.session\n\
         scenario: adversary-success-record\n\
         summary: The success record the repository's vectors write closes the stream as completed.\n\
         \n\
         timeline:\n\
         \x20 - at: 2026-10-20T00:00:00Z\n\
         \x20   command: metaharness.session.CloseUnsteeredStream\n\
         \x20   input:\n\
         \x20     terminal_present: true\n\
         \x20     is_error: false\n\
         \x20     subtype: success\n\
         \x20     stop_reason: null\n\
         \x20     terminal_reason: null\n\
         \x20   outcome: positive-terminal-record\n\
         \x20   events:\n\
         \x20     - event: metaharness.session.StreamClassified\n\
         \x20       payload: {reason: completed}\n",
    )
    .unwrap();
    let id = "metaharness.session/authored/adversary-success-record";
    let (report, text) = reference_model(&[
        "--scenarios".as_ref(),
        scenarios.path().as_os_str(),
        "--suite-format".as_ref(),
        "5".as_ref(),
    ]);
    assert_eq!(
        status_of(&report, id),
        "passed",
        "{id}: {}",
        reason_line(&text, id)
    );
}
