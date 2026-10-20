//! Adversary cases against amendment a27 (`story:strict-hermetic-with-embedder-scratch-cwd`).
//!
//! Each case drives the production builder, a scripted runner and the production floor, and
//! asserts a sentence of a27 or of the story's acceptance statement. No vendor process, no
//! credential, no network.

use std::path::{Path, PathBuf};

use metaharness::protocol::{
    CredentialSource, DecisionMode, Event, HermeticAttestation, HermeticMode, HermeticRow, Kind,
    RowVerdict, Verdict,
};
use metaharness::{
    Input, ManualClock, Metaharness, Run, ScriptedLog, ScriptedRunner, SeamFactory,
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

fn production_seams() -> metaharness_b10x::B10xSeams {
    metaharness_b10x::B10xSeams::new(
        Some(metaharness_b10x::PINNED_VERSIONS[0].to_owned()),
        None,
        None,
    )
}

/// A strict b10x run over `copy` with the loop's lines and the seam factory chosen by the case.
fn b10x_strict_run_with(
    copy: &Path,
    declared: bool,
    started: &str,
    seams: &mut dyn SeamFactory,
) -> Run {
    let mut runner =
        ScriptedRunner::of_lines([started, B10X_FINISHED], ScriptedLog::new());
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
/// carries no working directory of its own (`metaharness-b10x/src/seam.rs`, `cwd: self.cwd`).
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

/// a27 § 4 and CHANGELOG "Changed": *"an a6 `--cwd` directory is now a `gap` in metaharness's own
/// words, where before the arm compared two paths and printed `ok` 'which metaharness created'
/// over a directory metaharness did not create"*; a27 "The b10x kind": *"the same rule applies
/// there"*. `metaharness aep drive` passes `--cwd` on the b10x arm (`drive.rs`), so this is the
/// driven arm's floor row.
#[test]
fn b10x_a6_named_directory_is_a_gap_on_h7_in_metaharness_own_words() {
    let (_root, copy) = embedder_copy();
    let run = b10x_strict_run(&copy, false);
    let h7 = floor_row(&run, HermeticRow::H7);
    assert_eq!(h7.verdict, Verdict::Gap, "{}", h7.detail);
    assert!(
        !h7.detail.contains("which metaharness created"),
        "{}",
        h7.detail
    );
    let (attested, _) = opening(&run);
    assert!(
        attested
            .unavailable
            .iter()
            .any(|control| control.row == HermeticRow::H7),
        "a6: H7 is attested unavailable under an operator-named --cwd; b10x attested: {:?}",
        attested.unavailable
    );
}

/// a27 § 4 on this kind, pinned as **today's state** and written to announce the landing of
/// `story:b10x-record-carries-the-loops-cwd`. The loop's `started` line names `/somewhere/else`;
/// the plan is the embedder's copy. The production seam never reads the line's cwd and writes the
/// planned one into the opening record (`metaharness-b10x/src/seam.rs`, `cwd: self.cwd`), so the
/// floor compares the plan with its own echo and reads H7 `ok` — on the flag plus the echo, not on
/// a directory the loop reported. That is the story's gap, and neither the seam nor the golden
/// expected stream is this unit's to change.
///
/// When the seam reads the line's cwd, this case goes red under either landing shape — no echo at
/// all leaves the record without a cwd (`None`, H7 `unk`), a fallback echo in the shape
/// `model: string("model").or_else(|| self.model.clone())` puts `/somewhere/else` in the record
/// (H7 `gap`) — and the test is then inverted: assert `Verdict::Gap` and a record carrying the
/// loop's directory, which is the comparison a27 § 4 describes made real on this kind.
#[test]
fn b10x_declared_copy_h7_reads_ok_on_the_echoed_plan_although_the_loop_named_another_directory() {
    let (_root, copy) = embedder_copy();
    let started = r#"{"kind":"started","model":"synthetic","published_tools":[],"credential_source":"none","cwd":"/somewhere/else"}"#;
    let run = b10x_strict_run_with(&copy, true, started, &mut production_seams());
    let planned = std::fs::canonicalize(&copy).unwrap().display().to_string();
    let (attested, event_cwd) = opening(&run);
    assert!(attested.declares(HermeticRow::H7));
    assert_eq!(
        event_cwd.as_deref(),
        Some(planned.as_str()),
        "the b10x opening record no longer echoes the plan over a loop line naming another \
         directory: story:b10x-record-carries-the-loops-cwd has landed; invert this case to \
         assert a record carrying the loop's directory and H7 gap"
    );
    let h7 = floor_row(&run, HermeticRow::H7);
    assert_eq!(
        h7.verdict,
        Verdict::Ok,
        "today H7 is ok on b10x under the declaration although the loop named /somewhere/else, \
         because the seam echoes the plan (story:b10x-record-carries-the-loops-cwd); when the seam \
         reads the line's cwd this flips to gap and the test is to be inverted: {}",
        h7.detail
    );
    assert!(
        h7.detail.contains("declared, not created"),
        "the row must say the directory was declared, never that metaharness created it: {}",
        h7.detail
    );
}
