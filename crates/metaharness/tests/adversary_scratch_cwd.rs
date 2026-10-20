//! Adversary cases against amendment a27 (`story:strict-hermetic-with-embedder-scratch-cwd`), and
//! its b10x cases as amendment a30 left them (`story:b10x-record-carries-the-loops-cwd`) and a31
//! moved them (`story:b10x-h7-from-the-loops-cwd`: the seam reads the loop's `cwd`).
//!
//! Each case drives the production builder, a scripted runner and the production floor, and
//! asserts a sentence of a27, of a30 or of a story's acceptance statement. No vendor process, no
//! credential, no network.

use std::path::{Path, PathBuf};

use metaharness::protocol::{
    CredentialSource, DecisionMode, Event, HermeticAttestation, HermeticMode, HermeticRow, Kind,
    RowVerdict, Severity, Verdict,
};
use metaharness::{
    AuditReport, FakeAuditor, Input, ManualClock, Metaharness, Run, ScriptedLog, ScriptedRunner,
    SeamFactory,
};

/// A fresh root holding `copy/`, the directory the embedder made.
fn embedder_copy() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let copy = root.path().join("copy");
    std::fs::create_dir(&copy).unwrap();
    (root, copy)
}

fn opening(run: &Run) -> (HermeticAttestation, Option<String>) {
    run.events()
        .iter()
        .find_map(|event| {
            if let Event::SessionStarted { hermetic, cwd, .. } = event {
                Some((hermetic.clone(), cwd.clone()))
            } else {
                None
            }
        })
        .expect("the run opened")
}

fn floor_row(run: &Run, row: HermeticRow) -> RowVerdict {
    run.hermetic_floor()
        .into_iter()
        .find(|verdict| verdict.row == row)
        .expect("every row is evaluated")
}

// ------------------------------------------------------------------------------ the b10x kind

const B10X_STARTED: &str =
    r#"{"kind":"started","model":"synthetic","published_tools":[],"credential_source":"none"}"#;
const B10X_FINISHED: &str = r#"{"kind":"finished","stop":{"kind":"completed"}}"#;

/// The bytes `b10x-harness 0.15.1` wrote, recorded for the adapter's golden vector. Its `started`
/// line names the recording's own directory (`/scratch/run/work`).
const B10X_RECORDED_LOOP: &str = include_str!("../../metaharness-b10x/fixtures/golden/loop.jsonl");

/// What H7 says on this kind when the record names no working directory (design a30).
const LOOP_REPORTED_NO_CWD: &str = "the loop did not report its working directory";

fn production_seams() -> metaharness_b10x::B10xSeams {
    metaharness_b10x::B10xSeams::new(Some(metaharness_b10x::PINNED_VERSIONS[0].to_owned()), None)
}

/// A strict b10x run over `copy` with the loop's lines and the seam factory chosen by the case.
fn b10x_strict_run_with(
    copy: &Path,
    declared: bool,
    started: &str,
    seams: &mut dyn SeamFactory,
) -> Run {
    let mut runner = ScriptedRunner::of_lines([started, B10X_FINISHED], ScriptedLog::new());
    let mut builder = Metaharness::new(Kind::B10x)
        .with_hermetic(HermeticMode::Strict)
        .with_credentials(CredentialSource::None)
        .with_decisions(DecisionMode::Observe)
        .with_model_endpoint("http://127.0.0.1:1")
        .with_model("synthetic-never-called")
        .with_cwd(copy);
    if declared {
        builder = builder.with_scratch_cwd();
    }
    let mut run = builder
        .start_with_clock(
            Input::Prompt("fixture".to_owned()),
            &mut runner,
            seams,
            Box::new(ManualClock::new()),
        )
        .expect("a named directory plans on b10x");
    run.drain().unwrap();
    run
}

/// A strict b10x run over `copy` through the **production** b10x seam, whose `started` line
/// carries no working directory of its own, so the opening record carries none either (design
/// a30).
fn b10x_strict_run(copy: &Path, declared: bool) -> Run {
    let mut seams = production_seams();
    let mut runner = ScriptedRunner::of_lines([B10X_STARTED, B10X_FINISHED], ScriptedLog::new());
    let mut builder = Metaharness::new(Kind::B10x)
        .with_hermetic(HermeticMode::Strict)
        .with_credentials(CredentialSource::None)
        .with_decisions(DecisionMode::Observe)
        .with_model_endpoint("http://127.0.0.1:1")
        .with_model("synthetic-never-called")
        .with_cwd(copy);
    if declared {
        builder = builder.with_scratch_cwd();
    }
    let mut run = builder
        .start_with_clock(
            Input::Prompt("fixture".to_owned()),
            &mut runner,
            &mut seams,
            Box::new(ManualClock::new()),
        )
        .expect("a named directory plans on b10x");
    run.drain().unwrap();
    run
}

