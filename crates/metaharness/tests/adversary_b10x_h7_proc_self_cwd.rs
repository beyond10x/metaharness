//! Adversary pass 1 against amendment a31 (`story:b10x-h7-from-the-loops-cwd`).
//!
//! a31 point 3 and `audit.rs` `same_directory` state the rule this file holds the code to: "a
//! relative name is never resolved, because it would be resolved against metaharness's own working
//! directory, which says nothing about where the loop worked". `/proc/self/cwd` is an absolute
//! name whose whole meaning is *the working directory of the process that resolves it*, and
//! `std::fs::canonicalize` resolves it inside metaharness. So a record naming it is read as the
//! planned directory exactly when metaharness itself runs there: an embedder that `cd`s into its
//! disposable copy and runs `metaharness run b10x --hermetic strict --cwd . --scratch-cwd`.
//!
//! One test in its own binary, because it moves the test process's working directory and nothing
//! else may race it. Production builder, b10x seam and floor over the bytes `b10x-harness 0.15.1`
//! wrote, with the `started` line's `cwd` replaced. No vendor process, no credential, no network.
#![cfg(target_os = "linux")]

use metaharness::protocol::{
    CredentialSource, DecisionMode, HermeticMode, HermeticRow, Kind, Verdict,
};
use metaharness::{FakeAuditor, Input, ManualClock, Metaharness, ScriptedLog, ScriptedRunner};
use serde_json::Value;

const B10X_RECORDED_LOOP: &str = include_str!("../../metaharness-b10x/fixtures/golden/loop.jsonl");

/// The recorded loop with its `started` line's `cwd` set to `cwd` (checked present first).
fn recorded_loop_naming(cwd: &str) -> Vec<String> {
    B10X_RECORDED_LOOP
        .lines()
        .map(|line| match serde_json::from_str::<Value>(line) {
            Ok(mut value) if value["kind"] == "started" => {
                let object = value
                    .as_object_mut()
                    .expect("the started line is an object");
                assert!(
                    object.get("cwd").is_some_and(Value::is_string),
                    "the recorded started line names a working directory"
                );
                object.insert("cwd".to_owned(), Value::String(cwd.to_owned()));
                value.to_string()
            }
            _ => line.to_owned(),
        })
        .collect()
}

/// a31 point 3 (and the story's "H7 reads `ok` only when the recorded directory equals the one
/// metaharness created, `declared` … when it equals the declared copy"): a record that names the
/// resolving process's own working directory says nothing about where the loop worked, so it is
/// not the declared copy, whatever directory metaharness happens to run in.
#[test]
fn a_record_naming_proc_self_cwd_is_not_the_declared_copy_metaharness_runs_in() {
    let root = tempfile::tempdir().expect("a scratch root");
    let copy = root.path().join("copy");
    std::fs::create_dir(&copy).expect("the embedder's copy");
    std::env::set_current_dir(&copy).expect("the embedder runs metaharness from its copy");
    let planned = std::fs::canonicalize(&copy).unwrap().display().to_string();

    for magic in ["/proc/self/cwd", "/proc/thread-self/cwd"] {
        let mut runner = ScriptedRunner::of_lines(recorded_loop_naming(magic), ScriptedLog::new());
        let mut seams = metaharness_b10x::B10xSeams::new(
            Some(metaharness_b10x::PINNED_VERSIONS[0].to_owned()),
            None,
        );
        let mut run = Metaharness::new(Kind::B10x)
            .with_hermetic(HermeticMode::Strict)
            .with_credentials(CredentialSource::None)
            .with_decisions(DecisionMode::Observe)
            .with_model_endpoint("http://127.0.0.1:1")
            .with_model("b10x-emulated")
            .with_cwd(".")
            .with_scratch_cwd()
            .start_with_clock(
                Input::Prompt("fixture".to_owned()),
                &mut runner,
                &mut seams,
                Box::new(ManualClock::new()),
            )
            .expect("the run plans on b10x");
        run.drain().unwrap();
        let recorded = run.events().iter().find_map(|event| match event {
            metaharness::protocol::Event::SessionStarted { cwd, .. } => Some(cwd.clone()),
            _ => None,
        });
        assert_eq!(
            recorded,
            Some(Some(magic.to_owned())),
            "the seam passes it through"
        );
        let report = run
            .audit(&mut FakeAuditor::default())
            .expect("the floor always runs");
        let h7 = report
            .rows
            .iter()
            .find(|row| row.row == HermeticRow::H7)
            .expect("H7 is evaluated");
        assert_eq!(
            (h7.verdict, h7.detail.as_str()),
            (
                Verdict::Gap,
                format!("metaharness planned {planned} and the session ran in {magic}").as_str()
            ),
            "{magic} was resolved against metaharness's own working directory, which a31 says \
             says nothing about where the loop worked:\n{}",
            report.render()
        );
        assert_ne!(run.exit(Some(&report)).code(), 0, "{}", report.render());
    }
}
