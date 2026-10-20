//! The b10x H7 row read from the working directory the loop reports (design amendment a31,
//! `story:b10x-h7-from-the-loops-cwd`).
//!
//! Every case drives the production builder, the production b10x seam and the production floor
//! over the bytes `b10x-harness 0.15.1` wrote (the adapter's golden `loop.jsonl`), replayed by a
//! scripted runner. The record is used as recorded, or derived from it at the spawn by setting the
//! `started` line's `cwd` to the directory the launch plan names (under the spelling the case
//! chooses) or by removing the key. No vendor process, no credential, no network.
#![cfg(unix)]

use std::path::{Path, PathBuf};

use metaharness::protocol::{
    CredentialSource, DecisionMode, HermeticMode, HermeticRow, Kind, Severity, Verdict,
};
use metaharness::{
    AuditReport, FakeAuditor, HarnessProcess, Input, LaunchPlanView, ManualClock, Metaharness,
    ProcessRunner, Run, ScriptedLog, ScriptedRunner,
};
use serde_json::Value;

/// The bytes `b10x-harness 0.15.1` wrote, recorded for the adapter's golden vector.
const B10X_RECORDED_LOOP: &str = include_str!("../../metaharness-b10x/fixtures/golden/loop.jsonl");

/// The directory the recording's `started` line names.
const RECORDED_CWD: &str = "/scratch/run/work";

/// Where the run works.
#[derive(Debug, Clone, Copy)]
enum Directory {
    /// The scratch directory metaharness creates (no `--cwd`).
    Created,
    /// `--cwd <copy> --scratch-cwd`: the embedder's declared copy (a27).
    Declared,
    /// `--cwd <copy>` without the declaration: a6's operator-named tree.
    Named,
}

/// What the replayed `started` line says about its working directory.
#[derive(Debug, Clone, Copy)]
enum Reported {
    /// The planned directory, canonical: what the pinned loop writes (the canonical root its file
    /// tools act in).
    Canonical,
    /// The planned directory through a link made at the spawn: the same directory, another
    /// spelling.
    ThroughALink,
    /// The recording's own directory, which is not this run's.
    AsRecorded,
    /// No `cwd` key: the pinned loop under `--substrate <socket>`, or a root whose name is not
    /// UTF-8.
    Absent,
}

/// A runner that derives the loop's record from the golden at the spawn, when the launch plan's
/// directory is known.
struct DerivingRunner {
    reported: Reported,
    links: PathBuf,
    log: ScriptedLog,
}

impl ProcessRunner for DerivingRunner {
    fn requires_executable(&self) -> bool {
        false
    }

    fn start(&mut self, plan: &LaunchPlanView) -> std::io::Result<Box<dyn HarnessProcess>> {
        let cwd = match self.reported {
            Reported::Canonical => Some(std::fs::canonicalize(plan.cwd)?.display().to_string()),
            Reported::ThroughALink => {
                let link = self.links.join("alias");
                std::os::unix::fs::symlink(plan.cwd, &link)?;
                Some(link.display().to_string())
            }
            Reported::AsRecorded => Some(RECORDED_CWD.to_owned()),
            Reported::Absent => None,
        };
        let record = with_working_directory(B10X_RECORDED_LOOP, cwd.as_deref());
        ScriptedRunner::of_lines(record.lines(), self.log.clone()).start(plan)
    }
}

/// The record with its `started` line's `cwd` set to `cwd`, or removed when `None`.
fn with_working_directory(record: &str, cwd: Option<&str>) -> String {
    record
        .lines()
        .map(|line| match serde_json::from_str::<Value>(line) {
            Ok(mut value) if value["kind"] == "started" => {
                let object = value
                    .as_object_mut()
                    .expect("the started line is an object");
                assert_eq!(
                    object.get("cwd").and_then(Value::as_str),
                    Some(RECORDED_CWD),
                    "the recorded started line names its working directory"
                );
                match cwd {
                    Some(cwd) => {
                        object.insert("cwd".to_owned(), Value::String(cwd.to_owned()));
                    }
                    None => {
                        object.remove("cwd");
                    }
                }
                format!("{value}\n")
            }
            _ => format!("{line}\n"),
        })
        .collect()
}