/// a27 § 4 and CHANGELOG "Changed" said an a6 `--cwd` directory is a `gap` in metaharness's own
/// words; a27 "The b10x kind" said the same rule applies there. On b10x that `gap` stood on
/// metaharness's echo of its own plan, and design a30 removes the echo: the loop's `started` line
/// names no working directory, so the record carries none and H7 reads `unk` in the loop's words.
/// The `(None, _)` arm runs before the attestation is read, which is why the a6 `unavailable` no
/// longer decides the row on this kind. The attestation still says it — a launch fact, which the
/// floor reports beside the record and does not substitute for it. `metaharness aep drive` passes
/// `--cwd` on the b10x arm (`drive.rs`), so this is the driven arm's floor row.
#[test]
fn b10x_a6_named_directory_is_unk_on_h7_because_the_loop_reported_no_directory() {
    let (_root, copy) = embedder_copy();
    let run = b10x_strict_run(&copy, false);
    let h7 = floor_row(&run, HermeticRow::H7);
    assert_eq!(h7.verdict, Verdict::Unk, "{}", h7.detail);
    assert_eq!(h7.detail, LOOP_REPORTED_NO_CWD);
    assert!(
        !h7.detail.contains("which metaharness created"),
        "{}",
        h7.detail
    );
    let (attested, event_cwd) = opening(&run);
    assert_eq!(
        event_cwd, None,
        "the record carries no directory the loop did not report"
    );
    assert!(
        attested
            .unavailable
            .iter()
            .any(|control| control.row == HermeticRow::H7),
        "a6: H7 is attested unavailable under an operator-named --cwd; b10x attested: {:?}",
        attested.unavailable
    );
    // The exit does not move: a6 attests H11 unavailable on this kind too, its `gap` outranks
    // H7's `unk`, and the run still ends at 1 as it did when H7 was the `gap`.
    let report = run
        .audit(&mut FakeAuditor::default())
        .expect("the floor always runs");
    assert_eq!(
        floor_row(&run, HermeticRow::H11).verdict,
        Verdict::Gap,
        "{}",
        report.render()
    );
    assert_eq!(run.exit(Some(&report)).code(), 1, "{}", report.render());
}

/// a27 § 4 on this kind, after `story:b10x-record-carries-the-loops-cwd` (design a30) and
/// `story:b10x-h7-from-the-loops-cwd` (design a31). This case was written to go red when the
/// first of those landed: it pinned H7 `ok` on b10x under the declaration although the loop's line
/// named `/somewhere/else`, because the seam wrote the plan into the record and the floor compared
/// the plan with its own echo.
///
/// The line still names `/somewhere/else`, under the top-level `cwd` key harness 0.15.1 writes.
/// Since a31 the seam reads that key, so the record states `/somewhere/else` and H7 compares it
/// with the declared copy: `gap`, never `ok` on the flag plus the plan.
#[test]
fn b10x_declared_copy_h7_is_a_gap_and_never_ok_on_the_plan_when_the_loop_reports_elsewhere() {
    let (_root, copy) = embedder_copy();
    let started = r#"{"kind":"started","model":"synthetic","published_tools":[],"credential_source":"none","cwd":"/somewhere/else"}"#;
    let run = b10x_strict_run_with(&copy, true, started, &mut production_seams());
    let (attested, event_cwd) = opening(&run);
    assert!(attested.declares(HermeticRow::H7));
    assert_eq!(
        event_cwd.as_deref(),
        Some("/somewhere/else"),
        "the b10x opening record carries the line's `cwd` key, never the plan"
    );
    let h7 = floor_row(&run, HermeticRow::H7);
    assert_eq!(h7.verdict, Verdict::Gap, "{}", h7.detail);
    assert_eq!(
        h7.detail,
        format!(
            "metaharness planned {} and the session ran in /somewhere/else",
            std::fs::canonicalize(&copy).unwrap().display()
        )
    );
    assert!(
        !h7.detail.contains("declared, not created"),
        "the declaration does not stand in for the record: {}",
        h7.detail
    );
}

