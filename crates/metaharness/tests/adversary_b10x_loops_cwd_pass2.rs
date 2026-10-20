//! Adversary pass 2 against amendment a30 (`story:b10x-record-carries-the-loops-cwd`).
//!
//! A measurement, not a document check: `--audit` without `--hermetic` (mode `off`, the default)
//! on b10x. The b10x launch assembles the same attestation in every mode (`start_b10x` reads no
//! `spec.hermetic` beyond stamping it), and the floor reads no mode, so an audited run judges the
//! same rows whatever `--hermetic` says. Before a30, H7 read `ok` here; after it, H7 is the only
//! gating row that is not `ok`, so the exit moved from 0 to 3. The CHANGELOG entry names the
//! `--hermetic --audit` form of that move. Production builder, b10x seam and floor over a loop
//! record whose `started` line names no working directory — the state a30 measured; since a31 it
//! is derived from the bytes `b10x-harness 0.15.1` wrote by removing their `cwd` key. No vendor
//! process, no credential, no network.

use std::path::Path;

use metaharness::protocol::{
    CredentialSource, DecisionMode, HermeticMode, HermeticRow, Kind, Severity, Verdict,
};
use metaharness::{
    AuditReport, FakeAuditor, Input, ManualClock, Metaharness, Run, ScriptedLog, ScriptedRunner,
};

const B10X_RECORDED_LOOP: &str = include_str!("../../metaharness-b10x/fixtures/golden/loop.jsonl");

/// The recorded loop with its `started` line's `cwd` removed (checked present first).
fn recorded_loop_naming_no_directory() -> Vec<String> {
    B10X_RECORDED_LOOP
        .lines()
        .map(
            |line| match serde_json::from_str::<serde_json::Value>(line) {
                Ok(mut value) if value["kind"] == "started" => {
                    assert!(
                        value
                            .as_object_mut()
                            .and_then(|object| object.remove("cwd"))
                            .is_some(),
                        "the recorded started line names a working directory"
                    );
                    value.to_string()
                }
                _ => line.to_owned(),
            },
        )
        .collect()
}

fn audited_b10x_run(mode: HermeticMode, copy: Option<&Path>) -> (Run, AuditReport) {
    let mut runner =
        ScriptedRunner::of_lines(recorded_loop_naming_no_directory(), ScriptedLog::new());
    let mut seams = metaharness_b10x::B10xSeams::new(
        Some(metaharness_b10x::PINNED_VERSIONS[0].to_owned()),
        None,
    );
    let mut builder = Metaharness::new(Kind::B10x)
        .with_hermetic(mode)
        .with_audit(true)
        .with_credentials(CredentialSource::None)
        .with_decisions(DecisionMode::Observe)
        .with_model_endpoint("http://127.0.0.1:1")
        .with_model("b10x-emulated");
    if let Some(copy) = copy {
        builder = builder.with_cwd(copy).with_scratch_cwd();
    }
    let mut run = builder
        .start_with_clock(
            Input::Prompt("fixture".to_owned()),
            &mut runner,
            &mut seams,
            Box::new(ManualClock::new()),
        )
        .expect("the run plans on b10x");
    run.drain().unwrap();
    assert!(run.wants_audit(), "--audit asks for a verdict");
    let report = run
        .audit(&mut FakeAuditor::default())
        .expect("the floor always runs");
    (run, report)
}

fn gating_not_ok(report: &AuditReport) -> Vec<(HermeticRow, Verdict)> {
    report
        .rows
        .iter()
        .filter(|row| row.severity == Severity::Gating && row.verdict != Verdict::Ok)
        .map(|row| (row.row, row.verdict))
        .collect()
}

/// `metaharness run b10x --audit`, no `--hermetic`: exit 3 on H7 alone, with and without a
/// declared copy.
#[test]
fn b10x_audit_without_the_hermetic_flag_exits_3_on_h7_alone() {
    let (run, report) = audited_b10x_run(HermeticMode::Off, None);
    assert_eq!(run.exit(Some(&report)).code(), 3, "{}", report.render());
    assert_eq!(gating_not_ok(&report), [(HermeticRow::H7, Verdict::Unk)]);

    let root = tempfile::tempdir().unwrap();
    let copy = root.path().join("copy");
    std::fs::create_dir(&copy).unwrap();
    let (run, report) = audited_b10x_run(HermeticMode::Off, Some(&copy));
    assert_eq!(run.exit(Some(&report)).code(), 3, "{}", report.render());
    assert_eq!(gating_not_ok(&report), [(HermeticRow::H7, Verdict::Unk)]);
}
