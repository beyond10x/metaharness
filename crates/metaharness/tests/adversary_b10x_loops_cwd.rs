//! Adversary pass 1 against amendment a30 (`story:b10x-record-carries-the-loops-cwd`).
//!
//! Each case measures the production builder, b10x seam and floor over the bytes
//! `b10x-harness 0.15.0` wrote (the adapter's golden `loop.jsonl`, replayed by a scripted
//! runner), and then holds a document the unit published to what was measured. No vendor
//! process, no credential, no network.

use std::path::{Path, PathBuf};

use metaharness::protocol::{
    CredentialSource, DecisionMode, HermeticMode, HermeticRow, Kind, Severity, Verdict,
};
use metaharness::{
    AuditReport, FakeAuditor, Input, ManualClock, Metaharness, Run, ScriptedLog, ScriptedRunner,
};

const B10X_RECORDED_LOOP: &str = include_str!("../../metaharness-b10x/fixtures/golden/loop.jsonl");
const CLI_REFERENCE: &str = include_str!("../../../website/docs/reference/cli.md");
const CHANGELOG: &str = include_str!("../../../CHANGELOG.md");

/// A b10x run over the recorded loop through the production builder, seam and floor, audited.
fn audited_b10x_run(mode: HermeticMode, audit: bool, copy: Option<&Path>) -> (Run, AuditReport) {
    let mut runner = ScriptedRunner::of_lines(B10X_RECORDED_LOOP.lines(), ScriptedLog::new());
    let mut seams = metaharness_b10x::B10xSeams::new(
        Some(metaharness_b10x::PINNED_VERSIONS[0].to_owned()),
        None,
    );
    let mut builder = Metaharness::new(Kind::B10x)
        .with_hermetic(mode)
        .with_audit(audit)
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
    assert!(run.wants_audit(), "this case asked for a verdict");
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

fn embedder_copy() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let copy = root.path().join("copy");
    std::fs::create_dir(&copy).unwrap();
    (root, copy)
}

/// `website/docs/reference/cli.md`'s flag table says of `--scratch-cwd`, with no kind named,
/// that "`--hermetic strict` passes on the pair". The unit qualified the bullet further down
/// ("on the vendor kinds") and left the table row standing. Measured here: on b10x the pair under
/// strict exits 3, H7 alone. A reader of the flag table is told 0.
#[test]
fn the_cli_flag_table_does_not_promise_strict_passes_on_a_b10x_declared_copy() {
    let (_root, copy) = embedder_copy();
    let (run, report) = audited_b10x_run(HermeticMode::Strict, false, Some(&copy));
    assert_eq!(run.exit(Some(&report)).code(), 3, "{}", report.render());
    assert_eq!(gating_not_ok(&report), [(HermeticRow::H7, Verdict::Unk)]);

    let row = CLI_REFERENCE
        .lines()
        .find(|line| line.starts_with("| `--scratch-cwd` |"))
        .expect("the flag table has a --scratch-cwd row");
    let promises_pass = row.contains("`--hermetic strict` passes on the pair");
    let qualified = row.contains("b10x") || row.contains("vendor kinds");
    assert!(
        !promises_pass || qualified,
        "the --scratch-cwd flag-table row promises strict passes on the pair on every kind, and a \
         b10x declared copy under strict exits 3: {row}"
    );
}

/// The CHANGELOG's behaviour-change entry and a30 decision 3 ("The exits that move") name strict
/// runs only. Bare `--hermetic` with `--audit` judges the same floor (design § 9.4: a gating `unk`
/// is exit 3), so on b10x it moves from 0 to 3 as well, with or without a declared copy. Before
/// a30 this configuration read H7 `ok` ("which metaharness created" / "declared, not created"),
/// and H7 is the only gating row that is not `ok` here, so the 0 it exited before is the measured
/// difference.
#[test]
fn the_changelog_names_the_audited_non_strict_b10x_run_whose_exit_moved_to_3() {
    let (run, report) = audited_b10x_run(HermeticMode::On, true, None);
    assert_eq!(run.exit(Some(&report)).code(), 3, "{}", report.render());
    assert_eq!(gating_not_ok(&report), [(HermeticRow::H7, Verdict::Unk)]);
    let (_root, copy) = embedder_copy();
    let (run, report) = audited_b10x_run(HermeticMode::On, true, Some(&copy));
    assert_eq!(run.exit(Some(&report)).code(), 3, "{}", report.render());
    assert_eq!(gating_not_ok(&report), [(HermeticRow::H7, Verdict::Unk)]);

    let entry: String = CHANGELOG
        .split("\n- ")
        .find(|item| item.contains("story:b10x-record-carries-the-loops-cwd"))
        .expect("the CHANGELOG carries the a30 entry")
        .to_owned();
    assert!(
        entry.contains("--audit"),
        "the a30 behaviour-change entry names only strict runs, and `metaharness run b10x \
         --hermetic --audit` also moved from exit 0 to 3: {entry}"
    );
}
