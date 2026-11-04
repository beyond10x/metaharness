//! `story:aep-pin-0-77-1`: a run stopped at an `operator` step under `--approver` is answered by
//! `metaharness aep drive resume <run> --evidence <file>`.
//!
//! Under the aep 0.77.0 link the pause text named `aep govern evaluate --evidence --state`, which
//! evaluates a restored snapshot and writes nothing back, so every resume found nothing recorded
//! and stopped again. aep 0.77.1 moved the route into the resume (`ResumeArgs::evidence`,
//! `DriverOptions::evidence`): the records are submitted to the restored execution before the owed
//! step is settled. This drives the hosted verb the binary dispatches to (`drive::run`, which
//! `metaharness_aep::execute` wraps) offline: a map of `operator` steps only, so no model, no
//! `--aep-binary` and nothing on the network is involved.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use metaharness_aep::drive::DriveCommand;

/// `metaharness aep drive`, parsed the way the binary parses it.
#[derive(Debug, Parser)]
#[command(name = "drive")]
struct Drive {
    #[command(subcommand)]
    command: DriveCommand,
}

/// The pinned AEP source tree, resolved from the exact dependency and never an ambient sibling.
fn aep_root() -> PathBuf {
    let output = std::process::Command::new(env!("CARGO"))
        .args(["metadata", "--locked", "--format-version", "1"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("metadata is JSON");
    let manifest = metadata["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .find(|package| package["name"] == "aep-cli")
        .expect("the linked aep-cli")["manifest_path"]
        .as_str()
        .expect("a manifest path")
        .to_owned();
    Path::new(&manifest)
        .parent()
        .expect("the crate directory")
        .join("../../..")
        .canonicalize()
        .expect("the AEP source tree")
}

/// One granted approval by `agent:orchestrator`, in the records document `--evidence` reads.
const ORCHESTRATOR_APPROVAL: &str = "\
- kind: approval
  approval: specification
  decision: granted
  approver:
    producer: agent
    id: orchestrator
  observed_at: 2026-10-22
  producer:
    producer: agent
    id: orchestrator
";

/// Two `operator` steps in `receive`: the first is the one the approval answers, the second stops
/// the run again so the cursor shows where it carried on to.
const MAP: &str = concat!(
    "format: aep.driver-steps/1\n",
    "id: fixture/resume-evidence\n",
    "workflow: adp/default/2\n",
    "states:\n",
    "  receive:\n",
    "    steps:\n",
    "      - kind: operator\n",
    "        prompt: judge this change\n",
    "      - kind: operator\n",
    "        prompt: stop after the answer\n",
);

fn drive(project: &Path, root: &Path, verb: &[&str]) -> ExitCode {
    let mut argv: Vec<String> = vec!["drive".to_owned()];
    argv.extend(verb.iter().map(ToString::to_string));
    argv.extend(
        [
            "--project",
            project.to_str().expect("a printable path"),
            "--root",
            root.to_str().expect("a printable path"),
            "--task",
            project
                .join("task.yaml")
                .to_str()
                .expect("a printable path"),
            "--map",
            project
                .join("steps.yaml")
                .to_str()
                .expect("a printable path"),
            "--pause-on-approval",
        ]
        .map(ToOwned::to_owned),
    );
    let parsed = Drive::try_parse_from(&argv).unwrap_or_else(|error| {
        panic!(
            "`metaharness aep {}` does not parse: {error}",
            argv.join(" ")
        )
    });
    metaharness_aep::drive::run(parsed.command).unwrap_or_else(|error| {
        panic!(
            "`metaharness aep {}` was refused: {error:#}",
            argv.join(" ")
        )
    })
}

fn cursor(project: &Path) -> serde_json::Value {
    let path = project.join(".engineering/runs/RESUME-1/1/cursor.json");
    serde_json::from_str(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("reading {}: {error}", path.display())),
    )
    .expect("the cursor is JSON")
}

#[test]
fn an_approval_handed_to_the_hosted_resume_answers_the_operator_step_and_the_run_carries_on() {
    let root = aep_root();
    let scratch = tempfile::tempdir().expect("scratch project");
    let project = scratch.path();
    std::fs::create_dir_all(project.join(".engineering/planning")).unwrap();
    std::fs::write(
        project.join("task.yaml"),
        "id: RESUME-1\nkind: feature\nobjective: answer an operator step on resume\n\
         protocol: adp/1\nprofile: development.standard\n",
    )
    .unwrap();
    std::fs::write(project.join("steps.yaml"), MAP).unwrap();

    let started = drive(
        project,
        &root,
        &[
            "run",
            "--allow-evidence-gap",
            "--approver",
            "agent:orchestrator",
        ],
    );
    assert_eq!(started, ExitCode::SUCCESS);
    let paused = cursor(project);
    assert_eq!(
        paused["status"], "awaiting_operator",
        "the run did not reach the operator step, so this case is about nothing: {paused}"
    );
    assert_eq!(paused["owed"]["step"], 0, "{paused}");
    assert!(
        paused
            .get("answers")
            .is_none_or(|answers| answers.as_array().is_some_and(Vec::is_empty)),
        "nothing has answered yet: {paused}"
    );

    let approval = project.join("approval.yaml");
    std::fs::write(&approval, ORCHESTRATOR_APPROVAL).unwrap();
    let resumed = drive(
        project,
        &root,
        &[
            "resume",
            "RESUME-1/1",
            "--evidence",
            approval.to_str().expect("a printable path"),
        ],
    );
    assert_eq!(resumed, ExitCode::SUCCESS);
    let after = cursor(project);
    assert_eq!(
        after["answers"][0]["by"], "agent orchestrator",
        "the approval handed to the resume did not answer the step: {after}"
    );
    assert_eq!(after["answers"][0]["approval"], "specification", "{after}");
    assert_eq!(after["answers"][0]["state"], "receive", "{after}");
    assert_eq!(after["answers"][0]["step"], 0, "{after}");
    assert_eq!(
        after["owed"]["step"], 1,
        "the run carried on past the answered step to the next one: {after}"
    );
}