/// A strict b10x run that replays the bytes `b10x-harness 0.15.1` wrote **with the `started`
/// line's `cwd` removed**, in `copy` declared as the embedder's scratch copy or, given none, in the
/// directory metaharness creates; and its audit.
///
/// The absent shape over a derived record (design a31): the golden names a working directory since
/// 0.15.1, and what both callers pin rests on a loop that names none — which the pinned loop still
/// writes under `--substrate <socket>` and for a root whose name is not UTF-8. The key is checked
/// present before it is removed, so the derivation cannot pass on a recording that never had it.
/// The production builder, seam and floor; a scripted runner replays the record; the exit is the
/// one `metaharness run` returns for a strict run (`run.exit(Some(&report))`).
fn b10x_strict_run_over_the_recorded_loop(copy: Option<&Path>) -> (Run, AuditReport) {
    let mut opening_line: serde_json::Value = serde_json::from_str(
        B10X_RECORDED_LOOP
            .lines()
            .next()
            .expect("the recorded loop opens"),
    )
    .expect("the opening line is JSON");
    assert_eq!(opening_line["kind"], "started");
    assert!(
        opening_line
            .as_object_mut()
            .expect("the opening line is an object")
            .remove("cwd")
            .is_some(),
        "the recorded opening line names a working directory: {opening_line}"
    );
    let derived: Vec<String> = std::iter::once(opening_line.to_string())
        .chain(B10X_RECORDED_LOOP.lines().skip(1).map(ToOwned::to_owned))
        .collect();

    let mut runner = ScriptedRunner::of_lines(derived, ScriptedLog::new());
    let mut seams = production_seams();
    let mut builder = Metaharness::new(Kind::B10x)
        .with_hermetic(HermeticMode::Strict)
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
    assert!(run.wants_audit(), "strict owes a verdict");
    let report = run
        .audit(&mut FakeAuditor::default())
        .expect("the floor always runs");
    (run, report)
}

/// Every gating row that is not `ok`, with its words.
fn gating_not_ok(report: &AuditReport) -> Vec<(HermeticRow, Verdict, String)> {
    report
        .rows
        .iter()
        .filter(|row| row.severity == Severity::Gating && row.verdict != Verdict::Ok)
        .map(|row| (row.row, row.verdict, row.detail.clone()))
        .collect()
}

/// The story's pinned exit: `metaharness run b10x --hermetic strict --cwd <copy> --scratch-cwd`
/// over a record whose `started` line names no working directory ends at **3**, nobody found out,
/// and H7 is the one gating row that is not `ok`. It was 0 while the seam echoed the plan. Since
/// a31 a record that names the copy reads `ok` and exits 0 (`b10x_h7_loops_cwd.rs`); this is the
/// absent shape, over the 0.15.1 recording with its `cwd` removed.
#[test]
fn b10x_strict_declared_copy_over_the_recorded_loop_exits_3_on_h7() {
    let (_root, copy) = embedder_copy();
    let (run, report) = b10x_strict_run_over_the_recorded_loop(Some(&copy));
    assert_eq!(run.exit(Some(&report)).code(), 3, "{}", report.render());
    let (attested, event_cwd) = opening(&run);
    assert!(attested.declares(HermeticRow::H7));
    assert_eq!(event_cwd, None);
    assert_eq!(
        gating_not_ok(&report),
        [(
            HermeticRow::H7,
            Verdict::Unk,
            LOOP_REPORTED_NO_CWD.to_owned()
        )],
        "H7 is the only gating row not ok"
    );
}

/// The same holds without `--cwd`, where metaharness creates the directory: the row's evidence is
/// the record (§ 8.1 H7), not who made the directory, so a strict b10x run in metaharness's own
/// scratch cwd is `unk` on H7 and ends at 3 as well, when the record names no directory. Before
/// design a30 the record echoed that directory and the row read `ok`, *"which metaharness
/// created"*; since a31 it reads `ok` again only on a record that names that directory.
#[test]
fn b10x_strict_run_in_metaharness_own_scratch_cwd_exits_3_on_h7() {
    let (run, report) = b10x_strict_run_over_the_recorded_loop(None);
    assert_eq!(run.exit(Some(&report)).code(), 3, "{}", report.render());
    let (attested, event_cwd) = opening(&run);
    assert!(!attested.declares(HermeticRow::H7));
    assert_eq!(event_cwd, None);
    assert_eq!(
        gating_not_ok(&report),
        [(
            HermeticRow::H7,
            Verdict::Unk,
            LOOP_REPORTED_NO_CWD.to_owned()
        )],
        "H7 is the only gating row not ok"
    );
}