struct Observed {
    run: Run,
    report: AuditReport,
    /// The directory the launch plan named.
    planned: String,
    /// The directory the opening record states.
    recorded: Option<String>,
}

/// One b10x run over the derived record, audited the way `metaharness run` audits it.
fn b10x_run(
    mode: HermeticMode,
    audit: bool,
    directory: Directory,
    reported: Reported,
) -> (tempfile::TempDir, Observed) {
    let root = tempfile::tempdir().expect("a scratch root");
    let copy = root.path().join("copy");
    std::fs::create_dir(&copy).expect("the embedder's copy");
    let log = ScriptedLog::new();
    let mut runner = DerivingRunner {
        reported,
        links: root.path().to_owned(),
        log: log.clone(),
    };
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
    match directory {
        Directory::Created => {}
        Directory::Declared => builder = builder.with_cwd(&copy).with_scratch_cwd(),
        Directory::Named => builder = builder.with_cwd(&copy),
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
    assert!(run.wants_audit(), "every case here owes a verdict");
    let report = run
        .audit(&mut FakeAuditor::default())
        .expect("the floor always runs");
    let planned = log
        .launched_cwds()
        .first()
        .map(|cwd| cwd.display().to_string())
        .expect("one spawn");
    let recorded = run.events().iter().find_map(|event| match event {
        metaharness::protocol::Event::SessionStarted { cwd, .. } => Some(cwd.clone()),
        _ => None,
    });
    let observed = Observed {
        run,
        report,
        planned,
        recorded: recorded.expect("the run opened"),
    };
    (root, observed)
}

fn h7(observed: &Observed) -> (Verdict, String) {
    let row = observed
        .report
        .rows
        .iter()
        .find(|row| row.row == HermeticRow::H7)
        .expect("H7 is evaluated");
    (row.verdict, row.detail.clone())
}

fn gating_not_ok(observed: &Observed) -> Vec<(HermeticRow, Verdict)> {
    observed
        .report
        .rows
        .iter()
        .filter(|row| row.severity == Severity::Gating && row.verdict != Verdict::Ok)
        .map(|row| (row.row, row.verdict))
        .collect()
}

fn exit(observed: &Observed) -> i32 {
    observed.run.exit(Some(&observed.report)).code()
}

/// The three verdict-owing configurations a30 moved to 3: strict, `--hermetic --audit`, and plain
/// `--audit`.
const OWING: [(HermeticMode, bool); 3] = [
    (HermeticMode::Strict, false),
    (HermeticMode::On, true),
    (HermeticMode::Off, true),
];

/// **Present and equal.** The record states the directory metaharness created, or the copy the
/// embedder declared, under either spelling, and H7 is `ok` in the words of who made it: *which
/// metaharness created*, or a27's *which was declared, not created*. It is the only gating row a30
/// left not `ok`, so all three configurations a30 moved from 0 to 3 come back to **0**.
#[test]
fn a_record_that_states_the_created_or_declared_directory_is_ok_on_h7_and_exits_0() {
    for (mode, audit) in OWING {
        for directory in [Directory::Created, Directory::Declared] {
            for reported in [Reported::Canonical, Reported::ThroughALink] {
                let case = format!("{mode:?} audit={audit} {directory:?} {reported:?}");
                let (_root, observed) = b10x_run(mode, audit, directory, reported);
                let recorded = observed.recorded.clone().expect("the loop reported one");
                let (verdict, detail) = h7(&observed);
                assert_eq!(verdict, Verdict::Ok, "{case}: {detail}");
                let words = match directory {
                    Directory::Declared => "which was declared, not created",
                    _ => "which metaharness created",
                };
                assert!(
                    detail.starts_with(&format!("the session ran in {recorded}, {words}")),
                    "{case}: {detail}"
                );
                assert!(
                    gating_not_ok(&observed).is_empty(),
                    "{case}: {}",
                    observed.report.render()
                );
                assert_eq!(exit(&observed), 0, "{case}");
            }
        }
    }
}

/// The spelling the case chose is the one the record states: the canonical form of the plan, or
/// the link made at the spawn, which differs from the plan byte for byte. So the `ok` above stood
/// on the canonical comparison, not on two equal strings.
#[test]
fn the_link_spelling_is_not_the_planned_spelling() {
    for directory in [Directory::Created, Directory::Declared] {
        let (root, observed) = b10x_run(
            HermeticMode::Strict,
            false,
            directory,
            Reported::ThroughALink,
        );
        let recorded = observed.recorded.clone().expect("the loop reported one");
        assert_eq!(Path::new(&recorded), root.path().join("alias"));
        assert_ne!(recorded, observed.planned, "{directory:?}");
        let (_root, canonical) =
            b10x_run(HermeticMode::Strict, false, directory, Reported::Canonical);
        assert_eq!(
            canonical.recorded.as_deref().map(Path::new),
            Some(std::fs::canonicalize(&canonical.planned).unwrap().as_path()),
            "{directory:?}"
        );
    }
}

/// **Present and different.** The recording's own directory is not this run's: H7 is `gap`,
/// *"metaharness planned … and the session ran in …"*, and a strict or audited run exits **1**,
/// whoever made the planned directory. Never `ok` on the plan.
#[test]
fn a_record_that_states_another_directory_is_a_gap_on_h7_and_exits_1() {
    for (mode, audit) in OWING {
        for directory in [Directory::Created, Directory::Declared] {
            let case = format!("{mode:?} audit={audit} {directory:?}");
            let (_root, observed) = b10x_run(mode, audit, directory, Reported::AsRecorded);
            assert_eq!(observed.recorded.as_deref(), Some(RECORDED_CWD), "{case}");
            let (verdict, detail) = h7(&observed);
            assert_eq!(verdict, Verdict::Gap, "{case}: {detail}");
            assert_eq!(
                detail,
                format!(
                    "metaharness planned {} and the session ran in {RECORDED_CWD}",
                    observed.planned
                ),
                "{case}"
            );
            assert_eq!(
                gating_not_ok(&observed),
                [(HermeticRow::H7, Verdict::Gap)],
                "{case}"
            );
            assert_eq!(exit(&observed), 1, "{case}");
        }
    }
}

/// **Absent.** A record with no `cwd` keeps a30's branch: the opening record states `null`, H7 is
/// `unk` in the loop's words, and a strict or audited run exits **3**, whoever made the directory.
#[test]
fn a_record_that_states_no_directory_keeps_unk_on_h7_and_exits_3() {
    for (mode, audit) in OWING {
        for directory in [Directory::Created, Directory::Declared] {
            let case = format!("{mode:?} audit={audit} {directory:?}");
            let (_root, observed) = b10x_run(mode, audit, directory, Reported::Absent);
            assert_eq!(observed.recorded, None, "{case}");
            let (verdict, detail) = h7(&observed);
            assert_eq!(verdict, Verdict::Unk, "{case}: {detail}");
            assert_eq!(
                detail, "the loop did not report its working directory",
                "{case}"
            );
            assert_eq!(
                gating_not_ok(&observed),
                [(HermeticRow::H7, Verdict::Unk)],
                "{case}"
            );
            assert_eq!(exit(&observed), 3, "{case}");
        }
    }
}

/// a6 on this kind, now that the loop reports the directory: the operator's tree is the one the
/// session ran in, metaharness could not impose it, so H7 is `gap` in a6's words (a27 § 4), H11 is
/// `gap` beside it, and the strict run **stays at 1**.
#[test]
fn an_a6_named_directory_the_loop_reports_is_a_gap_on_h7_and_stays_at_1() {
    let (_root, observed) = b10x_run(
        HermeticMode::Strict,
        false,
        Directory::Named,
        Reported::Canonical,
    );
    let recorded = observed.recorded.clone().expect("the loop reported one");
    let (verdict, detail) = h7(&observed);
    assert_eq!(verdict, Verdict::Gap, "{detail}");
    assert!(
        detail.starts_with(&format!(
            "the session ran in {recorded}, and metaharness could not impose it"
        )),
        "{detail}"
    );
    assert!(!detail.contains("which metaharness created"), "{detail}");
    let rows = gating_not_ok(&observed);
    assert!(
        rows.contains(&(HermeticRow::H7, Verdict::Gap))
            && rows.contains(&(HermeticRow::H11, Verdict::Gap)),
        "{rows:?}"
    );
    assert_eq!(exit(&observed), 1);
}
